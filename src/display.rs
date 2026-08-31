use std::io::{self, Write};

use crate::charset::CharacterTable;
use crate::config::{Base, BorderStyle, Config, Endianness};

const BYTES_PER_PANEL: usize = 8;
const DEFAULT_PANELS: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayRow {
    pub offset: u64,
    pub hex_panels: Vec<String>,
    pub char_panels: Vec<String>,
    pub is_squeeze_marker: bool,
}

pub fn render(data: &[u8], config: &Config, out: &mut dyn Write) -> io::Result<()> {
    if config.include {
        return render_include(data, config, out);
    }

    if data.is_empty() {
        return render_empty(config, out);
    }

    let bytes_per_line = bytes_per_line(config);
    let rows = build_rows(data, config);
    let rows = if config.no_squeezing {
        rows
    } else {
        apply_squeezing(rows, data.len(), bytes_per_line, config.display_offset as u64)
    };

    match config.border {
        BorderStyle::None => render_borderless(&rows, config, out),
        BorderStyle::Ascii => render_bordered(&rows, config, out, true),
        BorderStyle::Unicode => render_bordered(&rows, config, out, false),
    }
}

fn panel_count(config: &Config) -> usize {
    if matches!(config.base, Base::Octal | Base::Decimal | Base::Binary) {
        return 1;
    }
    config.panels.unwrap_or(DEFAULT_PANELS).max(1)
}

fn bytes_per_line(config: &Config) -> usize {
    if matches!(config.base, Base::Octal | Base::Decimal | Base::Binary) {
        BYTES_PER_PANEL
    } else {
        BYTES_PER_PANEL * panel_count(config)
    }
}

fn hex_content_width(config: &Config) -> usize {
    match config.base {
        Base::Hexadecimal => match config.group_size {
            1 => 23,
            2 => 19,
            4 | 8 => 17,
            _ => 23,
        },
        Base::Binary => 73,
        Base::Octal | Base::Decimal => 33,
    }
}

fn hex_panel_width(config: &Config) -> usize {
    match config.base {
        Base::Hexadecimal if config.group_size == 1 => 25,
        Base::Hexadecimal if config.group_size == 2 => 21,
        _ => hex_content_width(config),
    }
}

fn build_rows(data: &[u8], config: &Config) -> Vec<DisplayRow> {
    let bytes_per_line = bytes_per_line(config);
    let panels = panel_count(config);
    let mut rows = Vec::new();
    let mut offset = 0usize;

    while offset < data.len() {
        let end = (offset + bytes_per_line).min(data.len());
        let chunk = &data[offset..end];
        let display_offset = config.display_offset as u64 + offset as u64;
        let (hex_panels, char_panels) = format_line(chunk, config, panels);
        rows.push(DisplayRow {
            offset: display_offset,
            hex_panels,
            char_panels,
            is_squeeze_marker: false,
        });
        offset = end;
    }
    rows
}

fn apply_squeezing(rows: Vec<DisplayRow>, data_len: usize, bytes_per_line: usize, display_offset: u64) -> Vec<DisplayRow> {
    if rows.is_empty() {
        return rows;
    }
    let mut result = Vec::new();
    let mut had_squeeze = false;
    let mut i = 0;
    while i < rows.len() {
        let current = rows[i].clone();
        result.push(current.clone());
        let mut j = i + 1;
        while j < rows.len()
            && rows[j].hex_panels == current.hex_panels
            && rows[j].char_panels == current.char_panels
        {
            j += 1;
        }
        if j > i + 1 {
            had_squeeze = true;
            result.push(DisplayRow {
                offset: 0,
                hex_panels: vec![String::new(); current.hex_panels.len()],
                char_panels: vec![String::new(); current.char_panels.len()],
                is_squeeze_marker: true,
            });
        }
        i = j;
    }

    if had_squeeze && data_len > 0 && data_len % bytes_per_line == 0 {
        result.push(DisplayRow {
            offset: display_offset + data_len as u64,
            hex_panels: vec![String::new(); result[0].hex_panels.len()],
            char_panels: vec![String::new(); result[0].char_panels.len()],
            is_squeeze_marker: false,
        });
    }

    result
}

fn format_line(chunk: &[u8], config: &Config, panels: usize) -> (Vec<String>, Vec<String>) {
    let mut hex_panels = Vec::with_capacity(panels);
    let mut char_panels = Vec::with_capacity(panels);

    for panel_idx in 0..panels {
        let start = panel_idx * BYTES_PER_PANEL;
        let end = (start + BYTES_PER_PANEL).min(chunk.len());
        if start >= chunk.len() {
            hex_panels.push(String::new());
            char_panels.push(String::new());
            continue;
        }
        let panel_bytes = &chunk[start..end];
        hex_panels.push(format_hex_panel(panel_bytes, config));
        char_panels.push(format_char_panel(panel_bytes, config.character_table));
    }

    (hex_panels, char_panels)
}

