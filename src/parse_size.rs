use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseSizeError {
    Empty,
    InvalidUnit,
    InvalidNumber,
    Overflow,
}

impl fmt::Display for ParseSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty size"),
            Self::InvalidUnit => write!(f, "invalid unit"),
            Self::InvalidNumber => write!(f, "invalid number"),
            Self::Overflow => write!(f, "size overflow"),
        }
    }
}

impl std::error::Error for ParseSizeError {}

fn parse_number_part(s: &str) -> Result<u128, ParseSizeError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseSizeError::Empty);
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u128::from_str_radix(hex, 16).map_err(|_| ParseSizeError::InvalidNumber)
    } else if s.contains('.') {
        Err(ParseSizeError::InvalidNumber)
    } else {
        s.parse::<u128>().map_err(|_| ParseSizeError::InvalidNumber)
    }
}

fn decimal_multiplier(unit: &str) -> Result<u128, ParseSizeError> {
    match unit {
        "" => Ok(1),
        "B" | "b" => Ok(1),
        "kB" => Ok(1_000),
        "MB" => Ok(1_000_000),
        "GB" => Ok(1_000_000_000),
        "TB" => Ok(1_000_000_000_000),
        _ => Err(ParseSizeError::InvalidUnit),
    }
}

fn binary_multiplier(unit: &str) -> Result<u128, ParseSizeError> {
    match unit {
        "" => Ok(1),
        "B" | "b" => Ok(1),
        "KiB" | "kiB" | "K" | "k" => Ok(1 << 10),
        "MiB" | "miB" | "M" => Ok(1 << 20),
        "GiB" | "giB" | "G" => Ok(1 << 30),
        "TiB" | "tiB" | "T" => Ok(1 << 40),
        _ => Err(ParseSizeError::InvalidUnit),
    }
}

pub fn parse_size(input: &str, block_size: u64) -> Result<u64, ParseSizeError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseSizeError::Empty);
    }

    if input.ends_with("block") {
        let num_part = input.trim_end_matches("block").trim();
        let blocks = parse_number_part(num_part)?;
        let result = blocks
            .checked_mul(block_size as u128)
            .ok_or(ParseSizeError::Overflow)?;
        return u64::try_from(result).map_err(|_| ParseSizeError::Overflow);
    }

    if input.starts_with("0x") || input.starts_with("0X") {
        let value = parse_number_part(input)?;
        return u64::try_from(value).map_err(|_| ParseSizeError::Overflow);
    }

    let (num_str, unit) = split_number_unit(input);
    let value = parse_number_part(num_str)?;

    let multiplier = if unit.ends_with('B') && unit.contains('i') {
        binary_multiplier(unit)?
    } else if unit.ends_with('B') {
        decimal_multiplier(unit)?
    } else if unit.is_empty() {
        1
    } else {
        return Err(ParseSizeError::InvalidUnit);
    };

    let result = value
        .checked_mul(multiplier)
        .ok_or(ParseSizeError::Overflow)?;
    u64::try_from(result).map_err(|_| ParseSizeError::Overflow)
}

fn split_number_unit(input: &str) -> (&str, &str) {
    let input = input.trim();
    if let Some(pos) = input
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit() && *c != '.')
        .map(|(i, _)| i)
    {
        let (num, unit) = input.split_at(pos);
        (num.trim(), unit.trim())
    } else {
        (input, "")
    }
}
