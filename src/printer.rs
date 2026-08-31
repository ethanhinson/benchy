use std::io::{self, Read, Write};

use crate::chars::CharacterTable;
use crate::colors::{ColorScheme, Colors};

const BYTES_PER_GROUP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    Unicode,
    Ascii,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayBase {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}

pub struct PrinterConfig {
    pub show_position: bool,
    pub show_characters: bool,
    pub border: BorderStyle,
    pub squeeze: bool,
    pub colors: Colors,
    pub use_color: bool,
    pub color_scheme: ColorScheme,
    pub character_table: CharacterTable,
    pub display_offset: u64,
    pub group_size: usize,
    pub endianness: Endianness,
    pub base: DisplayBase,
    pub bytes_per_panel: usize,
    pub panel_count: usize,
}

impl PrinterConfig {
    pub fn bytes_per_line(&self) -> usize {
        self.bytes_per_panel * self.panel_count
    }
}

pub struct LineContent {
    pub offset: u64,
    pub bytes: Vec<u8>,
    pub plain_body: String,
    pub colored_body: String,
    pub squeeze_key: String,
}

pub fn read_input(
    mut reader: impl Read,
    skip: u64,
    length: Option<u64>,
    file_len: Option<u64>,
) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;

    let effective_len = file_len.unwrap_or(data.len() as u64);
    let start = if skip < 0i64 as u64 {
        0
    } else if skip > effective_len {
        effective_len
    } else {
        skip
    };

    let start = start as usize;
    let mut sliced = if start >= data.len() {
        Vec::new()
    } else {
        data[start..].to_vec()
    };

    if let Some(len) = length {
        let len = len as usize;
        if sliced.len() > len {
            sliced.truncate(len);
        }
    }

    Ok(sliced)
}

pub fn resolve_skip(skip: i64, file_len: u64) -> u64 {
    if skip >= 0 {
        skip as u64
    } else {
        let abs = (-skip) as u64;
        file_len.saturating_sub(abs)
    }
}