fn format_hex_panel(bytes: &[u8], config: &Config) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    let gs = config.group_size as usize;
    let mut groups = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let end = (i + gs).min(bytes.len());
        let mut group = bytes[i..end].to_vec();
        if config.endianness == Endianness::Little && group.len() > 1 {
            group.reverse();
        }
        groups.push(format_group(&group, config.base));
        i = end;
    }
    format!(" {}", groups.join(" "))
}

fn format_group(bytes: &[u8], base: Base) -> String {
    match base {
        Base::Hexadecimal => bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        Base::Binary => bytes
            .iter()
            .map(|b| format!("{b:08b}"))
            .collect::<Vec<_>>()
            .join(" "),
        Base::Octal => bytes
            .iter()
            .map(|b| format!("{b:03o}"))
            .collect::<Vec<_>>()
            .join(" "),
        Base::Decimal => bytes
            .iter()
            .map(|b| format!("{b:03}"))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn format_char_panel(bytes: &[u8], table: CharacterTable) -> String {
    bytes.iter().map(|b| table.map_byte(*b)).collect()
}

fn render_empty(config: &Config, out: &mut dyn Write) -> io::Result<()> {
    let panels = panel_count(config);
    let hex_w = hex_panel_width(config);
    let char_w = if config.show_characters { BYTES_PER_PANEL } else { 0 };

    match config.border {
        BorderStyle::None => writeln!(out),
        BorderStyle::Ascii => {
            writeln!(out, "{}", top_border('+', '-', '+', panels, hex_w, char_w, config.show_position))?;
            writeln!(out, "{}", empty_row('|', '|', panels, hex_w, char_w, config.show_position))?;
            writeln!(out, "{}", bottom_border('+', '-', '+', panels, hex_w, char_w, config.show_position))
        }
        BorderStyle::Unicode => {
            writeln!(out, "{}", top_border('┌', '─', '┬', panels, hex_w, char_w, config.show_position))?;
            writeln!(out, "{}", empty_row('│', '┊', panels, hex_w, char_w, config.show_position))?;
            writeln!(out, "{}", bottom_border('└', '─', '┴', panels, hex_w, char_w, config.show_position))
        }
    }
}

fn empty_row(vert: char, sep: char, panels: usize, hex_w: usize, char_w: usize, show_pos: bool) -> String {
    let mut line = String::new();
    line.push(vert);
    if show_pos {
        line.push_str(&format!("{:<8}", ""));
        line.push(vert);
    }
    line.push_str(" No content ");
    line.push_str(&" ".repeat(hex_w.saturating_sub(" No content ".len())));
    for i in 1..panels {
        if i > 0 {
            line.push(sep);
        }
        line.push_str(&" ".repeat(hex_w));
    }
    if char_w > 0 {
        line.push(vert);
        for i in 0..panels {
            if i > 0 {
                line.push(sep);
            }
            line.push_str(&" ".repeat(char_w));
        }
    }
    line.push(vert);
    line
}

fn top_border(left: char, horiz: char, mid: char, panels: usize, hex_w: usize, char_w: usize, show_pos: bool) -> String {
    let right = match left {
        '┌' => '┐',
        '+' => '+',
        _ => left,
    };
    border_line(left, right, horiz, mid, panels, hex_w, char_w, show_pos)
}

fn bottom_border(left: char, horiz: char, mid: char, panels: usize, hex_w: usize, char_w: usize, show_pos: bool) -> String {
    let right = match left {
        '└' => '┘',
        '+' => '+',
        _ => left,
    };
    border_line(left, right, horiz, mid, panels, hex_w, char_w, show_pos)
}

fn border_line(left: char, right: char, horiz: char, mid: char, panels: usize, hex_w: usize, char_w: usize, show_pos: bool) -> String {
    let h = horiz.to_string().repeat(hex_w);
    let p = horiz.to_string().repeat(8);
    let c = horiz.to_string().repeat(char_w);
    let mut s = String::new();
    s.push(left);
    if show_pos {
        s.push_str(&p);
        s.push(mid);
    }
    for i in 0..panels {
        if i > 0 {
            s.push(mid);
        }
        s.push_str(&h);
    }
    if char_w > 0 {
        for _ in 0..panels {
            s.push(mid);
            s.push_str(&c);
        }
    }
    s.push(right);
    s
}

fn render_bordered(rows: &[DisplayRow], config: &Config, out: &mut dyn Write, ascii: bool) -> io::Result<()> {
    let panels = panel_count(config);
    let hex_w = hex_panel_width(config);
    let char_w = if config.show_characters { BYTES_PER_PANEL } else { 0 };

    let (tl, th, tm, _tr, v, bl, bm, _br, sep) = if ascii {
        ('+', '-', '+', '+', '|', '+', '+', '+', '|')
    } else {
        ('┌', '─', '┬', '┐', '│', '└', '┴', '┘', '┊')
    };

    writeln!(out, "{}", top_border(tl, th, tm, panels, hex_w, char_w, config.show_position))?;

    for row in rows {
        if row.is_squeeze_marker {
            writeln!(out, "{}", squeeze_line(v, sep, panels, hex_w, char_w, config.show_position))?;
        } else {
            writeln!(out, "{}", content_line(v, sep, row, hex_w, char_w, config))?;
        }
    }

    writeln!(out, "{}", bottom_border(bl, th, bm, panels, hex_w, char_w, config.show_position))
}

fn content_line(vert: char, sep: char, row: &DisplayRow, hex_w: usize, char_w: usize, config: &Config) -> String {
    let mut line = String::new();
    line.push(vert);
    if config.show_position {
        line.push_str(&format!("{:08x}", row.offset));
        line.push(vert);
    }
    for (i, hex) in row.hex_panels.iter().enumerate() {
        if i > 0 {
            line.push(sep);
        }
        line.push_str(&format!("{hex:<width$}", width = hex_w));
    }
    if char_w > 0 {
        line.push(vert);
        for (i, chars) in row.char_panels.iter().enumerate() {
            if i > 0 {
                line.push(sep);
            }
            line.push_str(&format!("{chars:<width$}", width = char_w));
        }
    }
    line.push(vert);
    line
}

fn squeeze_line(vert: char, sep: char, panels: usize, hex_w: usize, char_w: usize, show_pos: bool) -> String {
    let mut line = String::new();
    line.push(vert);
    if show_pos {
        line.push_str(&format!("{:<8}", "*"));
        line.push(vert);
    }
    for i in 0..panels {
        if i > 0 {
            line.push(sep);
        }
        line.push_str(&" ".repeat(hex_w));
    }
    if char_w > 0 {
        line.push(vert);
        for i in 0..panels {
            if i > 0 {
                line.push(sep);
            }
            line.push_str(&" ".repeat(char_w));
        }
    }
    line.push(vert);
    line
}

fn render_borderless(rows: &[DisplayRow], config: &Config, out: &mut dyn Write) -> io::Result<()> {
    let panels = panel_count(config);
    let hex_w = hex_content_width(config);

    for row in rows {
        let mut line = if config.show_position {
            String::from(" ")
        } else {
            String::from("  ")
        };

        if row.is_squeeze_marker {
            if config.show_position {
                line.push_str("*       ");
            } else {
                line.clear();
                line.push_str("  ");
            }
            for i in 0..panels {
                if i > 0 {
                    line.push_str("   ");
                }
                line.push_str(&" ".repeat(hex_w));
            }
            if config.show_characters {
                line.push_str("     ");
                for (i, chars) in row.char_panels.iter().enumerate() {
                    if i > 0 {
                        line.push(' ');
                    }
                    line.push_str(chars);
                }
                line.push_str("  ");
            } else if !config.show_position {
                line.push_str("     ");
            }
        } else {
            if config.show_position {
                line.push_str(&format!("{:08x}  ", row.offset));
            }
            for (i, hex) in row.hex_panels.iter().enumerate() {
                if i > 0 {
                    line.push_str("   ");
                }
                let hex_body = hex.strip_prefix(' ').unwrap_or(hex);
                line.push_str(hex_body);
            }
            if config.show_characters {
                line.push_str("     ");
                for (i, chars) in row.char_panels.iter().enumerate() {
                    if i > 0 {
                        line.push(' ');
                    }
                    line.push_str(chars);
                }
                line.push_str("  ");
            } else if !config.show_position {
                line.push_str("     ");
            }
        }
        writeln!(out, "{line}")?;
    }
    Ok(())
}

fn render_include(data: &[u8], config: &Config, out: &mut dyn Write) -> io::Result<()> {
    let name = config.file_name.as_deref().unwrap_or("stdin");
    let var_name = sanitize_name(name);
    writeln!(out, "unsigned char {var_name}[] = {{")?;
    write!(out, "  ")?;
    let mut line_start = true;
    for (i, byte) in data.iter().enumerate() {
        if !line_start {
            write!(out, ", ")?;
        }
        line_start = false;
        write!(out, "0x{byte:02x}")?;
        if (i + 1) % 12 == 0 && i + 1 < data.len() {
            writeln!(out, ",")?;
            write!(out, "  ")?;
            line_start = true;
        }
    }
    writeln!(out, "\n}};")?;
    writeln!(out, "unsigned int {var_name}_len = {};", data.len())
}

fn sanitize_name(name: &str) -> String {
    let file_name = std::path::Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("stdin");
    let sanitized: String = file_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if sanitized.is_empty() {
        "stdin".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn render_to_string(data: &[u8], config: &Config) -> String {
        let mut buf = Vec::new();
        render(data, config, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn plain_output_matches_shape() {
        let data = b"Hello\x00World\xffABC";
        let mut config = Config::default();
        config.apply_plain();
        let out = render_to_string(data, &config);
        assert!(out.contains("48 65 6c"));
    }
}
