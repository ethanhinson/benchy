use std::io::{self, Write};

use crate::args::{
    BorderStyle, CharacterTable, Cli, ColorScheme, ColorWhen, Endianness, NumericBase,
};

const BYTES_PER_PANEL: usize = 8;

pub struct HexPrinter {
    cli: Cli,
    use_color: bool,
    terminal_width: usize,
    panel_count: usize,
}

impl HexPrinter {
    pub fn new(cli: Cli) -> Self {
        let cli = cli.resolved();
        let use_color = match cli.color {
            ColorWhen::Never => false,
            ColorWhen::Always | ColorWhen::Force => true,
            ColorWhen::Auto => atty::is(atty::Stream::Stdout),
        };

        let terminal_width = cli.terminal_width.unwrap_or(80);

        let panel_count = if cli.panels == "auto" {
            if terminal_width >= 80 {
                2
            } else {
                1
            }
        } else {
            cli.panels.parse().unwrap_or(2)
        };

        Self {
            cli,
            use_color,
            terminal_width,
            panel_count,
        }
    }

    pub fn print(&mut self, data: &[u8], display_offset: i64) {
        if matches!(self.cli.border, BorderStyle::None) {
            self.print_plain(data, display_offset);
        } else {
            self.print_bordered(data, display_offset);
        }
    }
}

impl HexPrinter {
    fn bytes_per_line(&self) -> usize {
        BYTES_PER_PANEL * self.panel_count
    }

    fn print_plain(&mut self, data: &[u8], display_offset: i64) {
        let bpl = self.bytes_per_line();
        let mut pos = 0usize;
        let mut prev_full_line: Option<Vec<u8>> = None;

        while pos < data.len() {
            let end = (pos + bpl).min(data.len());
            let chunk = &data[pos..end];
            let is_full = chunk.len() == bpl;

            if !self.cli.no_squeezing && is_full {
                if let Some(ref prev) = prev_full_line {
                    if prev.as_slice() == chunk {
                        let mut skip = pos + bpl;
                        while skip < data.len() {
                            let skip_end = (skip + bpl).min(data.len());
                            if skip_end - skip != bpl || &data[skip..skip_end] != chunk {
                                break;
                            }
                            skip += bpl;
                        }
                        let mut squeeze = String::with_capacity(self.line_width());
                        squeeze.push(' ');
                        squeeze.push('*');
                        squeeze.push_str(&" ".repeat(self.line_width().saturating_sub(2)));
                        let _ = writeln!(io::stdout(), "{squeeze}");
                        pos = skip;
                        if pos >= data.len() {
                            self.print_empty_offset(pos, display_offset);
                        }
                        prev_full_line = None;
                        continue;
                    }
                }
                prev_full_line = Some(chunk.to_vec());
            } else {
                prev_full_line = None;
            }

            self.print_line(pos, chunk, display_offset);
            pos = end;
        }
    }

    fn print_empty_offset(&self, pos: usize, display_offset: i64) {
        if !self.cli.no_position {
            let offset = pos as i64 + display_offset;
            let mut out = format!(" {:08x}", offset as u32);
            out.push_str(&" ".repeat(self.line_width().saturating_sub(out.len())));
            let _ = writeln!(io::stdout(), "{out}");
        }
    }

    fn print_line(&self, pos: usize, chunk: &[u8], display_offset: i64) {
        let mut out = String::new();

        if !self.cli.no_position {
            let offset = pos as i64 + display_offset;
            out.push(' ');
            out.push_str(&format!("{:08x}", offset as u32));
        }

        out.push_str("  ");
        let hex = self.format_hex_panels(chunk);
        out.push_str(&hex);
        out.push_str(&" ".repeat(self.hex_section_width().saturating_sub(hex.len())));

        if !self.cli.no_characters {
            let chars = self.format_chars(chunk);
            let char_count = chars.chars().count();
            let pad = self.char_section_width().saturating_sub(char_count);
            out.push_str(&chars);
            out.push_str(&" ".repeat(pad));
        } else {
            let target = self.line_width();
            out.push_str(&" ".repeat(target.saturating_sub(out.len())));
        }

        let _ = writeln!(io::stdout(), "{out}");
    }

    fn line_width(&self) -> usize {
        let prefix = if self.cli.no_position { 2 } else { 11 };
        prefix + self.hex_section_width() + self.char_section_capacity()
    }

