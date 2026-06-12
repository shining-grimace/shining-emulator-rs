use bevy::input::ButtonState;
use bevy::prelude::*;

use crate::game_boy::emulator::constants::{CLOCK_DIVISORS, CLOCK_MULTIPLIERS};
use crate::game_boy::emulator::input::JoypadInputNibbles;
use crate::game_boy::emulator::runtime::RuntimeControl;
use crate::game_boy::emulator::{GameBoyCore, GameBoyEmulator};
use crate::input::events::MappedInputEvent;
use crate::input::game_boy::GameBoyInputState;
use crate::storage::input_mappings::InputAction;

pub(crate) fn sync_joypad_input(
    input: Res<GameBoyInputState>,
    mut emulators: Query<&mut GameBoyCore, With<GameBoyEmulator>>,
) {
    if !input.is_changed() {
        return;
    }

    let next_joypad = JoypadInputNibbles::from(input.as_ref());
    for mut emulator in &mut emulators {
        let control = &mut emulator.runtime;
        if control.joypad == next_joypad {
            continue;
        }

        control.joypad = next_joypad;
        control.joypad_state_changed = true;
    }
}

pub(crate) fn apply_emulator_control_events(
    mut input_events: MessageReader<MappedInputEvent>,
    mut emulators: Query<&mut GameBoyCore, With<GameBoyEmulator>>,
) {
    for event in input_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        match event.action {
            InputAction::PauseAndResume => {
                for mut emulator in &mut emulators {
                    let control = &mut emulator.runtime;
                    if control.is_running {
                        control.is_paused = !control.is_paused;
                    }
                }
            }
            InputAction::SpeedUp => {
                for mut emulator in &mut emulators {
                    speed_up(&mut emulator.runtime);
                }
            }
            InputAction::SpeedDown => {
                for mut emulator in &mut emulators {
                    slow_down(&mut emulator.runtime);
                }
            }
            _ => {}
        }
    }
}

fn speed_up(control: &mut RuntimeControl) {
    let next_index = control.current_clock_multiplier_combo.saturating_add(1);
    set_clock_multiplier(control, next_index);
}

fn slow_down(control: &mut RuntimeControl) {
    let next_index = control.current_clock_multiplier_combo.saturating_sub(1);
    set_clock_multiplier(control, next_index);
}

fn set_clock_multiplier(control: &mut RuntimeControl, index: i32) {
    let Some(index) = usize::try_from(index).ok() else {
        return;
    };
    let Some((&multiply, &divide)) = CLOCK_MULTIPLIERS.get(index).zip(CLOCK_DIVISORS.get(index))
    else {
        return;
    };

    control.current_clock_multiplier_combo = index as i32;
    control.clock_multiply = multiply;
    control.clock_divide = divide;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_controls_follow_the_legacy_multiplier_table() {
        let mut control = RuntimeControl::default();

        speed_up(&mut control);

        assert_eq!(control.current_clock_multiplier_combo, 11);
        assert_eq!(control.clock_multiply, 5);
        assert_eq!(control.clock_divide, 4);

        slow_down(&mut control);

        assert_eq!(control.current_clock_multiplier_combo, 10);
        assert_eq!(control.clock_multiply, 1);
        assert_eq!(control.clock_divide, 1);
    }
}
