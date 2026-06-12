use crate::game_boy::emulator::constants::DEFAULT_CLOCK_MULTIPLIER_INDEX;
use crate::game_boy::emulator::input::JoypadInputNibbles;

#[derive(Clone, Copy, Debug)]
pub(crate) struct RuntimeControl {
    pub(crate) is_running: bool,
    pub(crate) is_paused: bool,
    pub(crate) joypad: JoypadInputNibbles,
    pub(crate) joypad_state_changed: bool,
    pub(crate) clock_multiply: i64,
    pub(crate) clock_divide: i64,
    pub(crate) current_clock_multiplier_combo: i32,
}

impl Default for RuntimeControl {
    fn default() -> Self {
        Self {
            is_running: false,
            is_paused: false,
            joypad: JoypadInputNibbles::default(),
            joypad_state_changed: false,
            clock_multiply: 1,
            clock_divide: 1,
            current_clock_multiplier_combo: DEFAULT_CLOCK_MULTIPLIER_INDEX,
        }
    }
}

impl RuntimeControl {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.is_running = true;
        self.is_paused = false;
        self.joypad = JoypadInputNibbles::default();
        self.joypad_state_changed = false;
        self.clock_multiply = 1;
        self.clock_divide = 1;
        self.current_clock_multiplier_combo = DEFAULT_CLOCK_MULTIPLIER_INDEX;
    }
}
