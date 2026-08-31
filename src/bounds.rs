use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bound {
    pub start: i64,
    pub end: Option<i64>,
    pub fallback: Option<String>,
}

#[derive(Clone, Debug)]
pub enum FormatPart {
    Literal(String),
    Field {
        start: i64,
        end: Option<i64>,
    },
}

#[derive(Clone, Debug)]
pub struct FormatSpec {
    pub parts: Vec<FormatPart>,
}

pub fn parse_bounds(raw: &str) -> Result<Vec<Bound>, String> {
    if raw.is_empty() {
        return Err(
            "UserBoundsList must contain at least one UserBounds".to_string(),
        );
    }

    let mut bounds = Vec::new();
    for segment in split_commas(raw) {
        if segment.is_empty() {
            continue;
        }
        bounds.push(parse_single_bound(&segment)?);
    }

    if bounds.is_empty() {
        return Err(
            "UserBoundsList must contain at least one UserBounds".to_string(),
        );
    }

    Ok(bounds)
}

fn split_commas(raw: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for ch in raw.chars() {
        if ch == ',' {
            parts.push(current.clone());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    parts.push(current);
    parts
}

fn parse_single_bound(segment: &str) -> Result<Bound, String> {
    let (spec, fallback) = if let Some((left, right)) = segment.split_once('=') {
        (left, Some(right.to_string()))
    } else {
        (segment, None)
    };

    if let Some((start_str, end_str)) = spec.split_once(':') {
        let start: i64 = if start_str.is_empty() {
            1
        } else {
            start_str
                .parse()
                .map_err(|_| format!("Invalid bound: {segment}"))?
        };
        let end: i64 = if end_str.is_empty() {
            -1
        } else {
            end_str
                .parse()
                .map_err(|_| format!("Invalid bound: {segment}"))?
        };
        Ok(Bound {
            start,
            end: Some(end),
            fallback,
        })
    } else {
        let start: i64 = spec
            .parse()
            .map_err(|_| format!("Invalid bound: {segment}"))?;
        Ok(Bound {
            start,
            end: None,
            fallback,
        })
    }
}

pub fn parse_format_spec(raw: &str) -> Result<FormatSpec, String> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let mut chars = raw.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            if matches!(chars.peek(), Some('{')) {
                chars.next();
                literal.push('{');
                continue;
            }
            if !literal.is_empty() {
                parts.push(FormatPart::Literal(take_unescaped(&mut literal)));
                literal.clear();
            }
            let mut inner = String::new();
            let mut closed = false;
            for next in chars.by_ref() {
                if next == '}' {
                    closed = true;
                    break;
                }
                inner.push(next);
            }
            if !closed {
                return Err(format!("Unclosed format placeholder in: {raw}"));
            }
            let (start, end) = if let Some((s, e)) = inner.split_once(':') {
                let start: i64 = s
                    .parse()
                    .map_err(|_| format!("Invalid format field: {{{inner}}}"))?;
                let end: i64 = if e.is_empty() {
                    -1
                } else {
                    e.parse()
                        .map_err(|_| format!("Invalid format field: {{{inner}}}"))?
                };
                (start, Some(end))
            } else {
                let start: i64 = inner
                    .parse()
                    .map_err(|_| format!("Invalid format field: {{{inner}}}"))?;
                (start, None)
            };
            parts.push(FormatPart::Field { start, end });
        } else if ch == '}' {
            if matches!(chars.peek(), Some('}')) {
                chars.next();
                literal.push('}');
            } else {
                return Err(format!("Unexpected '}}' in format: {raw}"));
            }
        } else if ch == '\\' {
            if let Some(next) = chars.next() {
                if next == 'n' {
                    literal.push('\n');
                } else {
                    literal.push('\\');
                    literal.push(next);
                }
            } else {
                literal.push('\\');
            }
        } else {
            literal.push(ch);
        }
    }

    if !literal.is_empty() {
        parts.push(FormatPart::Literal(take_unescaped(&mut literal)));
    }

    if parts.is_empty() {
        return Err(
            "UserBoundsList must contain at least one UserBounds".to_string(),
        );
    }

    Ok(FormatSpec { parts })
}

fn take_unescaped(s: &mut String) -> String {
    std::mem::take(s)
}

pub fn complement_bound(bound: &Bound, total: i64) -> Result<Vec<Bound>, String> {
    if total == 0 {
        return Err("Error: the complement is empty".to_string());
    }

    let (start, end) = resolve_range(bound, total)?;
    let mut out = Vec::new();

    if start > 1 {
        out.push(Bound {
            start: 1,
            end: Some(start - 1),
            fallback: bound.fallback.clone(),
        });
    }

    if end < total {
        out.push(Bound {
            start: end + 1,
            end: Some(-1),
            fallback: bound.fallback.clone(),
        });
    }

    if out.is_empty() {
        return Err("Error: the complement is empty".to_string());
    }

    Ok(out)
}

pub fn resolve_index(index: i64, total: i64) -> Result<i64, String> {
    if total == 0 {
        return Err(format!("Error: Out of bounds: {index}"));
    }
    let resolved = if index > 0 {
        index
    } else if index < 0 {
        total + index + 1
    } else {
        return Err(format!("Error: Out of bounds: {index}"));
    };
    if resolved < 1 || resolved > total {
        Err(format!("Error: Out of bounds: {index}"))
    } else {
        Ok(resolved)
    }
}

pub fn resolve_range(bound: &Bound, total: i64) -> Result<(i64, i64), String> {
    match bound.end {
        None => {
            let idx = resolve_index(bound.start, total)?;
            Ok((idx, idx))
        }
        Some(end_raw) => {
            let start = resolve_index(bound.start, total)?;
            let end = resolve_index(end_raw, total)?;
            if start <= end {
                Ok((start, end))
            } else {
                Ok((end, start))
            }
        }
    }
}

impl fmt::Display for Bound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.end {
            None => write!(f, "{}", self.start),
            Some(-1) => write!(f, "{}:", self.start),
            Some(end) => write!(f, "{}:{}", self.start, end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bounds() {
        let bounds = parse_bounds("1,3:5,-1").unwrap();
        assert_eq!(bounds.len(), 3);
    }
}
