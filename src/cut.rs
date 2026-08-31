use crate::bounds::{normalize_range, resolve_index, BoundItem, Bounds};
use regex::Regex;
use std::io::{self, Read, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutMode {
    Fields,
    Bytes,
    Characters,
    Lines,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimMode {
    Left,
    Right,
    Both,
}

pub struct Options {
    pub delimiter: String,
    pub regex: Option<Regex>,
    pub greedy: bool,
    pub compress: bool,
    pub trim: Option<TrimMode>,
    pub complement: bool,
    pub join: bool,
    pub replace_delimiter: Option<String>,
    pub only_delimited: bool,
    pub zero_terminated: bool,
    pub json: bool,
    pub fallback_oob: Option<String>,
    pub bounds: Bounds,
    pub bounds_raw: String,
    pub mode: CutMode,
    pub lines_no_join: bool,
}

pub fn run<R: Read + ?Sized, W: Write>(input: &mut R, output: &mut W, opts: &Options) -> io::Result<()> {
    let mut data = Vec::new();
    input.read_to_end(&mut data)?;

    match opts.mode {
        CutMode::Lines => process_lines(&data, output, opts),
        CutMode::Bytes => process_bytes(&data, output, opts),
        CutMode::Characters => process_characters(&data, output, opts),
        CutMode::Fields => process_fields(&data, output, opts),
    }
}

fn line_split(data: &[u8], zero: bool) -> Vec<Vec<u8>> {
    let sep = if zero { b'\0' } else { b'\n' };
    if data.is_empty() {
        return vec![];
    }
    let mut lines: Vec<Vec<u8>> = data.split(|&b| b == sep).map(|l| l.to_vec()).collect();
    if !zero && data.ends_with(b"\n") && lines.last().map(|l| l.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    lines
}

fn process_lines<W: Write>(data: &[u8], output: &mut W, opts: &Options) -> io::Result<()> {
    let lines = line_split(data, opts.zero_terminated);
    let len = lines.len() as i64;

    if opts.bounds.is_format {
        let s = crate::format::format_template(&opts.bounds_raw, |idx, start, end| {
            resolve_line_slice(&lines, idx, start, end, len, opts)
        })?;
        if !s.is_empty() {
            output.write_all(s.as_bytes())?;
        }
        output.write_all(b"\n")?;
        return Ok(());
    }

    let indices = select_line_indices(&opts.bounds, len)?;

    if opts.complement {
        let remove: std::collections::HashSet<usize> = indices.into_iter().collect();
        let kept: Vec<usize> = (0..lines.len()).filter(|i| !remove.contains(i)).collect();
        return write_line_selection(output, &lines, &kept, opts);
    }

    write_line_selection(output, &lines, &indices, opts)
}

fn resolve_line_slice(
    lines: &[Vec<u8>],
    idx: i64,
    start: Option<i64>,
    end: Option<i64>,
    len: i64,
    opts: &Options,
) -> io::Result<String> {
    if let (Some(s), Some(e)) = (start, end) {
        let (rs, re) = normalize_range(s, e, len);
        let mut out = String::new();
        for i in rs..=re {
            if let Some(r) = resolve_index(i, len) {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&String::from_utf8_lossy(&lines[(r - 1) as usize]));
            } else if let Some(fb) = &opts.fallback_oob {
                out.push_str(fb);
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Out of bounds: {i}")));
            }
        }
        return Ok(out);
    }
    if let Some(r) = resolve_index(idx, len) {
        Ok(String::from_utf8_lossy(&lines[(r - 1) as usize]).into_owned())
    } else if let Some(fb) = &opts.fallback_oob {
        Ok(fb.clone())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Out of bounds: {idx}"),
        ))
    }
}

