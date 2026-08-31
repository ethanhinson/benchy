use clap::{Parser, ValueEnum};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum ColorWhen {
    Always,
    Auto,
    Never,
    Force,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum BorderStyle {
    Unicode,
    Ascii,
    None,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum CharacterTable {
    Default,
    Ascii,
    Codepage1047,
    Codepage437,
    Braille,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum ColorScheme {
    Default,
    Gradient,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum Endianness {
    Little,
    Big,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum NumericBase {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

#[derive(Parser, Debug)]
#[command(
    name = "hexyl",
    version,
    about = "A command-line hex viewer",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,

    #[arg(short = 'n', long, alias = "bytes", value_name = "N")]
    pub length: Option<String>,

    #[arg(short = 'c', long = "bytes", hide = true, value_name = "N")]
    pub length_alias_c: Option<String>,

    #[arg(short = 'l', hide = true, value_name = "N")]
    pub length_alias_l: Option<String>,

    #[arg(short, long, value_name = "N")]
    pub skip: Option<String>,

    #[arg(long, default_value = "512", value_name = "SIZE")]
    pub block_size: u64,

    #[arg(short, long)]
    pub no_squeezing: bool,

    #[arg(long, value_enum, default_value = "always")]
    pub color: ColorWhen,

    #[arg(long, value_enum, default_value = "unicode")]
    pub border: BorderStyle,

    #[arg(short, long)]
    pub plain: bool,

    #[arg(long)]
    pub no_characters: bool,

    #[arg(short = 'C', long)]
    pub characters: bool,

    #[arg(long, value_enum, default_value = "default")]
    pub character_table: CharacterTable,

    #[arg(long, value_enum, default_value = "default")]
    pub color_scheme: ColorScheme,

    #[arg(short = 'P', long)]
    pub no_position: bool,

    #[arg(short, long, default_value = "0", value_name = "N")]
    pub display_offset: Option<String>,

    #[arg(long, default_value = "auto", value_name = "N")]
    pub panels: String,

    #[arg(short = 'g', long, alias = "groupsize", value_enum, default_value = "1")]
    pub group_size: GroupSizeArg,

    #[arg(long, value_enum, default_value = "big")]
    pub endianness: Endianness,

    #[arg(short = 'b', long, value_enum, default_value = "hexadecimal")]
    pub base: NumericBase,

    #[arg(long, value_name = "N")]
    pub terminal_width: Option<usize>,

    #[arg(long)]
    pub print_color_table: bool,

    #[arg(short, long)]
    pub include: bool,

    #[arg(long, value_enum, value_name = "SHELL")]
    pub completion: Option<Shell>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum GroupSizeArg {
    #[value(name = "1")]
    One,
    #[value(name = "2")]
    Two,
    #[value(name = "4")]
    Four,
    #[value(name = "8")]
    Eight,
}

impl GroupSizeArg {
    pub fn as_usize(self) -> usize {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Four => 4,
            Self::Eight => 8,
        }
    }
}

impl Cli {
    pub fn effective_length(&self) -> Option<&str> {
        self.length
            .as_deref()
            .or(self.length_alias_c.as_deref())
            .or(self.length_alias_l.as_deref())
    }

    pub fn resolved(&self) -> Self {
        let mut cli = self.clone();
        if cli.plain {
            cli.no_characters = true;
            cli.no_position = true;
            cli.border = BorderStyle::None;
            cli.color = ColorWhen::Never;
        }
        if std::env::var_os("NO_COLOR").is_some()
            && !matches!(cli.color, ColorWhen::Force | ColorWhen::Always)
        {
            if matches!(cli.color, ColorWhen::Auto | ColorWhen::Always) {
                // force overrides NO_COLOR
            }
        }
        if matches!(cli.color, ColorWhen::Force) {
            cli.color = ColorWhen::Always;
        }
        cli
    }
}

impl Clone for Cli {
    fn clone(&self) -> Self {
        Self {
            file: self.file.clone(),
            length: self.length.clone(),
            length_alias_c: self.length_alias_c.clone(),
            length_alias_l: self.length_alias_l.clone(),
            skip: self.skip.clone(),
            block_size: self.block_size,
            no_squeezing: self.no_squeezing,
            color: self.color,
            border: self.border,
            plain: self.plain,
            no_characters: self.no_characters,
            characters: self.characters,
            character_table: self.character_table,
            color_scheme: self.color_scheme,
            no_position: self.no_position,
            display_offset: self.display_offset.clone(),
            panels: self.panels.clone(),
            group_size: self.group_size,
            endianness: self.endianness,
            base: self.base,
            terminal_width: self.terminal_width,
            print_color_table: self.print_color_table,
            include: self.include,
            completion: self.completion,
        }
    }
}
