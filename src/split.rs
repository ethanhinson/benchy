pub fn split_literal(line: &str, delim: &str, greedy: bool) -> Vec<String> {
    if delim.is_empty() {
        return line.chars().map(|c| c.to_string()).collect();
    }
    if !greedy {
        return split_non_greedy(line, delim);
    }
    if delim.len() == 1 {
        let d = delim.as_bytes()[0];
        return split_greedy_single(line.as_bytes(), d)
            .into_iter()
            .map(|s| String::from_utf8_lossy(&s).into_owned())
            .collect();
    }
    split_non_greedy(line, delim)
}

fn split_non_greedy(line: &str, delim: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut rest = line;
    while let Some(pos) = rest.find(delim) {
        fields.push(rest[..pos].to_string());
        rest = &rest[pos + delim.len()..];
    }
    fields.push(rest.to_string());
    fields
}

fn split_greedy_single(bytes: &[u8], d: u8) -> Vec<Vec<u8>> {
    let mut fields = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == d {
            fields.push(bytes[start..i].to_vec());
            i += 1;
            while i < bytes.len() && bytes[i] == d {
                i += 1;
            }
            start = i;
        } else {
            i += 1;
        }
    }
    fields.push(bytes[start..].to_vec());
    fields
}

pub fn compress_delimiters(line: &str, delim: &str) -> String {
    if delim.len() != 1 {
        return compress_multichar(line, delim);
    }
    let d = delim.as_bytes()[0];
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == d {
            out.push(d);
            i += 1;
            while i < bytes.len() && bytes[i] == d {
                i += 1;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn compress_multichar(line: &str, delim: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while !rest.is_empty() {
        if let Some(pos) = rest.find(delim) {
            out.push_str(&rest[..pos]);
            out.push_str(delim);
            rest = &rest[pos + delim.len()..];
            while rest.starts_with(delim) {
                rest = &rest[delim.len()..];
            }
        } else {
            out.push_str(rest);
            break;
        }
    }
    out
}

pub fn trim_delimiter(line: &str, delim: &str, mode: char) -> String {
    match mode {
        'l' | 'L' => trim_left(line, delim),
        'r' | 'R' => trim_right(line, delim),
        'b' | 'B' => trim_right(&trim_left(line, delim), delim),
        _ => line.to_string(),
    }
}

fn trim_left(mut line: &str, delim: &str) -> String {
    while line.starts_with(delim) {
        line = &line[delim.len()..];
    }
    line.to_string()
}

fn trim_right(line: &str, delim: &str) -> String {
    let mut s = line.to_string();
    while s.ends_with(delim) {
        s.truncate(s.len() - delim.len());
    }
    s
}
