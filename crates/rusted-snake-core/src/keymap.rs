//! Keyboard mappings shared by the macro engine and onboard button editors.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl Modifiers {
    pub fn onboard_bits(self) -> u8 {
        (self.ctrl as u8)
            | ((self.shift as u8) << 1)
            | ((self.alt as u8) << 2)
            | ((self.super_key as u8) << 3)
    }

    pub fn label(self) -> String {
        let mut text = String::new();
        if self.ctrl {
            text.push_str("Ctrl+");
        }
        if self.shift {
            text.push_str("Shift+");
        }
        if self.alt {
            text.push_str("Alt+");
        }
        if self.super_key {
            text.push_str("Super+");
        }
        text
    }
}

/// A key captured in the interface, stored as a Linux input event code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapturedKey {
    pub code: u16,
    pub modifiers: Modifiers,
}

impl CapturedKey {
    pub fn label(self) -> String {
        format!("{}{}", self.modifiers.label(), evdev_name(self.code))
    }
}

/// Naga side-plate keys in plate order.
pub const SIDE_TRIGGERS: &[(u16, &str)] = &[
    (2, "Side button 1"),
    (3, "Side button 2"),
    (4, "Side button 3"),
    (5, "Side button 4"),
    (6, "Side button 5"),
    (7, "Side button 6"),
    (8, "Side button 7"),
    (9, "Side button 8"),
    (10, "Side button 9"),
    (11, "Side button 10"),
    (12, "Side button 11"),
    (13, "Side button 12"),
];

pub fn hid_usage_name(usage: u8) -> String {
    match usage {
        0x04..=0x1d => ((b'A' + (usage - 0x04)) as char).to_string(),
        0x1e..=0x27 => ((b'1' + (usage - 0x1e)) as char).to_string(),
        0x28 => "Enter".into(),
        0x29 => "Esc".into(),
        0x2a => "Backspace".into(),
        0x2b => "Tab".into(),
        0x2c => "Space".into(),
        0x2d => "-".into(),
        0x2e => "=".into(),
        0x2f => "[".into(),
        0x30 => "]".into(),
        0x31 => "\\".into(),
        0x32 => "#".into(),
        0x33 => ";".into(),
        0x34 => "'".into(),
        0x35 => "`".into(),
        0x36 => ",".into(),
        0x37 => ".".into(),
        0x38 => "/".into(),
        0x39 => "Caps Lock".into(),
        0x3a..=0x45 => format!("F{}", usage - 0x39),
        0x46 => "Print Screen".into(),
        0x47 => "Scroll Lock".into(),
        0x48 => "Pause".into(),
        0x49 => "Insert".into(),
        0x4a => "Home".into(),
        0x4b => "Page Up".into(),
        0x4c => "Delete".into(),
        0x4d => "End".into(),
        0x4e => "Page Down".into(),
        0x4f => "Right".into(),
        0x50 => "Left".into(),
        0x51 => "Down".into(),
        0x52 => "Up".into(),
        0x53 => "Num Lock".into(),
        0x54 => "Num /".into(),
        0x55 => "Num *".into(),
        0x56 => "Num -".into(),
        0x57 => "Num +".into(),
        0x58 => "Num Enter".into(),
        0x59..=0x62 => format!("Num {}", (usage - 0x58) % 10),
        0x63 => "Num .".into(),
        0x64 => "ISO \\".into(),
        0x65 => "Menu".into(),
        0x68..=0x73 => format!("F{}", usage - 0x68 + 13),
        0xe0 => "Left Ctrl".into(),
        0xe1 => "Left Shift".into(),
        0xe2 => "Left Alt".into(),
        0xe3 => "Left Super".into(),
        0xe4 => "Right Ctrl".into(),
        0xe5 => "Right Shift".into(),
        0xe6 => "Right Alt".into(),
        0xe7 => "Right Super".into(),
        _ => format!("Key {usage:02x}"),
    }
}

