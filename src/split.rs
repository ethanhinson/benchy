use crate::cut::{Options, TrimMode};
use regex::Regex;

pub fn split(text: &str, opts: &Options) -> Vec<String> {
    let working = if opts.compress && opts.regex.is_none() {
        compress_text(text, &opts.delimiter)
    } else {
        text.to_string()
    };

    let mut parts = if let Some(re) = &opts.regex {
        split_regex(&working, re)
    } else {
        working
            .split(&opts.delimiter)
            .map(|s| s.to_string())
            .collect()
    };

    if opts.greedy && opts.regex.is_none() && !opts.compress {
        parts = parts.into_iter().filter(|p| !p.is_empty()).collect();
    }

    if let Some(trim) = opts.trim {
        apply_trim_in_place(&mut parts, trim);
    }

    parts
}

fn compress_text(text: &str, delimiter: &str) -> String {
    if delimiter.is_empty() {
        return text.to_string();
    }
    let dbytes = delimiter.as_bytes();
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(dbytes) {
            out.push_str(delimiter);
            i += dbytes.len();
            while i < bytes.len() && bytes[i..].starts_with(dbytes) {
                i += dbytes.len();
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

fn split_regex(text: &str, re: &Regex) -> Vec<String> {
    re.split(text).map(|s| s.to_string()).collect()
}

fn apply_trim_in_place(parts: &mut Vec<String>, mode: TrimMode) {
    match mode {
        TrimMode::Left | TrimMode::Both => {
            while parts.first().map(|p| p.is_empty()).unwrap_or(false) {
                parts.remove(0);
            }
        }
        _ => {}
    }
    match mode {
        TrimMode::Right | TrimMode::Both => {
            while parts.last().map(|p| p.is_empty()).unwrap_or(false) {
                parts.pop();
            }
        }
        _ => {}
    }
}
