use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitPrefix {
    Decimal,
    Binary,
}

#[allow(dead_code)]

pub fn parse_byte_count(input: &str, block_size: u64) -> Result<u64> {
    let input = input.trim();
    if input.is_empty() {
        return Err(anyhow!("empty byte count"));
    }

    if let Some(rest) = input.strip_prefix("0x").or_else(|| input.strip_prefix("0X")) {
        return u64::from_str_radix(rest, 16).map_err(|e| anyhow!("invalid hex number: {e}"));
    }

    let lower = input.to_ascii_lowercase();
    if lower.ends_with("block") {
        let num_part = &input[..input.len() - 5];
        let n = parse_number(num_part)?;
        return n.checked_mul(block_size)
            .ok_or_else(|| anyhow!("byte count overflow"));
    }

    let (num_str, multiplier) = parse_with_unit(&lower)?;
    let n = parse_number(num_str)?;
    Ok(n.saturating_mul(multiplier))
}

fn parse_number(s: &str) -> Result<u64> {
    if s.is_empty() {
        return Ok(1);
    }
    s.parse::<u64>().map_err(|e| anyhow!("invalid number '{s}': {e}"))
}

fn parse_with_unit(s: &str) -> Result<(&str, u64)> {
    const UNITS: [(&str, u64); 16] = [
        ("kib", 1024),
        ("mib", 1024 * 1024),
        ("gib", 1024 * 1024 * 1024),
        ("tib", 1024 * 1024 * 1024 * 1024),
        ("pib", 1024_u64.pow(5)),
        ("eib", 1024_u64.pow(6)),
        ("kb", 1000),
        ("mb", 1000 * 1000),
        ("gb", 1000 * 1000 * 1000),
        ("tb", 1000_u64.pow(4)),
        ("pb", 1000_u64.pow(5)),
        ("eb", 1000_u64.pow(6)),
        ("ki", 1024),
        ("mi", 1024 * 1024),
        ("gi", 1024 * 1024 * 1024),
        ("ti", 1024_u64.pow(4)),
    ];

    for (unit, mult) in UNITS {
        if let Some(num) = s.strip_suffix(unit) {
            return Ok((num, mult));
        }
    }

    Ok((s, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex() {
        assert_eq!(parse_byte_count("0xff", 512).unwrap(), 255);
    }

    #[test]
    fn parses_kib() {
        assert_eq!(parse_byte_count("4KiB", 512).unwrap(), 4096);
    }

    #[test]
    fn parses_blocks() {
        assert_eq!(parse_byte_count("2block", 512).unwrap(), 1024);
    }
}
