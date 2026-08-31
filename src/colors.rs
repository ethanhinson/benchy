use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorWhen {
    Always,
    Auto,
    Never,
    Force,
}

impl ColorWhen {
    pub fn use_color(self, stdout_is_tty: bool) -> bool {
        match self {
            Self::Always => true,
            Self::Auto => stdout_is_tty,
            Self::Never => false,
            Self::Force => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Colors {
    pub null: String,
    pub ascii_printable: String,
    pub ascii_whitespace: String,
    pub ascii_other: String,
    pub non_ascii: String,
    pub offset: String,
    pub reset: String,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            null: ansi_color("bright black"),
            ascii_printable: ansi_color("cyan"),
            ascii_whitespace: ansi_color("green"),
            ascii_other: ansi_color("green"),
            non_ascii: ansi_color("yellow"),
            offset: ansi_color("bright black"),
            reset: "\x1b[39m".to_string(),
        }
    }
}

impl Colors {
    pub fn from_env() -> Self {
        let mut colors = Self::default();
        if let Ok(v) = env::var("HEXYL_COLOR_NULL") {
            colors.null = parse_color_spec(&v);
        }
        if let Ok(v) = env::var("HEXYL_COLOR_ASCII_PRINTABLE") {
            colors.ascii_printable = parse_color_spec(&v);
        }
        if let Ok(v) = env::var("HEXYL_COLOR_ASCII_WHITESPACE") {
            colors.ascii_whitespace = parse_color_spec(&v);
        }
        if let Ok(v) = env::var("HEXYL_COLOR_ASCII_OTHER") {
            colors.ascii_other = parse_color_spec(&v);
        }
        if let Ok(v) = env::var("HEXYL_COLOR_NONASCII") {
            colors.non_ascii = parse_color_spec(&v);
        }
        if let Ok(v) = env::var("HEXYL_COLOR_OFFSET") {
            colors.offset = parse_color_spec(&v);
        }
        colors
    }

    pub fn plain() -> Self {
        Self {
            null: String::new(),
            ascii_printable: String::new(),
            ascii_whitespace: String::new(),
            ascii_other: String::new(),
            non_ascii: String::new(),
            offset: String::new(),
            reset: String::new(),
        }
    }

    pub fn color_for_byte(&self, b: u8, scheme: ColorScheme) -> &str {
        match scheme {
            ColorScheme::Default => {
                if b == 0 {
                    &self.null
                } else if (0x20..=0x7e).contains(&b) {
                    &self.ascii_printable
                } else if matches!(b, 0x09 | 0x0a | 0x0c | 0x0d | 0x20) {
                    &self.ascii_whitespace
                } else if b < 0x80 {
                    &self.ascii_other
                } else {
                    &self.non_ascii
                }
            }
            ColorScheme::Gradient => {
                if b == 0 {
                    &self.null
                } else if (0x20..=0x7e).contains(&b) {
                    &self.ascii_printable
                } else if b < 0x80 {
                    gradient_ascii_other(b)
                } else {
                    gradient_non_ascii(b)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    Default,
    Gradient,
}

fn parse_color_spec(spec: &str) -> String {
    let spec = spec.trim();
    if let Some(hex) = spec.strip_prefix('#') {
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return format!("\x1b[38;2;{r};{g};{b}m");
            }
        }
    }
    ansi_color(spec)
}

fn ansi_color(name: &str) -> String {
    let name = name.trim().to_ascii_lowercase();
    let bright = name.starts_with("bright ");
    let base = name.trim_start_matches("bright ").trim();
    let code = match base {
        "black" => 30,
        "red" => 31,
        "green" => 32,
        "yellow" => 33,
        "blue" => 34,
        "magenta" => 35,
        "cyan" => 36,
        "white" => 37,
        _ => 37,
    };
    if bright {
        format!("\x1b[{}m", code + 60)
    } else {
        format!("\x1b[{}m", code)
    }
}

thread_local! {
    static GRADIENT_CACHE: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
}

fn gradient_ascii_other(b: u8) -> &'static str {
    // Pink to violet for non-printable ASCII
    let t = (b as f32) / 127.0;
    let r = (255.0 * (1.0 - t * 0.3)) as u8;
    let g = (105.0 * (1.0 - t) + 50.0 * t) as u8;
    let bv = (180.0 + 75.0 * t) as u8;
    store_gradient(r, g, bv)
}

fn gradient_non_ascii(b: u8) -> &'static str {
    // Heatmap red -> yellow -> white
    let t = ((b as f32) - 128.0) / 127.0;
    let r = 255;
    let g = (255.0 * t.min(0.5) * 2.0) as u8;
    let bv = if t > 0.5 {
        ((t - 0.5) * 2.0 * 255.0) as u8
    } else {
        0
    };
    store_gradient(r, g, bv)
}

fn store_gradient(r: u8, g: u8, b: u8) -> &'static str {
    GRADIENT_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let s = format!("\x1b[38;2;{r};{g};{b}m");
        cache.push(s);
        // SAFETY: we only return references to the last pushed element
        unsafe {
            let ptr = cache.last_mut().unwrap() as *mut String;
            &*ptr
        }
    })
}

pub fn print_color_table(colors: &Colors) {
    println!("hexyl color reference:\n");
    print!("{}", colors.null);
    println!("⋄ NULL bytes (0x00)");
    print!("{}", colors.reset);
    print!("{}", colors.ascii_printable);
    println!("a ASCII printable characters (0x20 - 0x7E)");
    print!("{}", colors.reset);
    print!("{}", colors.ascii_whitespace);
    println!("_ ASCII whitespace (0x09 - 0x0D, 0x20)");
    print!("{}", colors.reset);
    print!("{}", colors.ascii_other);
    println!("• ASCII control characters (except NULL and whitespace)");
    print!("{}", colors.reset);
    print!("{}", colors.non_ascii);
    println!("× Non-ASCII bytes (0x80 - 0xFF)");
    print!("{}", colors.reset);
}
