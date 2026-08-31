mod chars;
mod colors;
mod parse_size;
mod printer;

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::PathBuf;
use std::process;

use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::{generate, Shell};
use terminal_size::{terminal_size, Width};

use chars::CharacterTable;
use colors::{print_color_table, ColorScheme, ColorWhen, Colors};
use parse_size::parse_size;
use printer::{
    compute_panel_count, default_panel_count, effective_panel_count, print_data, print_include,
    resolve_skip, BorderStyle, DisplayBase, Endianness, PanelSetting, PrinterConfig,
};

#[derive(Debug, Clone, ValueEnum)]
enum ColorWhenArg {
    Always,
    Auto,
    Never,
    Force,
}

#[derive(Debug, Clone, ValueEnum)]
enum BorderStyleArg {
    Unicode,
    Ascii,
    None,
}

#[derive(Debug, Clone, ValueEnum)]
enum CharacterTableArg {
    Default,
    Ascii,
    #[value(name = "codepage-1047")]
    Codepage1047,
    #[value(name = "codepage-437")]
    Codepage437,
    Braille,
}

#[derive(Debug, Clone, ValueEnum)]
enum ColorSchemeArg {
    Default,
    Gradient,
}

#[derive(Debug, Clone, ValueEnum)]
enum EndiannessArg {
    Little,
    Big,
}

#[derive(Debug, Clone, ValueEnum)]
enum BaseArg {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

#[derive(Parser)]
#[command(name = "hexyl", version = "0.16.0", about = "A command-line hex viewer")]
struct Args {
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    #[arg(short = 'n', long, alias = "bytes", value_name = "N")]
    length: Option<String>,

    #[arg(short = 'c')]
    bytes_alias: Option<String>,

    #[arg(short = 'l')]
    length_alias: Option<String>,

    #[arg(short = 's', long, allow_hyphen_values = true, value_name = "N")]
    skip: Option<String>,

    #[arg(long, default_value = "512", value_name = "SIZE")]
    block_size: String,

    #[arg(short = 'v', long = "no-squeezing")]
    no_squeezing: bool,

    #[arg(long, value_enum, default_value = "always")]
    color: ColorWhenArg,

    #[arg(long, value_enum, default_value = "unicode")]
    border: BorderStyleArg,

    #[arg(short = 'p', long = "plain")]
    plain: bool,

    #[arg(long = "no-characters")]
    no_characters: bool,

    #[arg(short = 'C', long = "characters")]
    characters: bool,

    #[arg(long, value_enum, default_value = "default")]
    character_table: CharacterTableArg,

    #[arg(long, value_enum, default_value = "default")]
    color_scheme: ColorSchemeArg,

    #[arg(short = 'P', long = "no-position")]
    no_position: bool,

    #[arg(short = 'o', long, default_value = "0", value_name = "N")]
    display_offset: String,

    #[arg(long, value_name = "N")]
    panels: Option<String>,

    #[arg(short = 'g', long = "group-size", alias = "groupsize", default_value = "1")]
    group_size: u8,

    #[arg(long, value_enum, default_value = "big")]
    endianness: EndiannessArg,

    #[arg(short = 'e')]
    little_endian: bool,

    #[arg(short = 'b', long, value_enum, default_value = "hexadecimal")]
    base: BaseArg,

    #[arg(long, value_name = "N")]
    terminal_width: Option<u16>,

    #[arg(long)]
    print_color_table: bool,

    #[arg(short = 'i', long = "include")]
    include: bool,

    #[arg(long, value_enum, value_name = "SHELL")]
    completion: Option<Shell>,
}

fn main() {
    let args = Args::parse();

    if let Some(shell) = args.completion {
        let mut cmd = Args::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut io::stdout());
        return;
    }

    if args.print_color_table {
        let colors = Colors::from_env();
        print_color_table(&colors);
        return;
    }

