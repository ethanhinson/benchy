use crate::charset::CharacterTable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    Unicode,
    Ascii,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Always,
    Auto,
    Never,
    Force,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub length: Option<String>,
    pub skip: Option<String>,
    pub block_size: u64,
    pub no_squeezing: bool,
    pub color: ColorMode,
    pub border: BorderStyle,
    pub plain: bool,
    pub show_characters: bool,
    pub character_table: CharacterTable,
    pub show_position: bool,
    pub display_offset: i64,
    pub panels: Option<usize>,
    pub group_size: u8,
    pub endianness: Endianness,
    pub base: Base,
    pub include: bool,
    pub file_name: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            length: None,
            skip: None,
            block_size: 512,
            no_squeezing: false,
            color: ColorMode::Always,
            border: BorderStyle::Unicode,
            plain: false,
            show_characters: true,
            character_table: CharacterTable::Default,
            show_position: true,
            display_offset: 0,
            panels: None,
            group_size: 1,
            endianness: Endianness::Big,
            base: Base::Hexadecimal,
            include: false,
            file_name: None,
        }
    }
}

impl Config {
    pub fn apply_plain(&mut self) {
        self.plain = true;
        self.show_characters = false;
        self.show_position = false;
        self.border = BorderStyle::None;
        self.color = ColorMode::Never;
    }
}
