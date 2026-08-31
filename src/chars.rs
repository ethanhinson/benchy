#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterTable {
    Default,
    Ascii,
    Codepage1047,
    Codepage437,
    Braille,
}

impl CharacterTable {
    pub fn map(self, b: u8) -> char {
        match self {
            Self::Default => default_char(b),
            Self::Ascii => ascii_char(b),
            Self::Codepage1047 => codepage1047_char(b),
            Self::Codepage437 => codepage437_char(b),
            Self::Braille => braille_char(b),
        }
    }
}

fn default_char(b: u8) -> char {
    match b {
        0x00 => '⋄',
        0x20 => ' ',
        0x09 | 0x0a | 0x0c | 0x0d => '_',
        0x21..=0x7e => b as char,
        0x01..=0x7f => '•',
        _ => '×',
    }
}

fn ascii_char(b: u8) -> char {
    if (0x20..=0x7e).contains(&b) {
        b as char
    } else {
        '.'
    }
}

fn codepage1047_char(b: u8) -> char {
    if b == 0x00 {
        '⋄'
    } else if (0x20..=0x7e).contains(&b) {
        b as char
    } else {
        ebcdic1047_to_char(b).unwrap_or('.')
    }
}

fn ebcdic1047_to_char(b: u8) -> Option<char> {
    match b {
        0x40 => Some(' '),
        0x4a..=0x51 => Some(((b - 0x4a) + b'a') as char),
        0x59..=0x61 => Some(((b - 0x59) + b'j') as char),
        0x62..=0x69 => Some(((b - 0x62) + b'r') as char),
        0x6a..=0x71 => Some(((b - 0x6a) + b'z') as char),
        0x72..=0x79 => Some(((b - 0x72) + b'A') as char),
        0x81..=0x89 => Some(((b - 0x81) + b'I') as char),
        0x91..=0x99 => Some(((b - 0x91) + b'R') as char),
        0xa2..=0xa9 => Some(((b - 0xa2) + b'Z') as char),
        0xc1..=0xc9 => Some(((b - 0xc1) + b'A') as char),
        0xd1..=0xd9 => Some(((b - 0xd1) + b'J') as char),
        0xe2..=0xe9 => Some(((b - 0xe2) + b'R') as char),
        0xf0..=0xf9 => Some(((b - 0xf0) + b'0') as char),
        _ => None,
    }
}

fn codepage437_char(b: u8) -> char {
    if b == 0x00 {
        '⋄'
    } else if (0x20..=0x7e).contains(&b) {
        b as char
    } else {
        CP437[b as usize]
    }
}

fn braille_char(b: u8) -> char {
    match b {
        0x00 => '⋄',
        0x09 => '→',
        0x0a => '↵',
        0x0d => '←',
        0x20..=0x7e => b as char,
        _ => char::from_u32(0x2800 + b as u32).unwrap_or('×'),
    }
}

const CP437: [char; 256] = [
    '⋄', '☺', '☻', '♥', '♦', '♣', '♠', '•', '◘', '○', '◙', '♂', '♀', '♪', '♫', '☼', '►', '◄',
    '↕', '‼', '¶', '§', '▬', '↨', '↑', '↓', '→', '←', '∟', '↔', '▲', '▼', ' ', '!', '"', '#',
    '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/', '0', '1', '2', '3', '4', '5',
    '6', '7', '8', '9', ':', ';', '<', '=', '>', '?', '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G',
    'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y',
    'Z', '[', '\\', ']', '^', '_', '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k',
    'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}',
    '~', '⌂', 'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', 'á', 'í',
    'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', '░', '▒', '▓', '│',
    '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐', '└', '┴', '┬', '├', '─', '┼',
    '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧', '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫',
    '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀', 'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ',
    'Ω', 'δ', '∞', 'φ', 'ε', '∩', '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√',
    'ⁿ', '²', '■', 'ﬀ',
];