fn select_line_indices(bounds: &Bounds, len: i64) -> io::Result<Vec<usize>> {
    if len == 0 {
        return Ok(vec![]);
    }

    if bounds.items.len() == 1 {
        match &bounds.items[0] {
            BoundItem::Single(-1, _) => return Ok((0..len as usize).collect()),
            BoundItem::Range(a, b, _) if (*a == 0 || *a == 1) && (*b == 0 || *b == -1) => {
                return Ok((0..len as usize).collect());
            }
            _ => {}
        }
    }

    let mut indices = Vec::new();
    for item in &bounds.items {
        match item {
            BoundItem::Single(idx, _) => {
                let r = resolve_index(*idx, len).ok_or_else(|| oob(*idx))?;
                indices.push((r - 1) as usize);
            }
            BoundItem::Range(start, end, _) => {
                let (s, e) = normalize_range(*start, *end, len);
                if s > 0 && resolve_index(s, len).is_none() {
                    return Err(oob(s));
                }
                let rs = if s > 0 {
                    resolve_index(s, len).ok_or_else(|| oob(s))?
                } else {
                    1
                };
                let re = resolve_index(e, len).ok_or_else(|| oob(e))?;
                if rs > re {
                    return Err(oob(*start));
                }
                for i in rs..=re {
                    indices.push((i - 1) as usize);
                }
            }
        }
    }
    Ok(indices)
}

fn write_line_selection<W: Write>(
    output: &mut W,
    lines: &[Vec<u8>],
    indices: &[usize],
    opts: &Options,
) -> io::Result<()> {
    if indices.is_empty() {
        return Ok(());
    }

    if opts.lines_no_join {
        let mut merged = Vec::new();
        for &idx in indices {
            merged.extend_from_slice(&lines[idx]);
        }
        output.write_all(&merged)?;
        output.write_all(b"\n")?;
        return Ok(());
    }

    for (i, &idx) in indices.iter().enumerate() {
        output.write_all(&lines[idx])?;
        if i + 1 < indices.len() {
            if let Some(sep) = &opts.replace_delimiter {
                output.write_all(sep.as_bytes())?;
            }
        }
        output.write_all(b"\n")?;
    }
    Ok(())
}

fn process_fields<W: Write>(data: &[u8], output: &mut W, opts: &Options) -> io::Result<()> {
    let records = line_split(data, opts.zero_terminated);
    for record in records {
        process_field_record(&record, output, opts)?;
    }
    Ok(())
}

fn process_field_record<W: Write>(data: &[u8], output: &mut W, opts: &Options) -> io::Result<()> {
    if opts.only_delimited && !has_delimiter(data, opts) {
        return Ok(());
    }

    let text = String::from_utf8_lossy(data);
    let parts = crate::split::split(&text, opts);

    if opts.json {
        write_json(&parts, output)?;
        output.write_all(b"\n")?;
        return Ok(());
    }

    if opts.bounds.is_format {
        let template = opts.bounds.format_template.as_deref().unwrap_or(&opts.bounds_raw);
        let s = crate::format::format_template(template, |idx, start, end| {
            resolve_field_slice(&parts, idx, start, end, opts)
        })?;
        output.write_all(s.as_bytes())?;
        output.write_all(b"\n")?;
        return Ok(());
    }

    let selected = if opts.complement {
        complement_fields(&parts, opts)?
    } else {
        select_fields(&parts, opts)?
    };

    write_joined(output, &selected, opts)?;
    Ok(())
}

fn process_bytes<W: Write>(data: &[u8], output: &mut W, opts: &Options) -> io::Result<()> {
    let records = line_split(data, opts.zero_terminated);
    for record in records {
        let bytes = if opts.only_delimited && !has_delimiter(&record, opts) {
            continue;
        } else {
            record
        };
        let selected = select_bytes(&bytes, opts)?;
        write_joined(output, &selected, opts)?;
    }
    Ok(())
}

fn process_characters<W: Write>(data: &[u8], output: &mut W, opts: &Options) -> io::Result<()> {
    let records = line_split(data, opts.zero_terminated);
    for record in records {
        if opts.only_delimited && !has_delimiter(&record, opts) {
            continue;
        }
        let text = String::from_utf8_lossy(&record);
        let selected = select_characters(&text, opts)?;
        write_joined(output, &selected, opts)?;
    }
    Ok(())
}

fn has_delimiter(data: &[u8], opts: &Options) -> bool {
    if let Some(re) = &opts.regex {
        re.is_match(&String::from_utf8_lossy(data))
    } else {
        data.windows(opts.delimiter.len())
            .any(|w| w == opts.delimiter.as_bytes())
    }
}