    fn char_section_capacity(&self) -> usize {
        if matches!(self.cli.base, NumericBase::Hexadecimal) {
            18
        } else {
            9
        }
    }

    fn hex_section_width(&self) -> usize {
        let sample_len = if matches!(self.cli.base, NumericBase::Hexadecimal) {
            self.bytes_per_line()
        } else {
            BYTES_PER_PANEL
        };
        let full = self.format_hex_panels(&vec![0u8; sample_len]);
        full.len() + 2
    }

    fn char_section_width(&self) -> usize {
        self.char_section_capacity()
    }

    fn format_hex_panels(&self, chunk: &[u8]) -> String {
        let mut s = String::new();
        let panels = self.panel_count;
        for p in 0..panels {
            let start = p * BYTES_PER_PANEL;
            if start >= chunk.len() {
                break;
            }
            if p > 0 {
                s.push_str("   ");
            }
            let end = ((p + 1) * BYTES_PER_PANEL).min(chunk.len());
            s.push_str(&self.format_hex_group(&chunk[start..end]));
        }
        s
    }

    fn format_hex_group(&self, bytes: &[u8]) -> String {
        match self.cli.base {
            NumericBase::Hexadecimal => self.format_hex(bytes),
            NumericBase::Binary => self.format_binary(bytes),
            NumericBase::Octal => self.format_octal(bytes),
            NumericBase::Decimal => self.format_decimal(bytes),
        }
    }

