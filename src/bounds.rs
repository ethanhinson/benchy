use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundItem {
    Single(i64, Option<String>),
    Range(i64, i64, Option<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bounds {
    pub items: Vec<BoundItem>,
    pub is_format: bool,
    pub format_template: Option<String>,
}

#[derive(Debug)]
pub enum ParseError {
    Zero,
    NotANumber(String),
    Empty,
    InvalidRange,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Zero => write!(f, "Zero is not a valid field"),
            ParseError::NotANumber(s) => write!(f, "Not a number `{s}`"),
            ParseError::Empty => write!(f, "Empty bound"),
            ParseError::InvalidRange => write!(f, "Invalid range"),
        }
    }
}

pub fn parse_bounds(input: &str) -> Result<Bounds, ParseError> {
    if input.is_empty() {
        return Err(ParseError::Empty);
    }

    if input.contains('{') {
        return Ok(Bounds {
            items: vec![],
            is_format: true,
            format_template: Some(input.to_string()),
        });
    }

    let items = input
        .split(',')
        .map(parse_bound_item)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Bounds {
        items,
        is_format: false,
        format_template: None,
    })
}

fn parse_bound_item(part: &str) -> Result<BoundItem, ParseError> {
    let (spec, fallback) = match part.split_once('=') {
        Some((s, fb)) => (s, Some(fb.to_string())),
        None => (part, None),
    };

    if spec.contains(':') {
        let (start, end) = spec
            .split_once(':')
            .ok_or(ParseError::InvalidRange)?;
        let start = parse_index_optional(start)?;
        let end = parse_index_optional(end)?;
        Ok(BoundItem::Range(start, end, fallback))
    } else {
        let idx = parse_index_required(spec)?;
        Ok(BoundItem::Single(idx, fallback))
    }
}

fn parse_index_required(s: &str) -> Result<i64, ParseError> {
    if s.is_empty() {
        return Err(ParseError::Empty);
    }
    parse_index(s)
}

fn parse_index_optional(s: &str) -> Result<i64, ParseError> {
    if s.is_empty() {
        return Ok(0);
    }
    parse_index(s)
}

fn parse_index(s: &str) -> Result<i64, ParseError> {
    let n: i64 = s
        .parse()
        .map_err(|_| ParseError::NotANumber(s.to_string()))?;
    if n == 0 {
        return Err(ParseError::Zero);
    }
    Ok(n)
}

pub fn resolve_index(idx: i64, len: i64) -> Option<i64> {
    if idx == 0 {
        return None;
    }
    if idx > 0 {
        if idx <= len {
            Some(idx)
        } else {
            None
        }
    } else {
        let pos = len + idx + 1;
        if pos >= 1 && pos <= len {
            Some(pos)
        } else {
            None
        }
    }
}

pub fn normalize_range(start: i64, end: i64, len: i64) -> (i64, i64) {
    let s = if start == 0 { 1 } else { start };
    let e = if end == 0 { len } else { end };
    (s, e)
}
