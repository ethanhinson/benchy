use crate::bounds::{
    complement_bound, resolve_index, resolve_range, Bound, FormatPart, FormatSpec,
};
use crate::TrimMode;
use regex::Regex;

#[derive(Clone)]
pub struct CutConfig {
    pub delimiter: String,
    pub regex: Option<String>,
    pub join: bool,
    pub line_mode: bool,
    pub complement: bool,
    pub greedy: bool,
    pub compress: bool,
    pub only_delimited: bool,
    pub replace_delimiter: Option<String>,
    pub trim: Option<TrimMode>,
    pub json: bool,
    pub fallback_oob: Option<String>,
    pub bounds: Option<Vec<Bound>>,
    pub format_spec: Option<FormatSpec>,
}

pub fn process_fields(
    data: &[u8],
    config: &CutConfig,
    zero_terminated: bool,
) -> Result<String, String> {
    let line_sep = if zero_terminated { '\0' } else { '\n' };
    let text = String::from_utf8_lossy(data);
    let mut output = String::new();

    for line in split_lines(&text, line_sep) {
        let line = strip_line_terminator(line);
        if config.only_delimited && !line_contains_delimiter(&line, config) {
            continue;
        }

        let fields = split_into_fields(&line, config)?;
        if config.json {
            let selected = select_json_fields(&fields, config)?;
            output.push_str(&to_json(&selected)?);
        } else {
            let rendered = render_selection(&fields, config)?;
            output.push_str(&rendered);
            output.push('\n');
        }
    }

    Ok(output)
}

pub fn process_characters(
    data: &[u8],
    config: &CutConfig,
    zero_terminated: bool,
) -> Result<String, String> {
    let line_sep = if zero_terminated { '\0' } else { '\n' };
    let text = String::from_utf8_lossy(data);
    let mut output = String::new();

    for line in split_lines(&text, line_sep) {
        let line = strip_line_terminator(line);
        let chars: Vec<String> = line.chars().map(|c| c.to_string()).collect();
        if config.json {
            let selected = select_json_fields(&chars, config)?;
            output.push_str(&to_json(&selected)?);
        } else {
            let rendered = render_selection(&chars, config)?;
            output.push_str(&rendered);
            output.push('\n');
        }
    }

    Ok(output)
}

