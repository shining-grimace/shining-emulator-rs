mod controls;
mod lifecycle;
mod tick;

pub(crate) use controls::{apply_emulator_control_events, sync_joypad_input};
pub(crate) use lifecycle::spawn_game_boy_emulator;
pub(crate) use tick::{persist_dirty_sram, tick_game_boy_emulator};

#[cfg(test)]
pub(super) use tick::execute_next_test_step;
