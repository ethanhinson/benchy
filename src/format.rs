const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

pub fn format_value(value: f64, fix: usize, base: u8) -> String {
    if base == 10 {
        return format_decimal(value, fix);
    }

    let sign = if value.is_sign_negative() { "-" } else { "" };
    let abs = value.abs();
    let factor = 10_f64.powi(fix as i32);
    let rounded = (abs * factor).round() / factor;

    let int_part = rounded.trunc() as u128;
    let frac_part = rounded.fract();
    let int_str = int_to_base(int_part, base);

    let mut frac_str = String::new();
    let frac_digits = if base == 10 { fix } else { 1 };
    if frac_digits > 0 {
        let mut remainder = frac_part;
        for _ in 0..frac_digits {
            remainder *= base as f64;
            let digit = remainder.trunc() as u8;
            remainder -= digit as f64;
            frac_str.push(DIGITS[digit as usize] as char);
        }
    }

    let grouped = group_integer(&int_str);
    if frac_digits > 0 {
        format!("{}{}.{}", sign, grouped, frac_str)
    } else {
        format!("{}{}", sign, grouped)
    }
}

fn format_decimal(value: f64, fix: usize) -> String {
    let formatted = format!("{:.*}", fix, value);
    if value.abs() < 1000.0 {
        return formatted;
    }

    let sign = if value.is_sign_negative() { "-" } else { "" };
    let parts: Vec<&str> = formatted.trim_start_matches('-').split('.').collect();
    let int_part = parts[0];
    let frac_part = parts.get(1).copied().unwrap_or("");

    let grouped = group_decimal_integer(int_part);
    if fix > 0 {
        format!("{sign}{grouped}.{frac_part}")
    } else {
        format!("{sign}{grouped}")
    }
}

fn group_decimal_integer(int_str: &str) -> String {
    let chars: Vec<char> = int_str.chars().collect();
    let mut groups = Vec::new();
    let mut idx = chars.len();

    while idx > 0 {
        let start = idx.saturating_sub(3);
        groups.push(chars[start..idx].iter().collect::<String>());
        idx = start;
    }

    groups.reverse();
    groups.join(",")
}

fn int_to_base(mut value: u128, base: u8) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut digits = Vec::new();
    let b = base as u128;
    while value > 0 {
        let digit = (value % b) as u8;
        digits.push(DIGITS[digit as usize] as char);
        value /= b;
    }
    digits.iter().rev().collect()
}

fn group_integer(int_str: &str) -> String {
    let chars: Vec<char> = int_str.chars().collect();
    let mut groups = Vec::new();
    let mut idx = chars.len();

    while idx > 0 {
        let start = idx.saturating_sub(3);
        groups.push(chars[start..idx].iter().collect::<String>());
        idx = start;
    }

    if groups.is_empty() {
        groups.push("0".to_string());
    }

    while groups.len() < 4 {
        groups.push(String::new());
    }

    groups.reverse();

    groups
        .iter()
        .enumerate()
        .map(|(i, group)| format_group(group, i == 0))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_group(group: &str, is_first: bool) -> String {
    if group.is_empty() {
        if is_first {
            " ".to_string()
        } else {
            "   ".to_string()
        }
    } else if group.len() >= 3 {
        group.to_string()
    } else {
        format!("{:>3}", group)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_formatting() {
        assert_eq!(format_value(2.0, 10, 10), "2.0000000000");
        assert_eq!(format_value(0.3333333333, 5, 10), "0.33333");
    }

    #[test]
    fn hex_formatting() {
        assert_eq!(format_value(255.0, 1, 16), " ,   ,   , FF.0");
        assert_eq!(format_value(4096.0, 1, 16), " ,   ,  1,000.0");
    }
}