    if let Err(e) = run(args) {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn run(args: Args) -> Result<(), String> {
    let block_size = parse_size(&args.block_size, 512).map_err(|e| e.to_string())?;

    let length_str = args.length.or(args.bytes_alias).or(args.length_alias);
    let length = length_str
        .as_deref()
        .map(|s| parse_size(s, block_size))
        .transpose()
        .map_err(|e| e.to_string())?;

    let skip = args
        .skip
        .as_deref()
        .map(|s| parse_signed_size(s, block_size))
        .transpose()?
        .unwrap_or(0);

    let display_offset = parse_signed_size(&args.display_offset, block_size)?;

    let plain = args.plain;
    let show_characters = if plain {
        false
    } else if args.no_characters {
        false
    } else {
        true
    };
    let show_position = !plain && !args.no_position;

    let border = if plain {
        BorderStyle::None
    } else {
        match args.border {
            BorderStyleArg::Unicode => BorderStyle::Unicode,
            BorderStyleArg::Ascii => BorderStyle::Ascii,
            BorderStyleArg::None => BorderStyle::None,
        }
    };

    let color_when = if plain {
        ColorWhen::Never
    } else {
        match args.color {
            ColorWhenArg::Always => ColorWhen::Always,
            ColorWhenArg::Auto => ColorWhen::Auto,
            ColorWhenArg::Never => ColorWhen::Never,
            ColorWhenArg::Force => ColorWhen::Force,
        }
    };

    let stdout_is_tty = atty::is(atty::Stream::Stdout);
    let use_color = color_when.use_color(stdout_is_tty);

    let terminal_width = args
        .terminal_width
        .or_else(|| terminal_size().map(|(Width(w), _)| w));

    let panel_setting = match args.panels.as_deref() {
        Some("auto") => PanelSetting::Auto,
        Some(s) => PanelSetting::Fixed(
            s.parse::<usize>()
                .map_err(|_| format!("invalid panels value: {s}"))?,
        ),
        None => PanelSetting::Fixed(default_panel_count(terminal_width)),
    };

    let bytes_per_panel = BYTES_PER_GROUP;
    let panel_count = compute_panel_count(
        panel_setting,
        terminal_width,
        bytes_per_panel,
        show_position,
        show_characters,
    );

    let group_size = match args.group_size {
        1 | 2 | 4 | 8 => args.group_size as usize,
        other => return Err(format!("invalid group size: {other}")),
    };

    let endianness = if args.little_endian {
        Endianness::Little
    } else {
        match args.endianness {
            EndiannessArg::Little => Endianness::Little,
            EndiannessArg::Big => Endianness::Big,
        }
    };

    let colors = if use_color {
        Colors::from_env()
    } else {
        Colors::plain()
    };

    let mut cfg = PrinterConfig {
        show_position,
        show_characters,
        border,
        squeeze: !args.no_squeezing,
        colors,
        use_color,
        color_scheme: match args.color_scheme {
            ColorSchemeArg::Default => ColorScheme::Default,
            ColorSchemeArg::Gradient => ColorScheme::Gradient,
        },
        character_table: match args.character_table {
            CharacterTableArg::Default => CharacterTable::Default,
            CharacterTableArg::Ascii => CharacterTable::Ascii,
            CharacterTableArg::Codepage1047 => CharacterTable::Codepage1047,
            CharacterTableArg::Codepage437 => CharacterTable::Codepage437,
            CharacterTableArg::Braille => CharacterTable::Braille,
        },
        display_offset: display_offset.max(0) as u64,
        group_size,
        endianness,
        base: match args.base {
            BaseArg::Binary => DisplayBase::Binary,
            BaseArg::Octal => DisplayBase::Octal,
            BaseArg::Decimal => DisplayBase::Decimal,
            BaseArg::Hexadecimal => DisplayBase::Hexadecimal,
        },
        bytes_per_panel,
        panel_count,
    };

    cfg.panel_count = effective_panel_count(&cfg, terminal_width);

    let (data, name, data_start) = load_data(&args.file, skip, length)?;

    cfg.display_offset = cfg.display_offset.saturating_add(data_start);

    let mut stdout = io::stdout().lock();
    if args.include {
        print_include(&name, &data, &mut stdout).map_err(|e| e.to_string())?;
    } else {
        print_data(&data, &cfg, &mut stdout).map_err(|e| e.to_string())?;
    }

    Ok(())
}

const BYTES_PER_GROUP: usize = 8;

fn parse_signed_size(input: &str, block_size: u64) -> Result<i64, String> {
    let input = input.trim();
    if input.starts_with('-') {
        let mag = parse_size(&input[1..], block_size).map_err(|e| e.to_string())?;
        Ok(-(mag as i64))
    } else {
        Ok(parse_size(input, block_size).map_err(|e| e.to_string())? as i64)
    }
}

fn load_data(
    path: &Option<PathBuf>,
    skip: i64,
    length: Option<u64>,
) -> Result<(Vec<u8>, String, u64), String> {
    match path {
        Some(p) => {
            let file_len = fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            let skip_pos = resolve_skip(skip, file_len);
            let mut file = File::open(p).map_err(|e| e.to_string())?;
            if skip_pos > 0 {
                file.seek_relative(skip_pos as i64).map_err(|e| e.to_string())?;
            }
            let mut data = Vec::new();
            if let Some(len) = length {
                let mut limited = file.take(len);
                limited.read_to_end(&mut data).map_err(|e| e.to_string())?;
            } else {
                file.read_to_end(&mut data).map_err(|e| e.to_string())?;
            }
            let name = p
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("file")
                .to_string();
            Ok((data, name, skip_pos))
        }
        None => {
            let mut stdin = io::stdin().lock();
            let mut data = Vec::new();
            stdin.read_to_end(&mut data).map_err(|e| e.to_string())?;
            let skip_pos = resolve_skip(skip, data.len() as u64);
            let skip_usize = skip_pos as usize;
            let mut sliced = if skip_usize >= data.len() {
                Vec::new()
            } else {
                data[skip_usize..].to_vec()
            };
            if let Some(len) = length {
                if sliced.len() > len as usize {
                    sliced.truncate(len as usize);
                }
            }
            Ok((sliced, "stdin".to_string(), skip_pos))
        }
    }
}

mod atty {
    pub enum Stream {
        Stdout,
    }

    pub fn is(_: Stream) -> bool {
        terminal_size::terminal_size().is_some()
    }
}

trait SeekRelative {
    fn seek_relative(&mut self, offset: i64) -> io::Result<()>;
}

impl SeekRelative for File {
    fn seek_relative(&mut self, offset: i64) -> io::Result<()> {
        use std::io::{Seek, SeekFrom};
        let pos = self.stream_position()?;
        self.seek(SeekFrom::Start(pos + offset as u64))?;
        Ok(())
    }
}