fn hid_base(character: char) -> Option<(u8, bool)> {
    let lower = character.to_ascii_lowercase();
    if lower.is_ascii_lowercase() {
        return Some((0x04 + (lower as u8 - b'a'), character.is_ascii_uppercase()));
    }
    let shifted = match character {
        '!' => Some(0x1e),
        '@' => Some(0x1f),
        '#' => Some(0x20),
        '$' => Some(0x21),
        '%' => Some(0x22),
        '^' => Some(0x23),
        '&' => Some(0x24),
        '*' => Some(0x25),
        '(' => Some(0x26),
        ')' => Some(0x27),
        '_' => Some(0x2d),
        '+' => Some(0x2e),
        '{' => Some(0x2f),
        '}' => Some(0x30),
        '|' => Some(0x31),
        ':' => Some(0x33),
        '"' => Some(0x34),
        '~' => Some(0x35),
        '<' => Some(0x36),
        '>' => Some(0x37),
        '?' => Some(0x38),
        _ => None,
    };
    if let Some(usage) = shifted {
        return Some((usage, true));
    }
    let usage = match character {
        '1'..='9' => 0x1e + (character as u8 - b'1'),
        '0' => 0x27,
        '-' => 0x2d,
        '=' => 0x2e,
        '[' => 0x2f,
        ']' => 0x30,
        '\\' => 0x31,
        ';' => 0x33,
        '\'' => 0x34,
        '`' => 0x35,
        ',' => 0x36,
        '.' => 0x37,
        '/' => 0x38,
        _ => return None,
    };
    Some((usage, false))
}

/// Convert a GPUI key name plus modifiers to the onboard HID action format.
pub fn key_to_hid(key: &str, modifiers: Modifiers) -> Option<(u8, u8)> {
    let mut modifiers = modifiers;
    let usage = match key {
        "enter" => 0x28,
        "escape" => 0x29,
        "backspace" => 0x2a,
        "tab" => 0x2b,
        "space" => 0x2c,
        "capslock" => 0x39,
        "delete" => 0x4c,
        "insert" => 0x49,
        "home" => 0x4a,
        "end" => 0x4d,
        "pageup" => 0x4b,
        "pagedown" => 0x4e,
        "left" => 0x50,
        "right" => 0x4f,
        "up" => 0x52,
        "down" => 0x51,
        "numlock" => 0x53,
        "scrolllock" => 0x47,
        "pause" => 0x48,
        "menu" => 0x65,
        "print" | "printscreen" => 0x46,
        _ => {
            if let Some(rest) = key.strip_prefix('f')
                && let Ok(number) = rest.parse::<u8>()
                && (1..=24).contains(&number)
            {
                if number <= 12 {
                    0x3a + number - 1
                } else {
                    0x68 + number - 13
                }
            } else {
                let mut chars = key.chars();
                let character = chars.next()?;
                if chars.next().is_some() {
                    return None;
                }
                let (usage, shifted) = hid_base(character)?;
                if shifted {
                    modifiers.shift = true;
                }
                usage
            }
        }
    };
    Some((modifiers.onboard_bits(), usage))
}