    fn format_hex(&self, bytes: &[u8]) -> String {
        let gs = self.cli.group_size.as_usize();
        let mut parts = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            let end = (i + gs).min(bytes.len());
            let group = &bytes[i..end];
            if group.len() == gs {
                parts.push(self.format_one_group(group));
            } else {
                for &b in group {
                    parts.push(format!("{:02x}", b));
                }
            }
            i = end;
        }
        parts.join(" ")
    }

    fn format_one_group(&self, group: &[u8]) -> String {
        let mut ordered = group.to_vec();
        if matches!(self.cli.endianness, Endianness::Little) {
            ordered.reverse();
        }
        ordered.iter().map(|b| format!("{:02x}", b)).collect()
    }

    fn format_binary(&self, bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|b| format!("{:08b}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn format_octal(&self, bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|b| format!("{:03o}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn format_decimal(&self, bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|b| format!("{:03}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn format_chars(&self, bytes: &[u8]) -> String {
        let mut s = String::new();
        for (idx, &b) in bytes.iter().enumerate() {
            if idx == BYTES_PER_PANEL {
                s.push(' ');
            }
            s.push_str(&self.char_for_byte_plain(b));
        }
        s
    }

    fn char_for_byte_plain(&self, b: u8) -> String {
        match self.cli.character_table {
            CharacterTable::Default => match b {
                0x00 => '⋄'.to_string(),
                0x09..=0x0d if b != 0x20 => '_'.to_string(),
                0x20 => ' '.to_string(),
                0x21..=0x7e => (b as char).to_string(),
                0x01..=0x08 | 0x0e..=0x1f | 0x7f => '•'.to_string(),
                _ => '×'.to_string(),
            },
            CharacterTable::Ascii => match b {
                0x20 => ' '.to_string(),
                0x21..=0x7e => (b as char).to_string(),
                _ => '.'.to_string(),
            },
            CharacterTable::Codepage1047 | CharacterTable::Codepage437 | CharacterTable::Braille => {
                if (0x21..=0x7e).contains(&b) {
                    (b as char).to_string()
                } else if b == 0x20 {
                    ' '.to_string()
                } else {
                    '.'.to_string()
                }
            }
        }
    }

    fn colorize_char(&self, b: u8, ch: &str) -> String {
        let code = self.color_code(b);
        format!("\x1b[{code}m{ch}\x1b[39m")
    }

    fn color_code(&self, b: u8) -> u8 {
        match self.cli.color_scheme {
            ColorScheme::Default => match b {
                0x00 => 90,
                0x09..=0x0d | 0x20 => 32,
                0x21..=0x7e => 36,
                0x01..=0x08 | 0x0e..=0x1f | 0x7f => 32,
                _ => 33,
            },
            ColorScheme::Gradient => match b {
                0x00 => 90,
                0x21..=0x7e => 36,
                _ if b < 0x80 => 35,
                _ => 31,
            },
        }
    }

    fn print_bordered(&mut self, data: &[u8], display_offset: i64) {
        // Border mode: render with table borders
        let bpl = self.bytes_per_line();
        let mut pos = 0usize;
        let border = match self.cli.border {
            BorderStyle::Ascii => BorderChars::ascii(),
            BorderStyle::Unicode => BorderChars::unicode(),
            BorderStyle::None => BorderChars::unicode(),
        };

        let _ = writeln!(io::stdout(), "{}", border.top());

        while pos < data.len() {
            let end = (pos + bpl).min(data.len());
            let chunk = &data[pos..end];

            if !self.cli.no_squeezing
                && pos >= bpl
                && chunk.len() == bpl
                && data[pos - bpl..pos] == *chunk
            {
                let _ = writeln!(io::stdout(), "{}", border.middle_star());
                pos += bpl;
                continue;
            }

            let offset = pos as i64 + display_offset;
            let hex = self.format_hex_panels(chunk);
            let chars = if self.cli.no_characters {
                String::new()
            } else {
                self.format_chars(chunk)
            };
            let _ = writeln!(
                io::stdout(),
                "{}{}{}{}{}",
                border.row_start(),
                if self.cli.no_position {
                    String::new()
                } else {
                    format!("{:08x}", offset as u32)
                },
                border.row_mid(),
                hex,
                format!("{}{}", chars, border.row_end())
            );
            pos = end;
        }

        let _ = writeln!(io::stdout(), "{}", border.bottom());
    }
}

struct BorderChars {
    tl: &'static str,
    tr: &'static str,
    bl: &'static str,
    br: &'static str,
    h: &'static str,
    v: &'static str,
    lm: &'static str,
    rm: &'static str,
    tm: &'static str,
    bm: &'static str,
    cross: &'static str,
}

impl BorderChars {
    fn unicode() -> Self {
        Self {
            tl: "┌", tr: "┐", bl: "└", br: "┘", h: "─", v: "│",
            lm: "├", rm: "┤", tm: "┬", bm: "┴", cross: "┼",
        }
    }

    fn ascii() -> Self {
        Self {
            tl: "+", tr: "+", bl: "+", br: "+", h: "-", v: "|",
            lm: "+", rm: "+", tm: "+", bm: "+", cross: "+",
        }
    }

    fn top(&self) -> String {
        format!("{}{}{}", self.tl, self.h.repeat(70), self.tr)
    }

    fn bottom(&self) -> String {
        format!("{}{}{}", self.bl, self.h.repeat(70), self.br)
    }

    fn middle_star(&self) -> String {
        format!("{} *", self.v)
    }

    fn row_start(&self) -> &'static str {
        self.v
    }

    fn row_mid(&self) -> &'static str {
        self.v
    }

    fn row_end(&self) -> &'static str {
        self.v
    }
}

pub fn print_include(file: &Option<std::path::PathBuf>, data: &[u8]) {
    let name = file
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("stdin");
    let var = name.replace('.', "_").replace('-', "_");
    print!("unsigned char {var}[] = {{\n  ");
    for (i, b) in data.iter().enumerate() {
        if i > 0 {
            print!(", ");
        }
        print!("0x{:02x}", b);
    }
    println!("\n}};");
    println!("unsigned int {var}_len = {};", data.len());
}

pub fn print_color_table() {
    print!("hexyl color reference:\n\n");
    print!("\x1b[90m⋄ NULL bytes (0x00)\n");
    print!("\x1b[39m\x1b[36ma ASCII printable characters (0x20 - 0x7E)\n");
    print!("\x1b[39m\x1b[32m_ ASCII whitespace (0x09 - 0x0D, 0x20)\n");
    print!("\x1b[39m\x1b[32m• ASCII control characters (except NULL and whitespace)\n");
    print!("\x1b[39m\x1b[33m× Non-ASCII bytes (0x80 - 0xFF)\n");
    print!("\x1b[39m");
}

// Minimal isatty helper without extra crate
mod atty {
    pub enum Stream {
        Stdout,
    }

    pub fn is(_: Stream) -> bool {
        unsafe { libc::isatty(libc::STDOUT_FILENO) != 0 }
    }
}
