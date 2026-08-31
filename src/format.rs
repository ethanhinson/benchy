use crate::bounds::resolve_index;

pub fn render_format(template: &str, fields: &[String], delim: &str) -> Result<String, String> {
    let mut out = String::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '{' {
            if i + 1 < chars.len() && chars[i + 1] == '{' {
                out.push('{');
                i += 2;
                continue;
            }
            let j = find_closing(&chars, i)?;
            let inner: String = chars[i + 1..j].iter().collect();
            out.push_str(&render_placeholder(&inner, fields, delim)?);
            i = j + 1;
        } else if chars[i] == '}' {
            if i + 1 < chars.len() && chars[i + 1] == '}' {
                out.push('}');
                i += 2;
                continue;
            }
            return Err(format!("failed to parse '{template}': unexpected '}}'"));
        } else if chars[i] == '\\' && i + 1 < chars.len() && chars[i + 1] == 'n' {
            out.push('\n');
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    Ok(out)
}

fn find_closing(chars: &[char], start: usize) -> Result<usize, String> {
    for j in (start + 1)..chars.len() {
        if chars[j] == '}' {
            return Ok(j);
        }
    }
    Err("unclosed brace".into())
}

fn render_placeholder(inner: &str, fields: &[String], delim: &str) -> Result<String, String> {
    if let Some((a, b)) = inner.split_once(':') {
        let start: i64 = if a.is_empty() {
            1
        } else {
            parse_num(a)?
        };
        let end: Option<i64> = if b.is_empty() {
            None
        } else {
            Some(parse_num(b)?)
        };
        let s = resolve_index(start, fields.len())
            .ok_or_else(|| format!("Out of bounds: {start}"))?;
        let e = match end {
            Some(e) => resolve_index(e, fields.len()).ok_or_else(|| format!("Out of bounds: {e}"))?,
            None => fields.len(),
        };
        let (lo, hi) = if s <= e { (s, e) } else { (e, s) };
        return Ok(fields[lo - 1..hi]
            .join(delim));
    }
    let idx = parse_num(inner)?;
    if let Some(i) = resolve_index(idx, fields.len()) {
        Ok(fields[i - 1].clone())
    } else {
        Err(format!("Out of bounds: {idx}"))
    }
}

fn parse_num(s: &str) -> Result<i64, String> {
    s.parse::<i64>()
        .map_err(|_| format!("failed to parse '{{{s}}}': Not a number `{s}`"))
}