/// Convert a GPUI key name to a Linux input event code.
pub fn key_to_evdev(key: &str) -> Option<u16> {
    let code = match key {
        "escape" => 1,
        "enter" => 28,
        "backspace" => 14,
        "tab" => 15,
        "space" => 57,
        "capslock" => 58,
        "leftshift" => 42,
        "rightshift" => 54,
        "leftctrl" | "control" => 29,
        "rightctrl" => 97,
        "leftalt" => 56,
        "rightalt" => 100,
        "leftsuper" | "super" | "meta" => 125,
        "leftmeta" => 125,
        "rightmeta" => 126,
        "menu" => 139,
        "delete" => 111,
        "insert" => 110,
        "home" => 102,
        "end" => 107,
        "pageup" => 104,
        "pagedown" => 109,
        "left" => 105,
        "right" => 106,
        "up" => 103,
        "down" => 108,
        "numlock" => 69,
        "scrolllock" => 70,
        "pause" => 119,
        "print" | "printscreen" => 99,
        "minus" => 12,
        "equal" => 13,
        _ => {
            if let Some(rest) = key.strip_prefix('f')
                && let Ok(number) = rest.parse::<u16>()
            {
                match number {
                    1..=10 => 59 + number - 1,
                    11 => 87,
                    12 => 88,
                    _ => return None,
                }
            } else {
                let mut chars = key.chars();
                let character = chars.next()?;
                if chars.next().is_some() {
                    return None;
                }
                let mut shifted = character.is_ascii_uppercase();
                let lower = character.to_ascii_lowercase();
                let base = if lower.is_ascii_lowercase() {
                    30u16 + (lower as u8 - b'a') as u16
                } else if lower.is_ascii_digit() {
                    if lower == '0' {
                        11u16
                    } else {
                        2u16 + (lower as u8 - b'1') as u16
                    }
                } else {
                    let (mapped, is_shifted) = match character {
                        '!' => (2, true),
                        '@' => (3, true),
                        '#' => (4, true),
                        '$' => (5, true),
                        '%' => (6, true),
                        '^' => (7, true),
                        '&' => (8, true),
                        '*' => (9, true),
                        '(' => (10, true),
                        ')' => (11, true),
                        '_' => (12, true),
                        '+' => (13, true),
                        '{' => (26, true),
                        '}' => (27, true),
                        '|' => (43, true),
                        ':' => (39, true),
                        '"' => (40, true),
                        '~' => (41, true),
                        '<' => (51, true),
                        '>' => (52, true),
                        '?' => (53, true),
                        '-' => (12, false),
                        '=' => (13, false),
                        '[' => (26, false),
                        ']' => (27, false),
                        '\\' => (43, false),
                        ';' => (39, false),
                        '\'' => (40, false),
                        '`' => (41, false),
                        ',' => (51, false),
                        '.' => (52, false),
                        '/' => (53, false),
                        _ => return None,
                    };
                    shifted |= is_shifted;
                    mapped
                };
                let _ = shifted;
                base
            }
        }
    };
    Some(code)
}

pub fn evdev_name(code: u16) -> String {
    match code {
        1 => "Esc".into(),
        2..=10 => format!("{}", code - 1),
        11 => "0".into(),
        12 => "-".into(),
        13 => "=".into(),
        14 => "Backspace".into(),
        15 => "Tab".into(),
        16..=25 => ((b'Q' + (code - 16) as u8) as char).to_string(),
        26 => "[".into(),
        27 => "]".into(),
        28 => "Enter".into(),
        29 => "Left Ctrl".into(),
        30..=38 => ((b'A' + (code - 30) as u8) as char).to_string(),
        39 => ";".into(),
        40 => "'".into(),
        41 => "`".into(),
        42 => "Left Shift".into(),
        43 => "\\".into(),
        44..=50 => ((b'Z' + (code - 44) as u8) as char).to_string(),
        51 => ",".into(),
        52 => ".".into(),
        53 => "/".into(),
        54 => "Right Shift".into(),
        56 => "Left Alt".into(),
        57 => "Space".into(),
        58 => "Caps Lock".into(),
        59..=68 => format!("F{}", code - 58),
        69 => "Num Lock".into(),
        70 => "Scroll Lock".into(),
        71..=82 => format!("Num {}", (code - 70) % 10),
        83 => "Num .".into(),
        87 => "F11".into(),
        88 => "F12".into(),
        96 => "Num Enter".into(),
        97 => "Right Ctrl".into(),
        98 => "Num /".into(),
        99 => "Print Screen".into(),
        100 => "Right Alt".into(),
        102 => "Home".into(),
        103 => "Up".into(),
        104 => "Page Up".into(),
        105 => "Left".into(),
        106 => "Right".into(),
        107 => "End".into(),
        108 => "Down".into(),
        109 => "Page Down".into(),
        110 => "Insert".into(),
        111 => "Delete".into(),
        113 => "Mute".into(),
        114 => "Volume down".into(),
        115 => "Volume up".into(),
        116 => "Power".into(),
        119 => "Pause".into(),
        125 => "Left Super".into(),
        126 => "Right Super".into(),
        127 => "Menu".into(),
        128 => "Stop".into(),
        139 => "Menu".into(),
        142 => "Sleep".into(),
        158 => "Back".into(),
        159 => "Forward".into(),
        163 => "Next track".into(),
        164 => "Play/Pause".into(),
        165 => "Previous track".into(),
        166 => "Stop".into(),
        172 => "Home page".into(),
        173 => "Refresh".into(),
        217 => "Search".into(),
        224 => "Brightness down".into(),
        225 => "Brightness up".into(),
        226 => "Media".into(),
        272 => "Left click".into(),
        273 => "Right click".into(),
        274 => "Middle click".into(),
        275 => "Side click".into(),
        276 => "Extra click".into(),
        277 => "Forward click".into(),
        278 => "Back click".into(),
        279 => "Task click".into(),
        _ => format!("Key {code}"),
    }
}

