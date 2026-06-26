use bevy::prelude::*;

#[derive(Clone, Component, Debug)]
pub(crate) struct CheatCode {
    pub(crate) description: String,
    pub(crate) code: String,
    pub(crate) code_type: CheatCodeType,
    pub(crate) address: u16,
    pub(crate) value: u8,
    pub(crate) compare: Option<u8>,
    pub(crate) enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CheatCodeType {
    GameGenie,
    GameShark,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CheatTable {
    pub(crate) codes: Vec<CheatCode>,
}

impl CheatTable {
    pub(crate) fn is_empty(&self) -> bool {
        self.codes.is_empty()
    }

    pub(crate) fn read_patch(&self, address: u16, current_value: u8) -> Option<u8> {
        for code in &self.codes {
            if !code.enabled {
                continue;
            }
            if code.address != address {
                continue;
            }
            if let Some(compare) = code.compare {
                if current_value != compare {
                    continue;
                }
            }
            return Some(code.value);
        }
        None
    }

    pub(crate) fn toggle(&mut self, index: usize) {
        if let Some(code) = self.codes.get_mut(index) {
            code.enabled = !code.enabled;
        }
    }

    pub(crate) fn remove(&mut self, index: usize) {
        if index < self.codes.len() {
            self.codes.remove(index);
        }
    }
}

fn rotate_right_8(n: u8, shift: u8) -> u8 {
    (n >> shift) | (n << (8 - shift))
}

fn parse_game_genie(cleaned: &str) -> Option<(u16, u8, Option<u8>)> {
    let all_hex = cleaned.chars().all(|c| c.is_ascii_hexdigit());
    if !all_hex {
        return None;
    }
    let bytes: Vec<u8> = cleaned
        .chars()
        .map(|c| u8::from_str_radix(&c.to_string(), 16).ok())
        .collect::<Option<Vec<_>>>()?;

    match bytes.len() {
        6 => {
            let value = (bytes[0] << 4) | bytes[1];
            let addr_high = 0xF - bytes[5];
            let addr_low =
                (u16::from(bytes[2]) << 8) | (u16::from(bytes[3]) << 4) | u16::from(bytes[4]);
            let address = (u16::from(addr_high) << 12) | addr_low;
            Some((address, value, None))
        }
        9 => {
            let value = (bytes[0] << 4) | bytes[1];
            let addr_high = 0xF - bytes[5];
            let addr_low =
                (u16::from(bytes[2]) << 8) | (u16::from(bytes[3]) << 4) | u16::from(bytes[4]);
            let address = (u16::from(addr_high) << 12) | addr_low;
            let raw = (bytes[6] << 4) | bytes[8];
            let original = rotate_right_8(raw ^ 0xFF, 2) ^ 0x45;
            Some((address, value, Some(original)))
        }
        _ => None,
    }
}

fn parse_game_shark(cleaned: &str) -> Option<(u16, u8)> {
    let all_hex = cleaned.chars().all(|c| c.is_ascii_hexdigit());
    if !all_hex {
        return None;
    }
    let bytes: Vec<u8> = cleaned
        .chars()
        .map(|c| u8::from_str_radix(&c.to_string(), 16).ok())
        .collect::<Option<Vec<_>>>()?;

    match bytes.len() {
        8 => {
            let value = (bytes[2] << 4) | bytes[3];
            let addr_low = u16::from((bytes[4] << 4) | bytes[5]);
            let addr_high = u16::from((bytes[6] << 4) | bytes[7]);
            let address = (addr_high << 8) | addr_low;
            Some((address, value))
        }
        _ => None,
    }
}

pub(crate) fn parse_cheat_code(input: &str, description: String) -> Option<CheatCode> {
    let code = input.trim().to_string();
    let cleaned: String = input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();

    let cleaned_str = cleaned.as_str();

    if let Some((address, value)) = parse_game_shark(cleaned_str) {
        return Some(CheatCode {
            description,
            code,
            code_type: CheatCodeType::GameShark,
            address,
            value,
            compare: None,
            enabled: true,
        });
    }

    if cleaned_str.len() == 10 && &cleaned_str[0..2] == "01" {
        if let Some((address, value)) = parse_game_shark(&cleaned_str[2..]) {
            return Some(CheatCode {
                description,
                code,
                code_type: CheatCodeType::GameShark,
                address,
                value,
                compare: None,
                enabled: true,
            });
        }
    }

    if let Some((address, value, compare)) = parse_game_genie(cleaned_str) {
        return Some(CheatCode {
            description,
            code,
            code_type: CheatCodeType::GameGenie,
            address,
            value,
            compare,
            enabled: true,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gg6_decode(code: &str) -> (u16, u8) {
        let bytes: Vec<u8> = code
            .chars()
            .map(|c| c.to_digit(16).unwrap() as u8)
            .collect();
        let value = (bytes[0] << 4) | bytes[1];
        let addr = ((0xF - bytes[5]) as u16) << 12
            | (u16::from(bytes[2]) << 8)
            | (u16::from(bytes[3]) << 4)
            | u16::from(bytes[4]);
        (addr, value)
    }

    fn gg9_compare(code: &str) -> u8 {
        let bytes: Vec<u8> = code
            .chars()
            .map(|c| c.to_digit(16).unwrap() as u8)
            .collect();
        let raw = (bytes[6] << 4) | bytes[8];
        rotate_right_8(raw ^ 0xFF, 2) ^ 0x45
    }

    #[test]
    fn parse_game_genie_6_char() {
        let code = parse_cheat_code("004-BCE", "test".into()).unwrap();
        let (addr, val) = gg6_decode("004BCE");
        assert_eq!(code.address, addr);
        assert_eq!(code.value, val);
        assert_eq!(code.compare, None);
        assert!(code.enabled);
    }

    #[test]
    fn parse_game_genie_6_char_no_hyphen() {
        let code = parse_cheat_code("004BCE", "test".into()).unwrap();
        let (addr, val) = gg6_decode("004BCE");
        assert_eq!(code.address, addr);
        assert_eq!(code.value, val);
    }

    #[test]
    fn parse_game_genie_9_char() {
        let code = parse_cheat_code("004-BCE-E66", "test".into()).unwrap();
        let (addr, val) = gg6_decode("004BCE");
        assert_eq!(code.address, addr);
        assert_eq!(code.value, val);
        assert_eq!(code.compare, Some(gg9_compare("004BCEE66")));
        assert!(code.enabled);
    }

    #[test]
    fn parse_game_genie_9_char_no_hyphen() {
        let code = parse_cheat_code("004BCEE66", "test".into()).unwrap();
        let (addr, val) = gg6_decode("004BCE");
        assert_eq!(code.address, addr);
        assert_eq!(code.value, val);
        assert_eq!(code.compare, Some(gg9_compare("004BCEE66")));
    }

    #[test]
    fn parse_game_shark_8_hex_standard() {
        let code = parse_cheat_code("010238CD", "test".into()).unwrap();
        assert_eq!(code.address, 0xCD38);
        assert_eq!(code.value, 0x02);
        assert_eq!(code.compare, None);
        assert!(code.enabled);
    }

    #[test]
    fn parse_game_shark_8_hex_with_spaces() {
        let code = parse_cheat_code("01 02 38 CD", "test".into()).unwrap();
        assert_eq!(code.address, 0xCD38);
        assert_eq!(code.value, 0x02);
    }

    #[test]
    fn parse_game_shark_10_hex_with_01_prefix() {
        let code = parse_cheat_code("01010238CD", "test".into()).unwrap();
        assert_eq!(code.address, 0xCD38);
        assert_eq!(code.value, 0x02);
    }

    #[test]
    fn parse_game_shark_10_hex_with_spaces() {
        let code = parse_cheat_code("01 0102 38CD", "test".into()).unwrap();
        assert_eq!(code.address, 0xCD38);
        assert_eq!(code.value, 0x02);
    }

    #[test]
    fn parse_rejects_invalid_input() {
        assert!(parse_cheat_code("", "test".into()).is_none());
        assert!(parse_cheat_code("XYZ", "test".into()).is_none());
        assert!(parse_cheat_code("AAAA", "test".into()).is_none());
        assert!(parse_cheat_code("01", "test".into()).is_none());
        assert!(parse_cheat_code("GGGGGG", "test".into()).is_none());
    }

    #[test]
    fn parse_game_genie_known_example() {
        let code = parse_cheat_code("004-BCE-E66", "test".into()).unwrap();
        assert_eq!(code.address, 0x14BC);
        assert_eq!(code.value, 0x00);
        assert_eq!(code.compare, Some(0x03));
    }

    #[test]
    fn parse_game_shark_known_example() {
        let code = parse_cheat_code("010238CD", "test".into()).unwrap();
        assert_eq!(code.address, 0xCD38);
        assert_eq!(code.value, 0x02);
    }
}
