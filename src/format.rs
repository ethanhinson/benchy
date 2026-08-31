use std::io;

pub fn format_template<F>(template: &str, mut resolver: F) -> io::Result<String>
where
    F: FnMut(i64, Option<i64>, Option<i64>) -> io::Result<String>,
{
    let mut out = String::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
            out.push('{');
            i += 2;
            continue;
        }
        if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
            out.push('}');
            i += 2;
            continue;
        }
        if chars[i] == '{' {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && chars[j] != '}' {
                j += 1;
            }
            if j >= chars.len() {
                out.push('{');
                i += 1;
                continue;
            }
            let inner: String = chars[start..j].iter().collect();
            let value = resolve_placeholder(&inner, &mut resolver)?;
            out.push_str(&unescape_format(&value));
            i = j + 1;
            continue;
        }
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'n' => {
                    out.push('\n');
                    i += 2;
                    continue;
                }
                c => {
                    out.push(c);
                    i += 2;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    Ok(out)
}

fn resolve_placeholder<F>(inner: &str, resolver: &mut F) -> io::Result<String>
where
    F: FnMut(i64, Option<i64>, Option<i64>) -> io::Result<String>,
{
    if let Some((idx_str, rest)) = inner.split_once(':') {
        let start: i64 = idx_str
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, format!("Invalid index `{idx_str}`")))?;
        if rest.is_empty() {
            return resolver(start, Some(start), Some(0));
        }
        let end: i64 = rest
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, format!("Invalid index `{rest}`")))?;
        return resolver(start, Some(start), Some(end));
    }
    let idx: i64 = inner
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, format!("Invalid index `{inner}`")))?;
    resolver(idx, None, None)
}

fn unescape_format(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
}
