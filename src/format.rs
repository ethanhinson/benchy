const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

pub fn format_output(value: f64, base: u32, fix: usize) -> String {
    if base == 10 {
        format_decimal(value, fix)
    } else {
        format_radix(value, base)
    }
}

fn format_decimal(value: f64, fix: usize) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let abs = value.abs();
    let int_part = abs.trunc() as u64;
    let frac = abs - int_part as f64;

    let int_str = group_commas(&int_part.to_string());
    if fix == 0 {
        return format!("{sign}{int_str}");
    }

    let mut frac_str = String::new();
    let mut remaining = frac;
    for _ in 0..fix {
        remaining *= 10.0;
        let digit = remaining.trunc() as u64;
        frac_str.push(DIGITS[digit as usize] as char);
        remaining -= digit as f64;
    }

    format!("{sign}{int_str}.{frac_str}")
}

fn group_commas(s: &str) -> String {
    if s.len() <= 3 {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*c);
    }
    out
}

fn to_base(mut n: u64, base: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let mut digits = Vec::new();
    while n > 0 {
        digits.push(DIGITS[(n % base as u64) as usize] as char);
        n /= base as u64;
    }
    digits.reverse();
    digits.iter().collect()
}

fn format_radix(value: f64, base: u32) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let n = value.abs().trunc() as u64;
    let raw = to_base(n, base);
    if raw.len() <= 3 {
        format!("{sign} ,   ,   ,{raw:>3}.0")
    } else {
        let grouped = group_commas(&raw);
        format!("{sign} ,   ,  {grouped}.0")
    }
}
