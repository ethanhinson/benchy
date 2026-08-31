use crate::bounds::{
    bound_implies_join, complement_bounds, expand_bound, parse_fields_spec, FieldsSpec, UserBound,
};
use crate::cli::Args;
use crate::format::render_format;
use crate::split::{compress_delimiters, split_literal, trim_delimiter};
use regex::Regex;

pub struct Processor {
    pub args: Args,
    regex: Option<Regex>,
    bounds: FieldsSpec,
    complement: bool,
}

impl Processor {
    pub fn new(args: Args) -> Result<Self, String> {
        let spec_str = args.fields_spec_str();
        let bounds = parse_fields_spec(&spec_str)?;
        let regex = if let Some(r) = &args.regex {
            Some(Regex::new(r).map_err(|e| e.to_string())?)
        } else {
            None
        };
        Ok(Self {
            complement: args.complement,
            bounds,
            regex,
            args,
        })
    }

    pub fn process_input(&self, input: &str) -> Result<Vec<String>, String> {
        if self.args.is_lines() {
            return self.process_lines(input);
        }
        if self.args.is_bytes() {
            return self.process_bytes(input);
        }
        if self.args.is_characters() {
            return self.process_characters(input);
        }
        let mut outputs = Vec::new();
        for line in input.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.is_empty() && !input.ends_with('\n') && input.contains('\n') {
                // keep empty lines in middle
            }
            outputs.push(self.process_line(line)?);
        }
        if !input.contains('\n') {
            return Ok(vec![self.process_line(input)?]);
        }
        Ok(outputs)
    }

    pub fn process_line_stream(&self, line: &str) -> Result<String, String> {
        self.process_line(line)
    }

    fn process_lines(&self, input: &str) -> Result<Vec<String>, String> {
        let mut lines: Vec<&str> = if self.args.zero_terminated {
            input.split('\0').collect()
        } else {
            input.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect()
        };
        if lines.last() == Some(&"") && input.ends_with('\n') {
            lines.pop();
        }
        let n = lines.len();
        let bounds = self.resolve_bounds(n)?;
        let join = self.args.effective_join();
        let jd = self.args.join_delimiter();
        let mut pieces = Vec::new();
        for b in &bounds {
            let lb = normalize_lines_bound(b);
            validate_lines_bound(&lb)?;
            pieces.push(self.select_from_slice(&lines, &lb, join, &jd)?);
        }
        if join && pieces.len() > 1 && !self.args.no_join {
            Ok(vec![pieces.join(&jd)])
        } else if self.args.no_join {
            Ok(vec![pieces.concat()])
        } else {
            Ok(pieces)
        }
    }

    fn process_bytes(&self, input: &str) -> Result<Vec<String>, String> {
        let bytes = input.as_bytes();
        let n = bytes.len();
        let bounds = self.resolve_bounds(n)?;
        let join = self.args.effective_join() || bounds.iter().any(bound_implies_join);
        let jd = self.args.join_delimiter();
        let mut out = String::new();
        for (i, b) in bounds.iter().enumerate() {
            let part = self.select_bytes(bytes, b)?;
            if join || bound_implies_join(b) {
                if i > 0 {
                    out.push_str(&jd);
                }
                out.push_str(&part);
            } else {
                out.push_str(&part);
            }
        }
        Ok(vec![out])
    }

    fn process_characters(&self, input: &str) -> Result<Vec<String>, String> {
        let chars: Vec<char> = input.chars().collect();
        let n = chars.len();
        let bounds = self.resolve_bounds(n)?;
        let join = self.args.effective_join() || bounds.iter().any(bound_implies_join);
        let jd = self.args.join_delimiter();
        let mut out = String::new();
        for (i, b) in bounds.iter().enumerate() {
            let part = self.select_chars(&chars, b)?;
            if join || bound_implies_join(b) {
                if i > 0 {
                    out.push_str(&jd);
                }
                out.push_str(&part);
            } else {
                out.push_str(&part);
            }
        }
        Ok(vec![out])
    }

    fn process_line(&self, line: &str) -> Result<String, String> {
        if self.args.only_delimited && self.args.regex.is_none() {
            if !line.contains(self.args.delimiter.as_str()) {
                return Ok(String::new());
            }
        }

        let mut work = line.to_string();
        if let Some(t) = self.args.trim {
            work = trim_delimiter(&work, &self.args.delimiter, t);
        }
        if self.args.compress_delimiter {
            work = compress_delimiters(&work, &self.args.delimiter);
        }

        let fields = if let Some(re) = &self.regex {
            re.split(&work).map(|s| s.to_string()).collect()
        } else {
            split_literal(&work, &self.args.delimiter, self.args.greedy_delimiter)
        };

        if self.args.only_delimited && fields.len() <= 1 && line != fields.first().map(String::as_str).unwrap_or("") {
            // line had delimiter in regex mode handled above
        }

        if let FieldsSpec::Format(tmpl) = &self.bounds {
            return render_format(tmpl, &fields, &self.args.delimiter);
        }

        let bounds = self.resolve_bounds(fields.len())?;
        if self.args.json {
            let selected = self.collect_fields(&fields, &bounds)?;
            return Ok(json_array(&selected));
        }

        let join_sep = self.args.join_delimiter();
        let force_join = self.args.effective_join() || self.args.replace_delimiter.is_some();
        let mut out = String::new();
        for (i, b) in bounds.iter().enumerate() {
            let (text, internal_join) = self.select_fields(&fields, b)?;
            let use_join = force_join || internal_join || bound_implies_join(b);
            if use_join && i > 0 {
                out.push_str(&join_sep);
            }
            out.push_str(&text);
        }
        Ok(out)
    }

    fn resolve_bounds(&self, n: usize) -> Result<Vec<UserBound>, String> {
        let FieldsSpec::Bounds(list) = &self.bounds else {
            return Err("internal".into());
        };
        if self.complement && n > 0 {
            return Ok(complement_bounds(list, n));
        }
        Ok(list.clone())
    }

    fn collect_fields(&self, fields: &[String], bounds: &[UserBound]) -> Result<Vec<String>, String> {
        let mut out = Vec::new();
        for b in bounds {
            match &b.item {
                crate::bounds::BoundItem::Single(idx) => {
                    out.push(self.field_at(fields, *idx, &b.fallback)?);
                }
                crate::bounds::BoundItem::Range { .. } => {
                    for i in expand_bound(b, fields.len())? {
                        out.push(fields[i - 1].clone());
                    }
                }
            }
        }
        Ok(out)
    }

    fn field_at(&self, fields: &[String], idx: i64, fallback: &Option<String>) -> Result<String, String> {
        if let Some(i) = crate::bounds::resolve_index(idx, fields.len()) {
            Ok(fields[i - 1].clone())
        } else if let Some(fb) = fallback {
            Ok(fb.clone())
        } else if let Some(fb) = &self.args.fallback_oob {
            Ok(fb.clone())
        } else {
            Err(format!("Out of bounds: {idx}"))
        }
    }

    fn select_fields(&self, fields: &[String], b: &UserBound) -> Result<(String, bool), String> {
        match &b.item {
            crate::bounds::BoundItem::Single(idx) => {
                Ok((self.field_at(fields, *idx, &b.fallback)?, false))
            }
            crate::bounds::BoundItem::Range { .. } => {
                let indices = expand_bound(b, fields.len())?;
                let jd = self.args.join_delimiter();
                let text = indices
                    .iter()
                    .map(|&i| fields[i - 1].as_str())
                    .collect::<Vec<_>>()
                    .join(&jd);
                Ok((text, true))
            }
        }
    }

    fn select_from_slice(&self, items: &[&str], b: &UserBound, join: bool, jd: &str) -> Result<String, String> {
        match &b.item {
            crate::bounds::BoundItem::Single(idx) => {
                Ok(self.item_at(items, *idx, &b.fallback)?)
            }
            crate::bounds::BoundItem::Range { .. } => {
                let indices = expand_bound(b, items.len())?;
                if join {
                    Ok(indices.iter().map(|&i| items[i - 1]).collect::<Vec<_>>().join(jd))
                } else {
                    Ok(indices.iter().map(|&i| items[i - 1]).collect::<String>())
                }
            }
        }
    }

    fn item_at(&self, items: &[&str], idx: i64, fallback: &Option<String>) -> Result<String, String> {
        if let Some(i) = crate::bounds::resolve_index(idx, items.len()) {
            Ok(items[i - 1].to_string())
        } else if let Some(fb) = fallback {
            Ok(fb.clone())
        } else if let Some(fb) = &self.args.fallback_oob {
            Ok(fb.clone())
        } else {
            Err(format!("Out of bounds: {idx}"))
        }
    }

    fn select_bytes(&self, bytes: &[u8], b: &UserBound) -> Result<String, String> {
        match &b.item {
            crate::bounds::BoundItem::Single(idx) => {
                let i = crate::bounds::resolve_index(*idx, bytes.len())
                    .ok_or_else(|| format!("Out of bounds: {idx}"))?;
                Ok(String::from_utf8_lossy(&bytes[i - 1..i]).into_owned())
            }
            crate::bounds::BoundItem::Range { .. } => {
                let indices = expand_bound(b, bytes.len())?;
                let slice: Vec<u8> = indices.iter().flat_map(|&i| [bytes[i - 1]]).collect();
                Ok(String::from_utf8_lossy(&slice).into_owned())
            }
        }
    }

    fn select_chars(&self, chars: &[char], b: &UserBound) -> Result<String, String> {
        match &b.item {
            crate::bounds::BoundItem::Single(idx) => {
                let i = crate::bounds::resolve_index(*idx, chars.len())
                    .ok_or_else(|| format!("Out of bounds: {idx}"))?;
                Ok(chars[i - 1].to_string())
            }
            crate::bounds::BoundItem::Range { .. } => {
                let indices = expand_bound(b, chars.len())?;
                Ok(indices.iter().map(|&i| chars[i - 1]).collect())
            }
        }
    }
}