pub fn process_bytes(data: &[u8], config: &CutConfig) -> Result<String, String> {
    let byte_fields: Vec<String> = data.iter().map(|&b| (b as char).to_string()).collect();
    if config.json {
        let selected = select_json_fields(&byte_fields, config)?;
        return to_json(&selected);
    }
    select_bytes(data, config).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

fn select_bytes(data: &[u8], config: &CutConfig) -> Result<Vec<u8>, String> {
    let total = data.len() as i64;
    let bounds = config.bounds.as_ref().ok_or_else(|| "Missing bounds".to_string())?;
    let mut out = Vec::new();

    for bound in bounds {
        let use_bounds = if config.complement {
            complement_bound(bound, total)?
        } else {
            vec![bound.clone()]
        };
        for b in use_bounds {
            let (start, end) = resolve_range(&b, total)?;
            let start_idx = (start - 1) as usize;
            let end_idx = end as usize;
            if start_idx >= data.len() {
                if let Some(fb) = pick_fallback(&b, config) {
                    out.extend_from_slice(fb.as_bytes());
                    continue;
                }
                return Err(format!("Error: Out of bounds: {}", b.start));
            }
            let end_idx = end_idx.min(data.len());
            if start_idx < end_idx {
                out.extend_from_slice(&data[start_idx..end_idx]);
            }
        }
    }
    Ok(out)
}

pub fn process_lines(
    data: &[u8],
    config: &CutConfig,
    zero_terminated: bool,
) -> Result<String, String> {
    if let Some(bounds) = &config.bounds {
        for bound in bounds {
            validate_line_bound(bound)?;
        }
    }

    let line_sep = if zero_terminated { '\0' } else { '\n' };
    let text = String::from_utf8_lossy(data);
    let raw_lines: Vec<String> = split_lines(&text, line_sep)
        .into_iter()
        .map(strip_line_terminator)
        .collect();

    if raw_lines.is_empty() {
        return Ok(String::new());
    }

    if config.json {
        return to_json(&lines_for_json(&raw_lines, config)?);
    }

    let rendered = render_selection(&raw_lines, config)?;
    Ok(format!("{rendered}\n"))
}

fn select_json_fields(fields: &[String], config: &CutConfig) -> Result<Vec<String>, String> {
    let total = fields.len() as i64;
    let bounds = config.bounds.as_ref().ok_or_else(|| "Missing bounds".to_string())?;
    let mut selected = Vec::new();
    for bound in bounds {
        let use_bounds = if config.complement {
            complement_bound(bound, total)?
        } else {
            vec![bound.clone()]
        };
        for b in use_bounds {
            let (start, end) = resolve_range(&b, total)?;
            for idx in start..=end {
                selected.push(get_field(fields, idx, &b, config)?);
            }
        }
    }
    Ok(selected)
}

fn lines_for_json(lines: &[String], config: &CutConfig) -> Result<Vec<String>, String> {
    select_json_fields(lines, config)
}

fn validate_line_bound(bound: &Bound) -> Result<(), String> {
    if let Some(end) = bound.end {
        if bound.start < 1 {
            return Err(format!("Error: Out of bounds: {}", bound.start));
        }
        if end < 0 {
            if bound.start != 1 {
                return Err(format!("Error: Out of bounds: {}", bound.start));
            }
            if end != -1 {
                return Err(format!("Error: Out of bounds: {}", end));
            }
        }
    }
    Ok(())
}

fn join_delimiter(config: &CutConfig) -> &str {
    if config.line_mode {
        "\n"
    } else {
        config
            .replace_delimiter
            .as_deref()
            .unwrap_or(&config.delimiter)
    }
}

fn is_range_bound(bound: &Bound) -> bool {
    bound.end.is_some()
}

fn split_lines(text: &str, sep: char) -> Vec<&str> {
    if sep == '\n' {
        text.split_inclusive('\n').collect()
    } else {
        text.split(sep).collect()
    }
}

fn strip_line_terminator(line: &str) -> String {
    line.strip_suffix('\n')
        .or_else(|| line.strip_suffix("\r\n"))
        .unwrap_or(line)
        .to_string()
}

fn line_contains_delimiter(line: &str, config: &CutConfig) -> bool {
    if let Some(re) = &config.regex {
        Regex::new(re).map(|r| r.is_match(line)).unwrap_or(false)
    } else if !config.delimiter.is_empty() {
        line.contains(&config.delimiter)
    } else {
        true
    }
}

fn apply_trim(line: &str, config: &CutConfig) -> String {
    let Some(mode) = config.trim else {
        return line.to_string();
    };
    let delim = &config.delimiter;
    if delim.is_empty() {
        return line.to_string();
    }
    let mut s = line.to_string();
    match mode {
        TrimMode::Left | TrimMode::Both => {
            while s.starts_with(delim) {
                s = s[delim.len()..].to_string();
            }
        }
        _ => {}
    }
    match mode {
        TrimMode::Right | TrimMode::Both => {
            while s.ends_with(delim) {
                s.truncate(s.len().saturating_sub(delim.len()));
            }
        }
        _ => {}
    }
    s
}

fn compress_delimiters(line: &str, delimiter: &str) -> String {
    if delimiter.is_empty() {
        return line.to_string();
    }
    let mut out = String::new();
    let mut i = 0;
    let bytes = line.as_bytes();
    let dlen = delimiter.len();
    while i <= line.len() {
        if line[i..].starts_with(delimiter) {
            if !out.ends_with(delimiter) {
                out.push_str(delimiter);
            }
            i += dlen;
            while i <= line.len() && line[i..].starts_with(delimiter) {
                i += dlen;
            }
        } else if i == line.len() {
            break;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

fn split_into_fields(line: &str, config: &CutConfig) -> Result<Vec<String>, String> {
    let trimmed = apply_trim(line, config);
    let prepared = if config.compress {
        compress_delimiters(&trimmed, &config.delimiter)
    } else {
        trimmed
    };

    if let Some(re_str) = &config.regex {
        let re = Regex::new(re_str).map_err(|e| format!("Invalid regex: {e}"))?;
        Ok(re.split(&prepared)
            .map(|s| s.to_string())
            .collect())
    } else if config.greedy {
        Ok(split_greedy(&prepared, &config.delimiter))
    } else {
        Ok(split_normal(&prepared, &config.delimiter))
    }
}

fn split_normal(line: &str, delimiter: &str) -> Vec<String> {
    if delimiter.is_empty() {
        return vec![line.to_string()];
    }
    line.split(delimiter).map(|s| s.to_string()).collect()
}

fn split_greedy(line: &str, delimiter: &str) -> Vec<String> {
    if delimiter.is_empty() {
        return vec![line.to_string()];
    }
    let mut fields = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    let dlen = delimiter.len();
    while i <= line.len() {
        if line[i..].starts_with(delimiter) {
            fields.push(line[start..i].to_string());
            i += dlen;
            while i <= line.len() && line[i..].starts_with(delimiter) {
                i += dlen;
            }
            start = i;
        } else if i == line.len() {
            fields.push(line[start..].to_string());
            break;
        } else {
            i += 1;
        }
    }
    if fields.is_empty() {
        fields.push(String::new());
    }
    fields
}

fn render_selection(fields: &[String], config: &CutConfig) -> Result<String, String> {
    if let Some(format_spec) = &config.format_spec {
        return render_format(fields, format_spec, config);
    }

    let total = fields.len() as i64;
    if total == 0 && config.complement {
        return Err("Error: the complement is empty".to_string());
    }

    let bounds = config.bounds.as_ref().ok_or_else(|| "Missing bounds".to_string())?;
    let join_str = join_delimiter(config);

    if config.json {
        let mut selected = Vec::new();
        for bound in bounds {
            let use_bounds = if config.complement {
                complement_bound(bound, total)?
            } else {
                vec![bound.clone()]
            };
            for b in use_bounds {
                let (start, end) = resolve_range(&b, total)?;
                for idx in start..=end {
                    selected.push(get_field(fields, idx, &b, config)?);
                }
            }
        }
        return to_json(&selected);
    }

    let mut output = String::new();
    let mut first_chunk = true;
    for bound in bounds {
        let use_bounds = if config.complement {
            complement_bound(bound, total)?
        } else {
            vec![bound.clone()]
        };
        for b in use_bounds {
            let chunk = extract_range(fields, &b, config, join_str)?;
            if !first_chunk && config.join {
                output.push_str(join_str);
            }
            output.push_str(&chunk);
            first_chunk = false;
        }
    }
    Ok(output)
}

fn render_format(
    fields: &[String],
    spec: &FormatSpec,
    config: &CutConfig,
) -> Result<String, String> {
    let total = fields.len() as i64;
    let join_str = join_delimiter(config);
    let mut out = String::new();
    for part in &spec.parts {
        match part {
            FormatPart::Literal(s) => out.push_str(s),
            FormatPart::Field { start, end } => {
                let bound = Bound {
                    start: *start,
                    end: *end,
                    fallback: None,
                };
                out.push_str(&extract_range(fields, &bound, config, join_str)?);
            }
        }
    }
    if total == 0 {
        let _ = total;
    }
    Ok(out)
}

fn extract_range(
    fields: &[String],
    bound: &Bound,
    config: &CutConfig,
    join_str: &str,
) -> Result<String, String> {
    let total = fields.len() as i64;
    let (start, end) = match resolve_range(bound, total) {
        Ok(v) => v,
        Err(e) => {
            if let Some(fb) = pick_fallback(bound, config) {
                return Ok(fb);
            }
            return Err(e);
        }
    };

    let mut parts = Vec::new();
    for idx in start..=end {
        parts.push(get_field(fields, idx, bound, config)?);
    }

    if is_range_bound(bound) {
        Ok(parts.join(join_str))
    } else {
        Ok(parts.into_iter().next().unwrap_or_default())
    }
}

fn get_field(
    fields: &[String],
    idx: i64,
    bound: &Bound,
    config: &CutConfig,
) -> Result<String, String> {
    match resolve_index(idx, fields.len() as i64) {
        Ok(resolved) => Ok(fields[(resolved - 1) as usize].clone()),
        Err(e) => {
            if let Some(fb) = bound.fallback.as_ref() {
                Ok(fb.clone())
            } else if let Some(fb) = config.fallback_oob.as_ref() {
                Ok(fb.clone())
            } else {
                Err(e)
            }
        }
    }
}

fn pick_fallback(bound: &Bound, config: &CutConfig) -> Option<String> {
    bound
        .fallback
        .clone()
        .or_else(|| config.fallback_oob.clone())
}

fn to_json(values: &[String]) -> Result<String, String> {
    let mut out = String::from("[");
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&json_string(v));
    }
    out.push(']');
    out.push('\n');
    Ok(out)
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greedy_split() {
        let fields = split_greedy("foo    bar", " ");
        assert_eq!(fields, vec!["foo", "bar"]);
    }
}
