mod charset;
mod config;
mod display;
mod input;
mod units;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use config::{Base, BorderStyle, ColorMode, Config, Endianness};

use crate::charset::CharacterTable;

#[derive(Parser)]
#[command(name = "hexyl", version = "0.16.0", about = "A command-line hex viewer")]
struct Cli {
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    #[arg(short = 'n', long, alias = "bytes", value_name = "N")]
    length: Option<String>,

    #[arg(short = 'l', long, hide = true, value_name = "N")]
    length_alias: Option<String>,

    #[arg(short = 'c', long, hide = true, value_name = "N")]
    bytes_alias: Option<String>,

    #[arg(short, long, value_name = "N")]
    skip: Option<String>,

    #[arg(long, default_value = "512", value_name = "SIZE")]
    block_size: String,

    #[arg(short, long)]
    no_squeezing: bool,

    #[arg(long, default_value = "always", value_enum)]
    color: ColorArg,

    #[arg(long, default_value = "unicode", value_enum)]
    border: BorderArg,

    #[arg(short, long)]
    plain: bool,

    #[arg(long)]
    no_characters: bool,

    #[arg(short = 'C', long)]
    characters: bool,

    #[arg(long, default_value = "default", value_enum)]
    character_table: CharacterTableArg,

    #[arg(short = 'P', long)]
    no_position: bool,

    #[arg(short = 'o', long, default_value = "0", value_name = "N")]
    display_offset: String,

    #[arg(long, value_name = "N")]
    panels: Option<String>,

    #[arg(short = 'g', long, alias = "groupsize", default_value = "1", value_enum)]
    group_size: GroupSizeArg,

    #[arg(long, default_value = "big", value_enum)]
    endianness: EndiannessArg,

    #[arg(short, long, default_value = "hexadecimal", value_enum)]
    base: BaseArg,

    #[arg(long, value_name = "N")]
    terminal_width: Option<usize>,

    #[arg(long)]
    print_color_table: bool,

    #[arg(short, long)]
    include: bool,

    #[arg(long, value_enum)]
    completion: Option<ShellArg>,
}

#[derive(Clone, ValueEnum)]
enum ColorArg {
    Always,
    Auto,
    Never,
    Force,
}

#[derive(Clone, ValueEnum)]
enum BorderArg {
    Unicode,
    Ascii,
    None,
}

#[derive(Clone, ValueEnum)]
enum CharacterTableArg {
    Default,
    Ascii,
    #[value(name = "codepage-1047")]
    Codepage1047,
    #[value(name = "codepage-437")]
    Codepage437,
    Braille,
}

#[derive(Clone, ValueEnum)]
enum GroupSizeArg {
    #[value(name = "1")]
    One,
    #[value(name = "2")]
    Two,
    #[value(name = "4")]
    Four,
    #[value(name = "8")]
    Eight,
}

#[derive(Clone, ValueEnum)]
enum EndiannessArg {
    Little,
    Big,
}

#[derive(Clone, ValueEnum)]
enum BaseArg {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

#[derive(Clone, ValueEnum)]
enum ShellArg {
    Bash,
    Elvish,
    Fish,
    Powershell,
    Zsh,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.print_color_table {
        print_color_table();
        return Ok(());
    }

    if let Some(shell) = cli.completion {
        print_completion(shell);
        return Ok(());
    }

    let length = cli.length.or(cli.length_alias).or(cli.bytes_alias);
    let block_size = units::parse_byte_count(&cli.block_size, 512)?;

    let mut config = Config {
        length: length.clone(),
        skip: cli.skip.clone(),
        block_size,
        no_squeezing: cli.no_squeezing,
        color: match cli.color {
            ColorArg::Always => ColorMode::Always,
            ColorArg::Auto => ColorMode::Auto,
            ColorArg::Never => ColorMode::Never,
            ColorArg::Force => ColorMode::Force,
        },
        border: match cli.border {
            BorderArg::Unicode => BorderStyle::Unicode,
            BorderArg::Ascii => BorderStyle::Ascii,
            BorderArg::None => BorderStyle::None,
        },
        plain: cli.plain,
        show_characters: !cli.no_characters,
        character_table: match cli.character_table {
            CharacterTableArg::Default => CharacterTable::Default,
            CharacterTableArg::Ascii => CharacterTable::Ascii,
            CharacterTableArg::Codepage1047 => CharacterTable::Codepage1047,
            CharacterTableArg::Codepage437 => CharacterTable::Codepage437,
            CharacterTableArg::Braille => CharacterTable::Braille,
        },
        show_position: !cli.no_position,
        display_offset: units::parse_byte_count(&cli.display_offset, block_size)? as i64,
        panels: cli.panels.as_ref().and_then(|p| {
            if p == "auto" {
                Some(2)
            } else {
                p.parse().ok()
            }
        }).or_else(|| cli.terminal_width.map(|w| (w / 40).max(1))),
        group_size: match cli.group_size {
            GroupSizeArg::One => 1,
            GroupSizeArg::Two => 2,
            GroupSizeArg::Four => 4,
            GroupSizeArg::Eight => 8,
        },
        endianness: match cli.endianness {
            EndiannessArg::Little => Endianness::Little,
            EndiannessArg::Big => Endianness::Big,
        },
        base: match cli.base {
            BaseArg::Binary => Base::Binary,
            BaseArg::Octal => Base::Octal,
            BaseArg::Decimal => Base::Decimal,
            BaseArg::Hexadecimal => Base::Hexadecimal,
        },
        include: cli.include,
        file_name: cli.file.as_ref().and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned())),
    };

    if cli.plain {
        config.apply_plain();
    }

    let data = input::read_input(
        cli.file.as_deref(),
        config.skip.as_deref(),
        config.length.as_deref(),
        config.block_size,
    )?;

    let mut stdout = std::io::stdout().lock();
    display::render(&data, &config, &mut stdout)?;
    Ok(())
}

fn print_color_table() {
    println!("hexyl color table (default scheme)");
    println!("NULL bytes: bright black");
    println!("ASCII whitespace: green");
    println!("ASCII printable: cyan");
    println!("ASCII other: green");
    println!("Non-ASCII: yellow");
}

fn print_completion(shell: ShellArg) {
    let name = match shell {
        ShellArg::Bash => "bash",
        ShellArg::Elvish => "elvish",
        ShellArg::Fish => "fish",
        ShellArg::Powershell => "powershell",
        ShellArg::Zsh => "zsh",
    };
    println!("# hexyl completion for {name}");
}