fn normalize_lines_bound(b: &UserBound) -> UserBound {
    if let crate::bounds::BoundItem::Single(-1) = b.item {
        return UserBound {
            item: crate::bounds::BoundItem::Range {
                start: 1,
                end: None,
            },
            fallback: b.fallback.clone(),
        };
    }
    b.clone()
}

fn validate_lines_bound(b: &UserBound) -> Result<(), String> {
    match &b.item {
        crate::bounds::BoundItem::Single(idx) if *idx < 0 && *idx != -1 => {
            Err(format!("Out of bounds: {idx}"))
        }
        crate::bounds::BoundItem::Range { start, end: Some(end) } if *end < 0 => {
            if *start == 1 && *end == -1 {
                Ok(())
            } else if *start == 1 {
                Err(format!("Out of bounds: {end}"))
            } else {
                Err(format!("Out of bounds: {start}"))
            }
        }
        crate::bounds::BoundItem::Range { start, .. } if *start < 0 => {
            Err(format!("Out of bounds: {start}"))
        }
        _ => Ok(()),
    }
}

fn json_array(items: &[String]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|s| {
            let mut o = String::from("\"");
            for ch in s.chars() {
                match ch {
                    '\\' => o.push_str("\\\\"),
                    '"' => o.push_str("\\\""),
                    '\n' => o.push_str("\\n"),
                    '\r' => o.push_str("\\r"),
                    '\t' => o.push_str("\\t"),
                    c if c.is_control() => o.push_str(&format!("\\u{:04x}", c as u32)),
                    c => o.push(c),
                }
            }
            o.push('"');
            o
        })
        .collect();
    format!("[{}]", parts.join(","))
}
