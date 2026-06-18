use crate::input::game_boy::GameBoyInputState;

pub(crate) const JOYP_LOW_NIBBLE_MASK: u8 = 0x0f;
pub(crate) const JOYP_SELECT_MASK: u8 = 0x30;
pub(crate) const JOYP_SELECT_BUTTONS: u8 = 0x10;
pub(crate) const JOYP_SELECT_DIRECTIONS: u8 = 0x20;
pub(crate) const JOYP_SELECT_NONE: u8 = 0x30;

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

impl JoypadInputNibbles {
    pub(crate) fn low_nibble_for_select(self, select: u8) -> u8 {
        (match select & JOYP_SELECT_MASK {
            JOYP_SELECT_BUTTONS => self.button,
            JOYP_SELECT_DIRECTIONS => self.direction,
            0x00 => self.button & self.direction,
            _ => INPUT_IDLE_NIBBLE,
        }) & JOYP_LOW_NIBBLE_MASK
    }
}

pub(crate) fn joypad_low_nibble_falling_edge(old_joyp: u8, new_joyp: u8) -> bool {
    old_joyp & !new_joyp & JOYP_LOW_NIBBLE_MASK != 0
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

    #[test]
    fn selected_joypad_nibble_follows_active_low_selection_bits() {
        let nibbles = JoypadInputNibbles {
            button: 0x0e,
            direction: 0x0d,
        };

        assert_eq!(nibbles.low_nibble_for_select(JOYP_SELECT_BUTTONS), 0x0e);
        assert_eq!(nibbles.low_nibble_for_select(JOYP_SELECT_DIRECTIONS), 0x0d);
        assert_eq!(nibbles.low_nibble_for_select(0x00), 0x0c);
        assert_eq!(nibbles.low_nibble_for_select(JOYP_SELECT_NONE), 0x0f);
    }

    #[test]
    fn joypad_interrupt_edges_are_low_nibble_high_to_low_transitions() {
        assert!(joypad_low_nibble_falling_edge(0x2f, 0x2e));
        assert!(!joypad_low_nibble_falling_edge(0x2e, 0x2f));
        assert!(!joypad_low_nibble_falling_edge(0x1f, 0x2f));
    }
}