fn select_fields(parts: &[String], opts: &Options) -> io::Result<Vec<String>> {
    let len = parts.len() as i64;
    let mut out = Vec::new();
    for item in &opts.bounds.items {
        collect_items(parts, len, item, opts, &mut out)?;
    }
    Ok(out)
}

fn collect_items(
    parts: &[String],
    len: i64,
    item: &BoundItem,
    opts: &Options,
    out: &mut Vec<String>,
) -> io::Result<()> {
    match item {
        BoundItem::Single(idx, fb) => {
            out.push(resolve_field_with_fb(parts, *idx, None, None, opts, fb)?);
        }
        BoundItem::Range(start, end, fb) => {
            let (s, e) = normalize_range(*start, *end, len);
            let rs = resolve_index(s, len);
            let re = resolve_index(e, len);
            match (rs, re) {
                (Some(rs), Some(re)) => {
                    if rs == re {
                        out.push(resolve_field_with_fb(parts, rs, None, None, opts, fb)?);
                    } else {
                        let join = opts.join || opts.replace_delimiter.is_some();
                        if join {
                            for i in rs..=re {
                                out.push(resolve_field_with_fb(parts, i, None, None, opts, fb)?);
                            }
                        } else {
                            let sep = &opts.delimiter;
                            let mut chunk = String::new();
                            for i in rs..=re {
                                if !chunk.is_empty() {
                                    chunk.push_str(sep);
                                }
                                chunk.push_str(&parts[(i - 1) as usize]);
                            }
                            out.push(chunk);
                        }
                    }
                }
                _ => out.push(fallback_value(fb, opts)?),
            }
        }
    }
    Ok(())
}

fn fallback_value(fb: &Option<String>, opts: &Options) -> io::Result<String> {
    fb.clone()
        .or_else(|| opts.fallback_oob.clone())
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Out of bounds".to_string()))
}

fn fallback_value_idx(fb: &Option<String>, opts: &Options, idx: i64) -> io::Result<String> {
    fb.clone()
        .or_else(|| opts.fallback_oob.clone())
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, format!("Out of bounds: {idx}")))
}

fn resolve_field_slice(
    parts: &[String],
    idx: i64,
    start: Option<i64>,
    end: Option<i64>,
    opts: &Options,
) -> io::Result<String> {
    let len = parts.len() as i64;
    if let (Some(s), Some(e)) = (start, end) {
        let (rs, re) = normalize_range(s, e, len);
        let mut out = String::new();
        for i in rs..=re {
            match resolve_index(i, len) {
                Some(r) => {
                    if !out.is_empty() {
                        out.push_str(&opts.delimiter);
                    }
                    out.push_str(&parts[(r - 1) as usize]);
                }
                None => {
                    if let Some(fb) = &opts.fallback_oob {
                        out.push_str(fb);
                    } else {
                        return Err(oob(i));
                    }
                }
            }
        }
        return Ok(out);
    }
    resolve_field_with_fb(parts, idx, None, None, opts, &None)
}

fn resolve_field_with_fb(
    parts: &[String],
    idx: i64,
    start: Option<i64>,
    end: Option<i64>,
    opts: &Options,
    fb: &Option<String>,
) -> io::Result<String> {
    let len = parts.len() as i64;
    if let (Some(s), Some(e)) = (start, end) {
        let (rs, re) = normalize_range(s, e, len);
        let mut out = String::new();
        for i in rs..=re {
            match resolve_index(i, len) {
                Some(r) => {
                    if !out.is_empty() {
                        out.push_str(&opts.delimiter);
                    }
                    out.push_str(&parts[(r - 1) as usize]);
                }
                None => out.push_str(&fallback_value_idx(fb, opts, i)?),
            }
        }
        return Ok(out);
    }
    match resolve_index(idx, len) {
        Some(r) => Ok(parts[(r - 1) as usize].clone()),
        None => fallback_value_idx(fb, opts, idx),
    }
}

