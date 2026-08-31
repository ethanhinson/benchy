use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundItem {
    Single(i64),
    Range { start: i64, end: Option<i64> },
}

#[derive(Debug, Clone)]
pub struct UserBound {
    pub item: BoundItem,
    pub fallback: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FieldsSpec {
    Bounds(Vec<UserBound>),
    Format(String),
}

impl FieldsSpec {
    pub fn is_format(&self) -> bool {
        matches!(self, FieldsSpec::Format(_))
    }
}

pub fn parse_fields_spec(input: &str) -> Result<FieldsSpec, String> {
    if input.contains('{') {
        return Ok(FieldsSpec::Format(input.to_string()));
    }
    let mut bounds = Vec::new();
    for part in split_commas(input)? {
        bounds.push(parse_one_bound(&part)?);
    }
    if bounds.is_empty() {
        return Err("empty fields spec".into());
    }
    Ok(FieldsSpec::Bounds(bounds))
}

fn split_commas(input: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    for ch in input.chars() {
        if ch == ',' {
            if !cur.is_empty() {
                parts.push(cur.clone());
            }
            cur.clear();
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    Ok(parts)
}

fn parse_one_bound(part: &str) -> Result<UserBound, String> {
    let (core, fallback) = if let Some((left, fb)) = part.split_once('=') {
        (left, Some(fb.to_string()))
    } else {
        (part, None)
    };
    if let Some((start_s, end_s)) = core.split_once(':') {
        let start = if start_s.is_empty() {
            1
        } else {
            parse_index(start_s)?
        };
        let end = if end_s.is_empty() {
            None
        } else {
            Some(parse_index(end_s)?)
        };
        return Ok(UserBound {
            item: BoundItem::Range { start, end },
            fallback,
        });
    }
    Ok(UserBound {
        item: BoundItem::Single(parse_index(core)?),
        fallback,
    })
}

fn parse_index(s: &str) -> Result<i64, String> {
    s.parse::<i64>()
        .map_err(|_| format!("failed to parse '{s}': Not a number `{s}`"))
}

pub fn complement_bounds(bounds: &[UserBound], n: usize) -> Vec<UserBound> {
    let mut selected = vec![false; n];
    for b in bounds {
        for idx in expand_bound(b, n).unwrap_or_default() {
            if idx > 0 && (idx as usize) <= n {
                selected[idx as usize - 1] = true;
            }
        }
    }
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < n {
        if selected[i] {
            i += 1;
            continue;
        }
        let start = (i + 1) as i64;
        let mut j = i + 1;
        while j < n && !selected[j] {
            j += 1;
        }
        if j - i == 1 {
            out.push(UserBound {
                item: BoundItem::Single(start),
                fallback: None,
            });
        } else {
            out.push(UserBound {
                item: BoundItem::Range {
                    start,
                    end: Some(j as i64),
                },
                fallback: None,
            });
        }
        i = j;
    }
    out
}

pub fn resolve_index(idx: i64, n: usize) -> Option<usize> {
    if n == 0 {
        return None;
    }
    let i = if idx < 0 {
        (n as i64 + idx + 1) as isize
    } else {
        idx as isize
    };
    if i >= 1 && (i as usize) <= n {
        Some(i as usize)
    } else {
        None
    }
}

pub fn expand_bound(b: &UserBound, n: usize) -> Result<Vec<usize>, String> {
    match &b.item {
        BoundItem::Single(idx) => match resolve_index(*idx, n) {
            Some(i) => Ok(vec![i]),
            None if b.fallback.is_some() => Ok(vec![]),
            None => Err(format!("Out of bounds: {idx}")),
        },
        BoundItem::Range { start, end } => {
            let s = resolve_index(*start, n).ok_or_else(|| format!("Out of bounds: {start}"))?;
            let e = match end {
                Some(e) => resolve_index(*e, n).ok_or_else(|| format!("Out of bounds: {e}"))?,
                None => n,
            };
            let (lo, hi) = if s <= e { (s, e) } else { (e, s) };
            Ok((lo..=hi).collect())
        }
    }
}

pub fn bound_implies_join(b: &UserBound) -> bool {
    matches!(b.item, BoundItem::Range { .. })
}

impl fmt::Display for BoundItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoundItem::Single(i) => write!(f, "{i}"),
            BoundItem::Range { start, end: None } => write!(f, "{start}:"),
            BoundItem::Range { start, end: Some(e) } => write!(f, "{start}:{e}"),
        }
    }
}
