use crate::input::game_boy::GameBoyInputState;

const INPUT_IDLE_NIBBLE: u8 = 0x0f;
const RIGHT_MASK: u8 = 0x0e;
const LEFT_MASK: u8 = 0x0d;
const UP_MASK: u8 = 0x0b;
const DOWN_MASK: u8 = 0x07;
const SELECT_MASK: u8 = 0x0b;
const START_MASK: u8 = 0x07;
const A_MASK: u8 = 0x0e;
const B_MASK: u8 = 0x0d;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct JoypadInputNibbles {
    pub(crate) button: u8,
    pub(crate) direction: u8,
}

impl Default for JoypadInputNibbles {
    fn default() -> Self {
        Self {
            button: INPUT_IDLE_NIBBLE,
            direction: INPUT_IDLE_NIBBLE,
        }
    }
}

impl From<&GameBoyInputState> for JoypadInputNibbles {
    fn from(input: &GameBoyInputState) -> Self {
        let mut nibbles = Self::default();

        if input.dright {
            nibbles.direction &= RIGHT_MASK;
        }
        if input.dleft {
            nibbles.direction &= LEFT_MASK;
        }
        if input.dup {
            nibbles.direction &= UP_MASK;
        }
        if input.ddown {
            nibbles.direction &= DOWN_MASK;
        }
        if input.select {
            nibbles.button &= SELECT_MASK;
        }
        if input.start {
            nibbles.button &= START_MASK;
        }
        if input.a {
            nibbles.button &= A_MASK;
        }
        if input.b {
            nibbles.button &= B_MASK;
        }

        nibbles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_game_boy_input_matches_released_joypad_nibbles() {
        let input = GameBoyInputState::default();

        assert_eq!(
            JoypadInputNibbles::from(&input),
            JoypadInputNibbles::default()
        );
    }

    #[test]
    fn pressed_buttons_clear_the_active_low_joypad_bits() {
        let input = GameBoyInputState {
            dleft: true,
            dup: true,
            a: true,
            start: true,
            ..Default::default()
        };

        let nibbles = JoypadInputNibbles::from(&input);

        assert_eq!(nibbles.direction, 0x09);
        assert_eq!(nibbles.button, 0x06);
    }
}