/// ASCII character to a Linux key code and whether Shift is needed.
pub fn char_to_evdev(character: char) -> Option<(u16, bool)> {
    let lower = character.to_ascii_lowercase();
    if character.is_ascii_uppercase() {
        return Some((30u16 + (lower as u8 - b'a') as u16, true));
    }
    if lower.is_ascii_lowercase() {
        return Some((30u16 + (lower as u8 - b'a') as u16, false));
    }
    if lower.is_ascii_digit() {
        return Some((
            if lower == '0' {
                11u16
            } else {
                2u16 + (lower as u8 - b'1') as u16
            },
            false,
        ));
    }
    let pair = match character {
        ' ' => (57, false),
        '\n' => (28, false),
        '\t' => (15, false),
        '!' => (2, true),
        '@' => (3, true),
        '#' => (4, true),
        '$' => (5, true),
        '%' => (6, true),
        '^' => (7, true),
        '&' => (8, true),
        '*' => (9, true),
        '(' => (10, true),
        ')' => (11, true),
        '-' => (12, false),
        '_' => (12, true),
        '=' => (13, false),
        '+' => (13, true),
        '[' => (26, false),
        '{' => (26, true),
        ']' => (27, false),
        '}' => (27, true),
        '\\' => (43, false),
        '|' => (43, true),
        ';' => (39, false),
        ':' => (39, true),
        '\'' => (40, false),
        '"' => (40, true),
        '`' => (41, false),
        '~' => (41, true),
        ',' => (51, false),
        '<' => (51, true),
        '.' => (52, false),
        '>' => (52, true),
        '/' => (53, false),
        '?' => (53, true),
        _ => return None,
    };
    Some(pair)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hid_letters_and_shifted_punctuation() {
        assert_eq!(key_to_hid("a", Modifiers::default()), Some((0, 0x04)));
        assert_eq!(key_to_hid("!", Modifiers::default()), Some((2, 0x1e)));
        assert_eq!(key_to_hid("A", Modifiers::default()), Some((2, 0x04)));
        assert_eq!(key_to_hid("enter", Modifiers::default()), Some((0, 0x28)));
    }

    #[test]
    fn evdev_known_keys() {
        assert_eq!(key_to_evdev("a"), Some(30));
        assert_eq!(key_to_evdev("1"), Some(2));
        assert_eq!(key_to_evdev("0"), Some(11));
        assert_eq!(key_to_evdev("!"), Some(2));
        assert_eq!(key_to_evdev("f12"), Some(88));
        assert_eq!(char_to_evdev('?'), Some((53, true)));
        assert_eq!(char_to_evdev('A'), Some((30, true)));
    }
}