fn complement_fields(parts: &[String], opts: &Options) -> io::Result<Vec<String>> {
    let len = parts.len() as i64;
    let mut remove = std::collections::HashSet::new();
    for item in &opts.bounds.items {
        if let BoundItem::Single(idx, _) = item {
            if let Some(r) = resolve_index(*idx, len) {
                remove.insert(r);
            }
        }
    }

    if opts.join || opts.replace_delimiter.is_some() {
        return Ok(parts
            .iter()
            .enumerate()
            .filter(|(i, _)| !remove.contains(&((i + 1) as i64)))
            .map(|(_, p)| p.clone())
            .collect());
    }

    let sep = &opts.delimiter;
    let kept: Vec<usize> = (0..parts.len())
        .filter(|i| !remove.contains(&((*i + 1) as i64)))
        .collect();

    let mut out = String::new();
    for (pos, &ki) in kept.iter().enumerate() {
        out.push_str(&parts[ki]);
        if pos + 1 < kept.len() {
            let next = kept[pos + 1];
            if next == ki + 1 {
                out.push_str(sep);
            }
        }
    }
    Ok(vec![out])
}

fn select_bytes(data: &[u8], opts: &Options) -> io::Result<Vec<String>> {
    let len = data.len() as i64;
    let mut out = Vec::new();
    for item in &opts.bounds.items {
        match item {
            BoundItem::Single(idx, fb) => {
                out.push(slice_bytes(data, *idx, *idx, len, fb, opts)?);
            }
            BoundItem::Range(start, end, fb) => {
                let (s, e) = normalize_range(*start, *end, len);
                out.push(slice_bytes(data, s, e, len, fb, opts)?);
            }
        }
    }
    Ok(out)
}

fn slice_bytes(
    data: &[u8],
    start: i64,
    end: i64,
    len: i64,
    fb: &Option<String>,
    opts: &Options,
) -> io::Result<String> {
    let rs = resolve_index(start, len);
    let re = resolve_index(end, len);
    match (rs, re) {
        (Some(rs), Some(re)) => {
            let from = (rs - 1) as usize;
            let to = re as usize;
            Ok(String::from_utf8_lossy(&data[from..to]).into_owned())
        }
        _ => fallback_value(fb, opts),
    }
}

fn select_characters(text: &str, opts: &Options) -> io::Result<Vec<String>> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len() as i64;
    let mut out = Vec::new();
    for item in &opts.bounds.items {
        match item {
            BoundItem::Single(idx, fb) => {
                out.push(slice_chars(&chars, *idx, *idx, len, fb, opts)?);
            }
            BoundItem::Range(start, end, fb) => {
                let (s, e) = normalize_range(*start, *end, len);
                out.push(slice_chars(&chars, s, e, len, fb, opts)?);
            }
        }
    }
    Ok(out)
}

fn slice_chars(
    chars: &[char],
    start: i64,
    end: i64,
    len: i64,
    fb: &Option<String>,
    opts: &Options,
) -> io::Result<String> {
    let rs = resolve_index(start, len);
    let re = resolve_index(end, len);
    match (rs, re) {
        (Some(rs), Some(re)) => {
            let from = (rs - 1) as usize;
            let to = re as usize;
            Ok(chars[from..to].iter().collect())
        }
        _ => fallback_value(fb, opts),
    }
}

fn write_joined<W: Write>(output: &mut W, parts: &[String], opts: &Options) -> io::Result<()> {
    let join = opts.join || opts.replace_delimiter.is_some();
    let sep = opts
        .replace_delimiter
        .as_deref()
        .unwrap_or(&opts.delimiter);

    let mut line = String::new();
    for (i, part) in parts.iter().enumerate() {
        line.push_str(part);
        if join && i + 1 < parts.len() {
            line.push_str(sep);
        }
    }
    output.write_all(line.as_bytes())?;
    output.write_all(b"\n")?;
    Ok(())
}

fn write_json<W: Write>(parts: &[String], output: &mut W) -> io::Result<()> {
    write!(output, "[")?;
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            write!(output, ",")?;
        }
        write!(output, "{}", json_string(part))?;
    }
    write!(output, "]")
}

pub fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
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

fn oob(idx: i64) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("Out of bounds: {idx}"))
}
