#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterTable {
    Default,
    Ascii,
    Codepage1047,
    Codepage437,
    Braille,
}

impl CharacterTable {
    pub fn map_byte(self, byte: u8) -> char {
        match self {
            CharacterTable::Default => map_default(byte),
            CharacterTable::Ascii => map_ascii(byte),
            CharacterTable::Codepage1047 => map_codepage1047(byte),
            CharacterTable::Codepage437 => map_codepage437(byte),
            CharacterTable::Braille => map_braille(byte),
        }
    }
}

fn is_printable_ascii(b: u8) -> bool {
    (0x21..=0x7e).contains(&b)
}

fn is_ascii_whitespace(b: u8) -> bool {
    matches!(b, b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c')
}

fn map_default(byte: u8) -> char {
    if byte == 0 {
        '\u{22c4}'
    } else if byte == b' ' {
        ' '
    } else if is_printable_ascii(byte) {
        byte as char
    } else if is_ascii_whitespace(byte) {
        '_'
    } else if byte < 0x80 {
        '\u{2022}'
    } else {
        '\u{00d7}'
    }
}

fn map_ascii(byte: u8) -> char {
    if byte == b' ' {
        ' '
    } else if is_printable_ascii(byte) {
        byte as char
    } else {
        '.'
    }
}

fn map_codepage1047(byte: u8) -> char {
    // EBCDIC code page 1047 printable subset approximation for cleanroom rebuild.
    if byte == 0x40 {
        ' '
    } else if (0x41..=0x49).contains(&byte)
        || (0x4a..=0x51).contains(&byte)
        || (0x52..=0x59).contains(&byte)
        || (0x62..=0x69).contains(&byte)
        || (0x6a..=0x71).contains(&byte)
        || (0x72..=0x79).contains(&byte)
        || (0x81..=0x89).contains(&byte)
        || (0x8a..=0x91).contains(&byte)
        || (0x92..=0x99).contains(&byte)
        || (0xa2..=0xa9).contains(&byte)
        || (0xaa..=0xb1).contains(&byte)
        || (0xb2..=0xb9).contains(&byte)
        || (0xc1..=0xc9).contains(&byte)
        || (0xca..=0xd1).contains(&byte)
        || (0xd2..=0xd9).contains(&byte)
        || (0xe2..=0xe9).contains(&byte)
        || (0xea..=0xf1).contains(&byte)
        || (0xf2..=0xf9).contains(&byte)
        || byte == 0xfa
    {
        ebcdic1047_to_ascii(byte)
    } else {
        '.'
    }
}

fn ebcdic1047_to_ascii(b: u8) -> char {
    const TABLE: [u8; 256] = {
        let mut t = [b'.'; 256];
        let pairs: &[(u8, u8)] = &[
            (0x40, b' '), (0x4a, b'['), (0x4b, b'.'), (0x4c, b'<'), (0x4d, b'('),
            (0x4e, b'+'), (0x4f, b'!'), (0x50, b'&'), (0x51, b')'), (0x52, b';'),
            (0x53, b'-'), (0x54, b'/'), (0x55, b'%'), (0x56, b'_'), (0x57, b'>'),
            (0x58, b'?'), (0x59, b'#'), (0x5a, b'@'), (0x5b, b'\''), (0x5c, b'"'),
            (0x5d, b':'), (0x5e, b'$'), (0x5f, b'a'), (0x60, b'b'), (0x61, b'c'),
            (0x62, b'd'), (0x63, b'e'), (0x64, b'f'), (0x65, b'g'), (0x66, b'h'),
            (0x67, b'i'), (0x68, b'j'), (0x69, b'k'), (0x6a, b'l'), (0x6b, b'm'),
            (0x6c, b'n'), (0x6d, b'o'), (0x6e, b'p'), (0x6f, b'q'), (0x70, b'r'),
            (0x71, b's'), (0x72, b't'), (0x73, b'u'), (0x74, b'v'), (0x75, b'w'),
            (0x76, b'x'), (0x77, b'y'), (0x78, b'z'), (0x81, b'A'), (0x82, b'B'),
            (0x83, b'C'), (0x84, b'D'), (0x85, b'E'), (0x86, b'F'), (0x87, b'G'),
            (0x88, b'H'), (0x89, b'I'), (0x8a, b'J'), (0x8b, b'K'), (0x8c, b'L'),
            (0x8d, b'M'), (0x8e, b'N'), (0x8f, b'O'), (0x90, b'P'), (0x91, b'Q'),
            (0x92, b'R'), (0x93, b'S'), (0x94, b'T'), (0x95, b'U'), (0x96, b'V'),
            (0x97, b'W'), (0x98, b'X'), (0x99, b'Y'), (0x9a, b'Z'), (0xa1, b'~'),
            (0xa2, b'0'), (0xa3, b'1'), (0xa4, b'2'), (0xa5, b'3'), (0xa6, b'4'),
            (0xa7, b'5'), (0xa8, b'6'), (0xa9, b'7'), (0xaa, b'8'), (0xab, b'9'),
            (0xac, b'|'), (0xad, b','), (0xae, b'{'), (0xaf, b'}'), (0xb0, b'`'),
            (0xc0, b'}'), (0xc1, b'J'), (0xc2, b'A'), (0xc3, b'B'), (0xc4, b'C'),
            (0xc5, b'D'), (0xc6, b'E'), (0xc7, b'F'), (0xc8, b'G'), (0xc9, b'H'),
            (0xca, b'I'), (0xd0, b'}'), (0xd1, b'K'), (0xd2, b'L'), (0xd3, b'M'),
            (0xd4, b'N'), (0xd5, b'O'), (0xd6, b'P'), (0xd7, b'Q'), (0xd8, b'R'),
            (0xd9, b'S'), (0xe0, b'\\'), (0xe2, b'S'), (0xe3, b'T'), (0xe4, b'U'),
            (0xe5, b'V'), (0xe6, b'W'), (0xe7, b'X'), (0xe8, b'Y'), (0xe9, b'Z'),
            (0xf0, b'0'), (0xf1, b'1'), (0xf2, b'2'), (0xf3, b'3'), (0xf4, b'4'),
            (0xf5, b'5'), (0xf6, b'6'), (0xf7, b'7'), (0xf8, b'8'), (0xf9, b'9'),
            (0xfa, b'|'),
        ];
        let mut i = 0;
        while i < pairs.len() {
            t[pairs[i].0 as usize] = pairs[i].1;
            i += 1;
        }
        t
    };
    TABLE[b as usize] as char
}

fn map_codepage437(byte: u8) -> char {
    if byte == b' ' {
        ' '
    } else if is_printable_ascii(byte) {
        byte as char
    } else {
        CP437[byte as usize]
    }
}

const CP437: [char; 256] = [
    '\u{0000}', '\u{263A}', '\u{263B}', '\u{2665}', '\u{2666}', '\u{2663}', '\u{2660}', '\u{2022}',
    '\u{25D8}', '\u{25CB}', '\u{25D9}', '\u{2642}', '\u{2640}', '\u{266A}', '\u{266B}', '\u{263C}',
    '\u{25BA}', '\u{25C4}', '\u{2195}', '\u{203C}', '\u{00B6}', '\u{00A7}', '\u{25AC}', '\u{21A8}',
    '\u{2191}', '\u{2193}', '\u{2192}', '\u{2190}', '\u{221F}', '\u{2194}', '\u{25B2}', '\u{25BC}',
    ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
    '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
    'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '^', '_',
    '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
    'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~', '\u{2302}',
    '\u{00C7}', '\u{00FC}', '\u{00E9}', '\u{00E2}', '\u{00E4}', '\u{00E0}', '\u{00E5}', '\u{00E7}',
    '\u{00EA}', '\u{00EB}', '\u{00E8}', '\u{00EF}', '\u{00EE}', '\u{00EC}', '\u{00C4}', '\u{00C5}',
    '\u{00C9}', '\u{00E6}', '\u{00C6}', '\u{00F4}', '\u{00F6}', '\u{00F2}', '\u{00FB}', '\u{00F9}',
    '\u{00FF}', '\u{00D6}', '\u{00DC}', '\u{00A2}', '\u{00A3}', '\u{00A5}', '\u{20A7}', '\u{0192}',
    '\u{00E1}', '\u{00ED}', '\u{00F3}', '\u{00FA}', '\u{00F1}', '\u{00D1}', '\u{00AA}', '\u{00BA}',
    '\u{00BF}', '\u{2310}', '\u{00AC}', '\u{00BD}', '\u{00BC}', '\u{00A1}', '\u{00AB}', '\u{00BB}',
    '\u{2591}', '\u{2592}', '\u{2593}', '\u{2502}', '\u{2524}', '\u{2561}', '\u{2562}', '\u{2556}',
    '\u{2555}', '\u{2563}', '\u{2551}', '\u{2557}', '\u{255D}', '\u{255C}', '\u{255B}', '\u{2510}',
    '\u{2514}', '\u{2534}', '\u{252C}', '\u{251C}', '\u{2500}', '\u{253C}', '\u{255E}', '\u{255F}',
    '\u{255A}', '\u{2554}', '\u{2569}', '\u{2566}', '\u{2560}', '\u{2550}', '\u{256C}', '\u{2567}',
    '\u{2568}', '\u{2564}', '\u{2565}', '\u{2559}', '\u{2558}', '\u{2552}', '\u{2553}', '\u{256B}',
    '\u{256A}', '\u{2518}', '\u{250C}', '\u{2588}', '\u{2584}', '\u{258C}', '\u{2590}', '\u{2580}',
    '\u{03B1}', '\u{00DF}', '\u{0393}', '\u{03C0}', '\u{03A3}', '\u{03C3}', '\u{00B5}', '\u{03C4}',
    '\u{03A6}', '\u{0398}', '\u{03A9}', '\u{03B4}', '\u{221E}', '\u{03C6}', '\u{03B5}', '\u{2229}',
    '\u{2261}', '\u{00B1}', '\u{2265}', '\u{2264}', '\u{2320}', '\u{2321}', '\u{00F7}', '\u{2248}',
    '\u{00B0}', '\u{2219}', '\u{00B7}', '\u{221A}', '\u{207F}', '\u{00B2}', '\u{25A0}', '\u{00A0}',
];

fn map_braille(byte: u8) -> char {
    if byte == b' ' {
        ' '
    } else if is_printable_ascii(byte) {
        byte as char
    } else {
        char::from_u32(0x2800 + (byte as u32)).unwrap_or('.')
    }
}