pub fn print_include(name: &str, data: &[u8], out: &mut impl Write) -> io::Result<()> {
    let var = sanitize_name(name);
    writeln!(out, "unsigned char {var}[] = {{")?;
    for (i, chunk) in data.chunks(12).enumerate() {
        let hex = chunk
            .iter()
            .map(|b| format!("0x{b:02x}"))
            .collect::<Vec<_>>()
            .join(", ");
        if i + 1 == data.chunks(12).len() {
            writeln!(out, "  {hex}")?;
        } else {
            writeln!(out, "  {hex},")?;
        }
    }
    writeln!(out, "}};")?;
    writeln!(out, "unsigned int {var}_len = {};", data.len())?;
    Ok(())
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

pub fn print_data(data: &[u8], cfg: &PrinterConfig, out: &mut impl Write) -> io::Result<()> {
    if cfg.border == BorderStyle::None {
        print_plain_layout(data, cfg, out)
    } else {
        print_bordered_layout(data, cfg, out)
    }
}

fn print_plain_layout(data: &[u8], cfg: &PrinterConfig, out: &mut impl Write) -> io::Result<()> {
    let bpl = cfg.bytes_per_line();
    let mut prev_line: Option<String> = None;
    let mut in_squeeze = false;

    let mut offset = 0usize;
    while offset < data.len() {
        let end = (offset + bpl).min(data.len());
        let chunk = &data[offset..end];
        let line_offset = cfg.display_offset + offset as u64;

        let line = format_line(chunk, line_offset, cfg, false);

        if cfg.squeeze {
            if Some(&line.squeeze_key) == prev_line.as_ref() {
                if !in_squeeze {
                    in_squeeze = true;
                }
                offset = end;
                continue;
            }

            if in_squeeze {
                write_squeeze_star_line(out, cfg)?;
                in_squeeze = false;
            }

            write_line(out, &line, cfg)?;
            prev_line = Some(line.squeeze_key.clone());
        } else {
            write_line(out, &line, cfg)?;
        }

        offset = end;
    }

    if cfg.squeeze && in_squeeze {
        write_squeeze_star_line(out, cfg)?;
        write_squeeze_end_line(out, cfg.display_offset + data.len() as u64, cfg)?;
    }

    Ok(())
}

fn print_bordered_layout(data: &[u8], cfg: &PrinterConfig, out: &mut impl Write) -> io::Result<()> {
    let bpl = cfg.bytes_per_line();
    let mut rows: Vec<BorderRow> = Vec::new();
    let mut offset = 0usize;
    let mut prev_key: Option<String> = None;
    let mut in_squeeze = false;

    while offset < data.len() {
        let end = (offset + bpl).min(data.len());
        let chunk = &data[offset..end];
        let line_offset = cfg.display_offset + offset as u64;
        let row = build_border_row(chunk, line_offset, cfg);

        if cfg.squeeze {
            if Some(&row.squeeze_key) == prev_key.as_ref() {
                in_squeeze = true;
                offset = end;
                continue;
            }
            if in_squeeze {
                rows.push(BorderRow::squeeze_star());
                in_squeeze = false;
            }
            prev_key = Some(row.squeeze_key.clone());
        }

        rows.push(row);
        offset = end;
    }

    if cfg.squeeze && in_squeeze {
        rows.push(BorderRow::squeeze_star());
        rows.push(BorderRow::squeeze_end(
            cfg.display_offset + data.len() as u64,
        ));
    }

    if rows.is_empty() {
        return Ok(());
    }

    let cols = border_columns(cfg);
    let style = cfg.border;
    writeln!(out, "{}", render_border_frame(&cols, style, FramePart::Top))?;
    for row in &rows {
        writeln!(out, "{}", render_border_data_row(row, &cols, style, cfg))?;
    }
    writeln!(out, "{}", render_border_frame(&cols, style, FramePart::Bottom))?;
    Ok(())
}

enum FramePart {
    Top,
    Bottom,
}

struct BorderColumns {
    offset: usize,
    hex_panel: usize,
    char_panel: usize,
    hex_panels: usize,
    char_panels: usize,
}

struct BorderRow {
    offset: u64,
    hex_panels: Vec<String>,
    char_panels: Vec<String>,
    squeeze_key: String,
    kind: BorderRowKind,
}

enum BorderRowKind {
    Data,
    SqueezeStar,
    SqueezeEnd,
}

impl BorderRow {
    fn squeeze_star() -> Self {
        Self {
            offset: 0,
            hex_panels: Vec::new(),
            char_panels: Vec::new(),
            squeeze_key: "*".to_string(),
            kind: BorderRowKind::SqueezeStar,
        }
    }

    fn squeeze_end(offset: u64) -> Self {
        Self {
            offset,
            hex_panels: Vec::new(),
            char_panels: Vec::new(),
            squeeze_key: String::new(),
            kind: BorderRowKind::SqueezeEnd,
        }
    }
}

fn border_columns(cfg: &PrinterConfig) -> BorderColumns {
    BorderColumns {
        offset: 8,
        hex_panel: cfg.bytes_per_panel * 3 + 1,
        char_panel: cfg.bytes_per_panel,
        hex_panels: cfg.panel_count,
        char_panels: if cfg.show_characters {
            cfg.panel_count
        } else {
            0
        },
    }
}

fn build_border_row(chunk: &[u8], offset: u64, cfg: &PrinterConfig) -> BorderRow {
    let mut hex_panels = Vec::new();
    for panel in 0..cfg.panel_count {
        let start = panel * cfg.bytes_per_panel;
        let end = ((panel + 1) * cfg.bytes_per_panel).min(chunk.len());
        let content = if start >= chunk.len() {
            String::new()
        } else {
            format_panel_hex(&chunk[start..end], cfg, false)
        };
        hex_panels.push(pad_field(format!(" {content} "), cfg.bytes_per_panel * 3 + 1));
    }

    let mut char_panels = Vec::new();
    if cfg.show_characters {
        for panel in 0..cfg.panel_count {
            let start = panel * cfg.bytes_per_panel;
            let end = ((panel + 1) * cfg.bytes_per_panel).min(chunk.len());
            let mut chars = String::new();
            if start < chunk.len() {
                for &b in &chunk[start..end] {
                    chars.push(cfg.character_table.map(b));
                }
            }
            char_panels.push(pad_field(chars, cfg.bytes_per_panel));
        }
    }

    let squeeze_key = format!(
        "{}|{}",
        hex_panels.join("|"),
        char_panels.join("|")
    );

    BorderRow {
        offset,
        hex_panels,
        char_panels,
        squeeze_key,
        kind: BorderRowKind::Data,
    }
}

fn pad_field(mut content: String, width: usize) -> String {
    while display_width(&content) < width {
        content.push(' ');
    }
    if display_width(&content) > width {
        content = content.chars().take(width).collect();
    }
    content
}

fn render_border_frame(cols: &BorderColumns, style: BorderStyle, part: FramePart) -> String {
    let (corner, cross, hline, end) = match (style, part) {
        (BorderStyle::Unicode, FramePart::Top) => ('┌', '┬', '─', '┐'),
        (BorderStyle::Unicode, FramePart::Bottom) => ('└', '┴', '─', '┘'),
        (BorderStyle::Ascii, FramePart::Top) => ('+', '+', '-', '+'),
        (BorderStyle::Ascii, FramePart::Bottom) => ('+', '+', '-', '+'),
        _ => ('+', '+', '-', '+'),
    };

    let mut sections = vec![cols.offset];
    sections.extend(std::iter::repeat(cols.hex_panel).take(cols.hex_panels));
    sections.extend(std::iter::repeat(cols.char_panel).take(cols.char_panels));

    let mut out = String::from(corner);
    for (i, width) in sections.iter().enumerate() {
        out.push_str(&hline.to_string().repeat(*width));
        out.push(if i + 1 == sections.len() { end } else { cross });
    }
    out
}

fn render_border_data_row(
    row: &BorderRow,
    cols: &BorderColumns,
    style: BorderStyle,
    cfg: &PrinterConfig,
) -> String {
    let vbar = match style {
        BorderStyle::Unicode => '│',
        BorderStyle::Ascii => '|',
        _ => '|',
    };
    let inner_hex = match style {
        BorderStyle::Unicode => '┊',
        BorderStyle::Ascii => '|',
        _ => '|',
    };
    let inner_char = inner_hex;

    let mut out = String::from(vbar);

    match row.kind {
        BorderRowKind::SqueezeStar => {
            out.push('*');
            out.push_str(&" ".repeat(cols.offset - 1));
            out.push(vbar);
            for _ in 0..cols.hex_panels {
                out.push_str(&" ".repeat(cols.hex_panel));
                out.push(vbar);
            }
            for _ in 0..cols.char_panels {
                out.push_str(&" ".repeat(cols.char_panel));
                out.push(vbar);
            }
        }
        BorderRowKind::SqueezeEnd => {
            out.push_str(&format!("{:08x}", row.offset));
            out.push(vbar);
            for _ in 0..cols.hex_panels {
                out.push_str(&" ".repeat(cols.hex_panel));
                out.push(vbar);
            }
            for _ in 0..cols.char_panels {
                out.push_str(&" ".repeat(cols.char_panel));
                out.push(vbar);
            }
        }
        BorderRowKind::Data => {
            out.push_str(&format!("{:08x}", row.offset));
            out.push(vbar);
            for (i, panel) in row.hex_panels.iter().enumerate() {
                out.push_str(panel);
                out.push(if i + 1 == row.hex_panels.len() {
                    vbar
                } else {
                    inner_hex
                });
            }
            if cfg.show_characters {
                for (i, panel) in row.char_panels.iter().enumerate() {
                    out.push_str(panel);
                    out.push(if i + 1 == row.char_panels.len() {
                        vbar
                    } else {
                        inner_char
                    });
                }
            }
        }
    }

    out
}

fn write_squeeze_star_line(out: &mut impl Write, cfg: &PrinterConfig) -> io::Result<()> {
    let width = default_line_width(cfg);
    let mut star_line = " *".to_string();
    while display_width(&star_line) < width {
        star_line.push(' ');
    }
    writeln!(out, "{star_line}")
}

fn write_squeeze_end_line(out: &mut impl Write, end_offset: u64, cfg: &PrinterConfig) -> io::Result<()> {
    if !cfg.show_position {
        return Ok(());
    }
    let width = default_line_width(cfg);
    let mut offset_line = format!(" {:08x}", end_offset);
    while display_width(&offset_line) < width {
        offset_line.push(' ');
    }
    writeln!(out, "{offset_line}")
}

fn write_line(out: &mut impl Write, line: &LineContent, cfg: &PrinterConfig) -> io::Result<()> {
    if cfg.use_color {
        write!(out, "{}", line.colored_body)?;
    } else {
        write!(out, "{}", line.plain_body)?;
    }
    writeln!(out)?;
    Ok(())
}

fn format_line(chunk: &[u8], offset: u64, cfg: &PrinterConfig, bordered: bool) -> LineContent {
    let mut plain = String::new();
    let mut colored = String::new();

    if cfg.show_position {
        let pos = format!(" {:08x}", offset);
        plain.push_str(&pos);
        if cfg.use_color {
            colored.push_str(&cfg.colors.offset);
            colored.push_str(&pos);
            colored.push_str(&cfg.colors.reset);
        } else {
            colored.push_str(&pos);
        }
        plain.push_str("  ");
        colored.push_str("  ");
    } else {
        plain.push_str("  ");
        colored.push_str("  ");
    }

    let hex_plain = format_hex_section(chunk, cfg, false);
    let hex_colored = format_hex_section(chunk, cfg, true);
    plain.push_str(&hex_plain);
    colored.push_str(&hex_colored);

    if cfg.show_characters {
        let target = if uses_fixed_char_column(cfg) {
            char_section_start(cfg)
        } else {
            display_width(&plain) + 2
        };
        while display_width(&plain) < target {
            plain.push(' ');
        }
        while display_width(&colored) < target {
            colored.push(' ');
        }
        let chars_plain = format_char_section(chunk, cfg, false);
        let chars_colored = format_char_section(chunk, cfg, true);
        plain.push_str(&chars_plain);
        colored.push_str(&chars_colored);
    } else {
        pad_to_char_column(&mut plain, cfg, bordered);
        pad_to_char_column(&mut colored, cfg, bordered);
    }

    pad_line_end(&mut plain, cfg);
    pad_line_end(&mut colored, cfg);

    let squeeze_key = if cfg.show_position {
        plain[11..].to_string()
    } else {
        plain[2..].to_string()
    };

    LineContent {
        offset,
        bytes: chunk.to_vec(),
        plain_body: plain,
        colored_body: colored,
        squeeze_key,
    }
}

fn total_line_width(cfg: &PrinterConfig) -> usize {
    let mut width = 0;
    if cfg.show_position {
        width += 10;
    } else {
        width += 1;
    }
    width += 2 + hex_section_width(cfg);
    if cfg.show_characters {
        width += char_section_start(cfg) - (if cfg.show_position { 10 } else { 1 }) - 2 - hex_section_width(cfg);
        width += char_section_width(cfg);
    }
    width + 4
}

fn hex_section_width(cfg: &PrinterConfig) -> usize {
    let mut width = 0;
    for panel in 0..cfg.panel_count {
        let panel_bytes = cfg.bytes_per_panel;
        width += format_panel_hex_width(panel_bytes, cfg);
        if panel + 1 < cfg.panel_count {
            width += 3;
        }
    }
    width
}

fn format_panel_hex_width(bytes: usize, cfg: &PrinterConfig) -> usize {
    let groups = bytes.div_ceil(cfg.group_size);
    match cfg.base {
        DisplayBase::Hexadecimal => groups * (cfg.group_size * 3 - 1) + groups.saturating_sub(1),
        DisplayBase::Binary => groups * (cfg.group_size * 8 + groups.saturating_sub(1)),
        DisplayBase::Octal | DisplayBase::Decimal => {
            bytes * 4 - 1
        }
    }
}

fn char_section_start(cfg: &PrinterConfig) -> usize {
    if cfg.show_position {
        9 + 2 + hex_section_width(cfg) + 2
    } else {
        2 + hex_section_width(cfg) + 2
    }
}

fn char_section_width(cfg: &PrinterConfig) -> usize {
    cfg.bytes_per_panel * cfg.panel_count + cfg.panel_count.saturating_sub(1)
}

fn pad_to_char_column(s: &mut String, cfg: &PrinterConfig, _bordered: bool) {
    let target = char_section_start(cfg);
    while display_width(s) < target {
        s.push(' ');
    }
}

fn display_width(s: &str) -> usize {
    s.chars().count()
}

fn pad_line_end(s: &mut String, cfg: &PrinterConfig) {
    if let Some(width) = target_line_width(cfg) {
        while display_width(s) < width {
            s.push(' ');
        }
    } else if cfg.show_characters {
        s.push(' ');
    }
}

fn target_line_width(cfg: &PrinterConfig) -> Option<usize> {
    if !uses_fixed_char_column(cfg) {
        return None;
    }
    if cfg.show_position {
        Some(if cfg.show_characters {
            80
        } else {
            char_section_start(cfg)
        })
    } else {
        Some(if cfg.show_characters {
            71
        } else {
            char_section_start(cfg) - 9
        })
    }
}

fn default_line_width(cfg: &PrinterConfig) -> usize {
    target_line_width(cfg).unwrap_or(80)
}

fn uses_fixed_char_column(cfg: &PrinterConfig) -> bool {
    cfg.base == DisplayBase::Hexadecimal && cfg.group_size == 1
}

pub fn effective_panel_count(cfg: &PrinterConfig, terminal_width: Option<u16>) -> usize {
    if cfg.base == DisplayBase::Binary {
        let width = terminal_width.unwrap_or(80) as usize;
        let bytes_per_binary = 9; // 8 digits + space
        let panel_hex_width = BYTES_PER_GROUP * bytes_per_binary - 1;
        let panel_total = panel_hex_width + 3 + 18; // hex + sep + chars approx
        let overhead = if cfg.show_position { 11 } else { 2 };
        if width >= overhead + panel_total * 2 {
            cfg.panel_count.max(1).min(2)
        } else {
            1
        }
    } else {
        cfg.panel_count
    }
}

fn format_hex_section(data: &[u8], cfg: &PrinterConfig, colorize: bool) -> String {
    let mut out = String::new();
    let use_color = colorize && cfg.use_color;

    for panel in 0..cfg.panel_count {
        if panel > 0 {
            out.push_str("   ");
        }
        let start = panel * cfg.bytes_per_panel;
        let end = ((panel + 1) * cfg.bytes_per_panel).min(data.len());
        if start >= data.len() {
            break;
        }
        let panel_data = &data[start..end];
        out.push_str(&format_panel_hex(panel_data, cfg, use_color));
    }

    out
}

fn format_panel_hex(data: &[u8], cfg: &PrinterConfig, use_color: bool) -> String {
    let mut parts = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let gs = cfg.group_size.min(data.len() - i);
        let group = &data[i..i + gs];
        parts.push(format_group(group, cfg, use_color));
        i += gs;
    }
    parts.join(" ")
}

