const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

pub fn format_output(value: f64, base: u32, fix: usize) -> String {
    if base == 10 {
        return format_base10(value, fix);
    }
    format_non_base10(value, base, fix)
}

fn is_negative_zero(value: f64) -> bool {
    value.to_bits() == (-0.0_f64).to_bits()
}

fn format_base10(value: f64, fix: usize) -> String {
    if !value.is_finite() {
        return value.to_string();
    }

    let formatted = if is_negative_zero(value) {
        format!("{:.*}", fix, -0.0_f64)
    } else {
        format!("{:.*}", fix, value)
    };

    insert_commas_in_formatted(&formatted)
}

fn insert_commas_in_formatted(formatted: &str) -> String {
    let negative = formatted.starts_with('-');
    let unsigned = formatted.strip_prefix('-').unwrap_or(formatted);
    let (int_part, frac_part) = unsigned
        .split_once('.')
        .map_or((unsigned, None), |(i, f)| (i, Some(f)));

    let grouped = group_integer_base10(int_part.parse::<u128>().unwrap_or(0));

    if let Some(frac) = frac_part {
        if negative {
            format!("-{grouped}.{frac}")
        } else {
            format!("{grouped}.{frac}")
        }
    } else if negative {
        format!("-{grouped}")
    } else {
        grouped
    }
}

fn group_integer_base10(n: u128) -> String {
    if n == 0 {
        return "0".to_string();
    }

    let mut digits = Vec::new();
    let mut value = n;
    while value > 0 {
        digits.push(char::from(b'0' + (value % 10) as u8));
        value /= 10;
    }
    digits.reverse();

    let mut out = String::new();
    for (i, d) in digits.iter().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*d);
    }
    out
}

fn format_non_base10(value: f64, base: u32, fix: usize) -> String {
    if !value.is_finite() {
        return value.to_string();
    }

    let sign = if is_negative_zero(value) || value.is_sign_negative() && value != 0.0 {
        "-"
    } else {
        ""
    };
    let abs = value.abs();
    let int_part = abs.trunc() as u128;
    let frac = abs - int_part as f64;

    let int_groups = group_integer_radix(int_part, base);
    let frac_str = format_fraction_radix(frac, base, fix);
    let frac_display = if frac_str.is_empty() || frac_str.chars().all(|c| c == '0') {
        "0".to_string()
    } else {
        frac_str
    };
    format!("{sign}{int_groups}.{frac_display}")
}

fn compact_comma_grouped(n: u128, base: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }

    let mut digits = Vec::new();
    let mut value = n;
    while value > 0 {
        digits.push(DIGITS[(value % base as u128) as usize] as char);
        value /= base as u128;
    }
    digits.reverse();

    let mut groups = Vec::new();
    let mut idx = digits.len();
    while idx > 0 {
        let start = idx.saturating_sub(3);
        groups.push(digits[start..idx].iter().collect::<String>());
        idx = start;
    }
    groups.reverse();
    groups.join(",")
}

fn group_integer_radix(n: u128, base: u32) -> String {
    if n == 0 {
        return " ,   ,   ,  0".to_string();
    }

    let compact = compact_comma_grouped(n, base);
    let parts: Vec<&str> = compact.split(',').collect();
    let field = match parts.len() {
        1 => match parts[0].len() {
            1 => format!(" ,   ,   ,  {}", parts[0]),
            2 => format!(" ,   ,   , {}", parts[0]),
            _ => format!(" ,   ,   ,{}", parts[0]),
        },
        2 => format!(" ,   ,  {compact}"),
        3 => format!(" ,  {compact}"),
        _ => compact,
    };

    debug_assert_eq!(field.len(), 13, "radix field width mismatch: {field:?}");
    field
}

fn format_fraction_radix(frac: f64, base: u32, fix: usize) -> String {
    let mut remaining = frac;
    let mut out = String::with_capacity(fix);
    for _ in 0..fix {
        remaining *= base as f64;
        let digit = remaining.floor() as u32;
        out.push(DIGITS[digit.min(35) as usize] as char);
        remaining -= digit as f64;
        if remaining.abs() < 1e-12 {
            break;
        }
    }
    out
}
