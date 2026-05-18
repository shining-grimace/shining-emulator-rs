use bevy::input::gamepad::GamepadButton;
use bevy::input::keyboard::KeyCode;

use crate::storage::input_mappings::InputKeyId;

pub(super) fn key_code_from_id(key_id: InputKeyId) -> Option<KeyCode> {
    match key_id {
        InputKeyId::ArrowLeft => Some(KeyCode::ArrowLeft),
        InputKeyId::ArrowRight => Some(KeyCode::ArrowRight),
        InputKeyId::ArrowUp => Some(KeyCode::ArrowUp),
        InputKeyId::ArrowDown => Some(KeyCode::ArrowDown),
        InputKeyId::KeyZ => Some(KeyCode::KeyZ),
        InputKeyId::KeyX => Some(KeyCode::KeyX),
        InputKeyId::Enter => Some(KeyCode::Enter),
        InputKeyId::ShiftRight => Some(KeyCode::ShiftRight),
        InputKeyId::ShiftLeft => Some(KeyCode::ShiftLeft),
        InputKeyId::Escape => Some(KeyCode::Escape),
        InputKeyId::ControlLeft => Some(KeyCode::ControlLeft),
        InputKeyId::ControlRight => Some(KeyCode::ControlRight),
        InputKeyId::Space => Some(KeyCode::Space),
        InputKeyId::Tab => Some(KeyCode::Tab),
        InputKeyId::Backspace => Some(KeyCode::Backspace),
        InputKeyId::Digit0 => Some(KeyCode::Digit0),
        InputKeyId::Digit1 => Some(KeyCode::Digit1),
        InputKeyId::Digit2 => Some(KeyCode::Digit2),
        InputKeyId::Digit3 => Some(KeyCode::Digit3),
        InputKeyId::Digit4 => Some(KeyCode::Digit4),
        InputKeyId::Digit5 => Some(KeyCode::Digit5),
        InputKeyId::Digit6 => Some(KeyCode::Digit6),
        InputKeyId::Digit7 => Some(KeyCode::Digit7),
        InputKeyId::Digit8 => Some(KeyCode::Digit8),
        InputKeyId::Digit9 => Some(KeyCode::Digit9),
        _ => None,
    }
}

pub(super) fn key_code_id(key_code: KeyCode) -> Option<InputKeyId> {
    match key_code {
        KeyCode::ArrowLeft => Some(InputKeyId::ArrowLeft),
        KeyCode::ArrowRight => Some(InputKeyId::ArrowRight),
        KeyCode::ArrowUp => Some(InputKeyId::ArrowUp),
        KeyCode::ArrowDown => Some(InputKeyId::ArrowDown),
        KeyCode::KeyZ => Some(InputKeyId::KeyZ),
        KeyCode::KeyX => Some(InputKeyId::KeyX),
        KeyCode::Enter => Some(InputKeyId::Enter),
        KeyCode::ShiftRight => Some(InputKeyId::ShiftRight),
        KeyCode::ShiftLeft => Some(InputKeyId::ShiftLeft),
        KeyCode::Escape => Some(InputKeyId::Escape),
        KeyCode::ControlLeft => Some(InputKeyId::ControlLeft),
        KeyCode::ControlRight => Some(InputKeyId::ControlRight),
        KeyCode::Space => Some(InputKeyId::Space),
        KeyCode::Tab => Some(InputKeyId::Tab),
        KeyCode::Backspace => Some(InputKeyId::Backspace),
        KeyCode::Digit0 => Some(InputKeyId::Digit0),
        KeyCode::Digit1 => Some(InputKeyId::Digit1),
        KeyCode::Digit2 => Some(InputKeyId::Digit2),
        KeyCode::Digit3 => Some(InputKeyId::Digit3),
        KeyCode::Digit4 => Some(InputKeyId::Digit4),
        KeyCode::Digit5 => Some(InputKeyId::Digit5),
        KeyCode::Digit6 => Some(InputKeyId::Digit6),
        KeyCode::Digit7 => Some(InputKeyId::Digit7),
        KeyCode::Digit8 => Some(InputKeyId::Digit8),
        KeyCode::Digit9 => Some(InputKeyId::Digit9),
        _ => None,
    }
}

pub(super) fn gamepad_button_from_id(key_id: InputKeyId) -> Option<GamepadButton> {
    match key_id {
        InputKeyId::South => Some(GamepadButton::South),
        InputKeyId::East => Some(GamepadButton::East),
        InputKeyId::North => Some(GamepadButton::North),
        InputKeyId::West => Some(GamepadButton::West),
        InputKeyId::C => Some(GamepadButton::C),
        InputKeyId::Z => Some(GamepadButton::Z),
        InputKeyId::LeftTrigger => Some(GamepadButton::LeftTrigger),
        InputKeyId::LeftTrigger2 => Some(GamepadButton::LeftTrigger2),
        InputKeyId::RightTrigger => Some(GamepadButton::RightTrigger),
        InputKeyId::RightTrigger2 => Some(GamepadButton::RightTrigger2),
        InputKeyId::Select => Some(GamepadButton::Select),
        InputKeyId::Start => Some(GamepadButton::Start),
        InputKeyId::Mode => Some(GamepadButton::Mode),
        InputKeyId::LeftThumb => Some(GamepadButton::LeftThumb),
        InputKeyId::RightThumb => Some(GamepadButton::RightThumb),
        InputKeyId::DPadUp => Some(GamepadButton::DPadUp),
        InputKeyId::DPadDown => Some(GamepadButton::DPadDown),
        InputKeyId::DPadLeft => Some(GamepadButton::DPadLeft),
        InputKeyId::DPadRight => Some(GamepadButton::DPadRight),
        _ => None,
    }
}

pub(super) fn gamepad_button_id(button: GamepadButton) -> Option<InputKeyId> {
    match button {
        GamepadButton::South => Some(InputKeyId::South),
        GamepadButton::East => Some(InputKeyId::East),
        GamepadButton::North => Some(InputKeyId::North),
        GamepadButton::West => Some(InputKeyId::West),
        GamepadButton::C => Some(InputKeyId::C),
        GamepadButton::Z => Some(InputKeyId::Z),
        GamepadButton::LeftTrigger => Some(InputKeyId::LeftTrigger),
        GamepadButton::LeftTrigger2 => Some(InputKeyId::LeftTrigger2),
        GamepadButton::RightTrigger => Some(InputKeyId::RightTrigger),
        GamepadButton::RightTrigger2 => Some(InputKeyId::RightTrigger2),
        GamepadButton::Select => Some(InputKeyId::Select),
        GamepadButton::Start => Some(InputKeyId::Start),
        GamepadButton::Mode => Some(InputKeyId::Mode),
        GamepadButton::LeftThumb => Some(InputKeyId::LeftThumb),
        GamepadButton::RightThumb => Some(InputKeyId::RightThumb),
        GamepadButton::DPadUp => Some(InputKeyId::DPadUp),
        GamepadButton::DPadDown => Some(InputKeyId::DPadDown),
        GamepadButton::DPadLeft => Some(InputKeyId::DPadLeft),
        GamepadButton::DPadRight => Some(InputKeyId::DPadRight),
        GamepadButton::Other(_) => None,
    }
}