fn format_group(group: &[u8], cfg: &PrinterConfig, use_color: bool) -> String {
    match cfg.base {
        DisplayBase::Hexadecimal if cfg.group_size == 1 => group
            .iter()
            .map(|b| format_single_hex(*b, cfg, use_color))
            .collect::<Vec<_>>()
            .join(" "),
        DisplayBase::Hexadecimal => {
            let value = grouped_value(group, cfg.endianness);
            let hex_width = cfg.group_size * 2;
            let s = format!("{value:0hex_width$x}");
            if use_color {
                colorize_group_string(group, &s, cfg)
            } else {
                s
            }
        }
        DisplayBase::Binary => group
            .iter()
            .map(|b| {
                if use_color {
                    format!("{:08b}", b)
                } else {
                    format!("{:08b}", b)
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
        DisplayBase::Octal => group
            .iter()
            .map(|b| format!("{b:03o}"))
            .collect::<Vec<_>>()
            .join(" "),
        DisplayBase::Decimal => group
            .iter()
            .map(|b| format!("{b:03}"))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn format_single_hex(b: u8, cfg: &PrinterConfig, use_color: bool) -> String {
    let s = format!("{b:02x}");
    if use_color {
        let color = cfg.colors.color_for_byte(b, cfg.color_scheme);
        format!("{color}{s}{}", cfg.colors.reset)
    } else {
        s
    }
}

fn colorize_group_string(group: &[u8], s: &str, cfg: &PrinterConfig) -> String {
    if group.len() == 1 {
        return format_single_hex(group[0], cfg, true);
    }
    let color = cfg.colors.color_for_byte(group[0], cfg.color_scheme);
    format!("{color}{s}{}", cfg.colors.reset)
}

fn grouped_value(group: &[u8], endianness: Endianness) -> u64 {
    let mut value = 0u64;
    match endianness {
        Endianness::Big => {
            for &b in group {
                value = (value << 8) | b as u64;
            }
        }
        Endianness::Little => {
            for &b in group.iter().rev() {
                value = (value << 8) | b as u64;
            }
        }
    }
    value
}

fn format_char_section(data: &[u8], cfg: &PrinterConfig, colorize: bool) -> String {
    let mut out = String::new();
    let use_color = colorize && cfg.use_color;

    for panel in 0..cfg.panel_count {
        if panel > 0 {
            out.push(' ');
        }
        let start = panel * cfg.bytes_per_panel;
        let end = ((panel + 1) * cfg.bytes_per_panel).min(data.len());
        if start >= data.len() {
            break;
        }
        for &b in &data[start..end] {
            let ch = cfg.character_table.map(b);
            if use_color {
                let color = cfg.colors.color_for_byte(b, cfg.color_scheme);
                out.push_str(color);
                out.push(ch);
                out.push_str(&cfg.colors.reset);
            } else {
                out.push(ch);
            }
        }
        for _ in end - start..cfg.bytes_per_panel {
            out.push(' ');
        }
    }
    out
}

pub fn compute_panel_count(
    requested: PanelSetting,
    terminal_width: Option<u16>,
    bytes_per_panel: usize,
    show_position: bool,
    show_characters: bool,
) -> usize {
    match requested {
        PanelSetting::Fixed(n) => n.max(1),
        PanelSetting::Auto => {
            let width = terminal_width.unwrap_or(80) as usize;
            let overhead = if show_position { 10 } else { 1 };
            let char_width = if show_characters {
                bytes_per_panel + 1
            } else {
                0
            };
            let panel_width = bytes_per_panel * 3 + 3 + char_width;
            let max_panels = ((width.saturating_sub(overhead + 2)) / panel_width).max(1);
            max_panels.min(4)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PanelSetting {
    Fixed(usize),
    Auto,
}

pub fn default_panel_count(_terminal_width: Option<u16>) -> usize {
    2
}
