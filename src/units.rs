use std::num::ParseIntError;

pub fn parse_byte_count(input: &str, block_size: u64) -> Result<i64, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("empty byte count".into());
    }

    if let Some(stripped) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        let v = i64::from_str_radix(stripped, 16)
            .map_err(|e: ParseIntError| e.to_string())?;
        return Ok(v);
    }

    let lower = s.to_ascii_lowercase();
    if lower.ends_with("block") {
        let num_part = &lower[..lower.len() - 5];
        let n: f64 = if num_part.is_empty() {
            1.0
        } else {
            num_part.parse().map_err(|_| format!("invalid block count: {input}"))?
        };
        return Ok((n * block_size as f64) as i64);
    }

    parse_with_unit(s)
}

fn parse_with_unit(s: &str) -> Result<i64, String> {
    let s = s.trim();
    let units: &[(&str, f64)] = &[
        ("kib", 1024.0),
        ("mib", 1024.0 * 1024.0),
        ("gib", 1024.0 * 1024.0 * 1024.0),
        ("tib", 1024.0 * 1024.0 * 1024.0 * 1024.0),
        ("kb", 1000.0),
        ("mb", 1000.0 * 1000.0),
        ("gb", 1000.0 * 1000.0 * 1000.0),
        ("tb", 1000.0 * 1000.0 * 1000.0 * 1000.0),
    ];

    for (unit, mult) in units {
        if s.len() > unit.len() && s[s.len() - unit.len()..].eq_ignore_ascii_case(unit) {
            let num_part = &s[..s.len() - unit.len()];
            let n: f64 = num_part
                .parse()
                .map_err(|_| format!("invalid byte count: {s}"))?;
            return Ok((n * mult) as i64);
        }
    }

    s.parse::<i64>()
        .map_err(|_| format!("invalid byte count: {s}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_and_units() {
        assert_eq!(parse_byte_count("0xff", 512).unwrap(), 255);
        assert_eq!(parse_byte_count("4KiB", 512).unwrap(), 4096);
        assert_eq!(parse_byte_count("2block", 1024).unwrap(), 2048);
    }
}
