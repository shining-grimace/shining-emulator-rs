use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDeviceMapping {
    pub r#type: InputDeviceType,
    pub controller_model_id: Option<String>,
    pub map: Vec<InputMapEntry>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InputDeviceType {
    Keyboard,
    Controller,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputMapEntry {
    pub key_id: InputKeyId,
    pub map_to: InputAction,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum InputKeyId {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyZ,
    KeyX,
    KeyY,
    Enter,
    ShiftRight,
    ShiftLeft,
    Escape,
    ControlLeft,
    ControlRight,
    Space,
    Tab,
    Backspace,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    South,
    East,
    North,
    West,
    C,
    Z,
    LeftTrigger,
    LeftTrigger2,
    RightTrigger,
    RightTrigger2,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InputAction {
    #[default]
    QuitApp,
    QuitRom,
    ResetRom,
    SaveState0,
    LoadState0,
    SaveStateModifier,
    LoadStateModifier,
    SpeedUp,
    SpeedDown,
    PauseAndResume,
    Dleft,
    Dright,
    Dup,
    Ddown,
    A,
    B,
    Start,
    Select,
}

pub(crate) fn default_input_mappings() -> Vec<InputDeviceMapping> {
    vec![default_keyboard_mapping()]
}

pub(crate) fn default_keyboard_mapping() -> InputDeviceMapping {
    InputDeviceMapping {
        r#type: InputDeviceType::Keyboard,
        controller_model_id: None,
        map: vec![
            InputMapEntry {
                key_id: InputKeyId::ArrowLeft,
                map_to: InputAction::Dleft,
            },
            InputMapEntry {
                key_id: InputKeyId::ArrowRight,
                map_to: InputAction::Dright,
            },
            InputMapEntry {
                key_id: InputKeyId::ArrowUp,
                map_to: InputAction::Dup,
            },
            InputMapEntry {
                key_id: InputKeyId::ArrowDown,
                map_to: InputAction::Ddown,
            },
            InputMapEntry {
                key_id: InputKeyId::Escape,
                map_to: InputAction::QuitApp,
            },
            InputMapEntry {
                key_id: InputKeyId::KeyZ,
                map_to: InputAction::B,
            },
            InputMapEntry {
                key_id: InputKeyId::KeyX,
                map_to: InputAction::A,
            },
            InputMapEntry {
                key_id: InputKeyId::Enter,
                map_to: InputAction::Start,
            },
            InputMapEntry {
                key_id: InputKeyId::ShiftRight,
                map_to: InputAction::Select,
            },
            InputMapEntry {
                key_id: InputKeyId::ControlLeft,
                map_to: InputAction::SaveStateModifier,
            },
        ],
    }
}

pub(crate) fn default_controller_mapping(model_id: impl Into<String>) -> InputDeviceMapping {
    let model_id = model_id.into();
    let (a_key_id, b_key_id) = if is_xbox_360_controller_model_id(&model_id) {
        (InputKeyId::South, InputKeyId::West)
    } else {
        (InputKeyId::East, InputKeyId::South)
    };

    InputDeviceMapping {
        r#type: InputDeviceType::Controller,
        controller_model_id: Some(model_id),
        map: vec![
            InputMapEntry {
                key_id: InputKeyId::DPadLeft,
                map_to: InputAction::Dleft,
            },
            InputMapEntry {
                key_id: InputKeyId::DPadRight,
                map_to: InputAction::Dright,
            },
            InputMapEntry {
                key_id: InputKeyId::DPadUp,
                map_to: InputAction::Dup,
            },
            InputMapEntry {
                key_id: InputKeyId::DPadDown,
                map_to: InputAction::Ddown,
            },
            InputMapEntry {
                key_id: a_key_id,
                map_to: InputAction::A,
            },
            InputMapEntry {
                key_id: b_key_id,
                map_to: InputAction::B,
            },
            InputMapEntry {
                key_id: InputKeyId::Start,
                map_to: InputAction::Start,
            },
            InputMapEntry {
                key_id: InputKeyId::Select,
                map_to: InputAction::Select,
            },
            InputMapEntry {
                key_id: InputKeyId::LeftTrigger,
                map_to: InputAction::SaveState0,
            },
            InputMapEntry {
                key_id: InputKeyId::RightTrigger,
                map_to: InputAction::LoadState0,
            },
            InputMapEntry {
                key_id: InputKeyId::RightTrigger2,
                map_to: InputAction::SpeedUp,
            },
            InputMapEntry {
                key_id: InputKeyId::LeftTrigger2,
                map_to: InputAction::SpeedDown,
            },
            InputMapEntry {
                key_id: InputKeyId::Mode,
                map_to: InputAction::PauseAndResume,
            },
        ],
    }
}

pub(crate) fn is_xbox_360_controller_model_id(model_id: &str) -> bool {
    let lower = model_id.to_ascii_lowercase();
    lower.contains("xbox 360")
        || lower.contains("x-box 360")
        || lower.starts_with("045e:028e:")
        || lower.starts_with("045e:028f:")
        || lower.starts_with("045e:0719:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xbox_360_default_maps_physical_a_to_emulated_a_and_x_to_emulated_b() {
        let mapping = default_controller_mapping("045e:028e:Xbox 360 Controller");

        assert!(
            mapping
                .map
                .iter()
                .any(|entry| entry.key_id == InputKeyId::South && entry.map_to == InputAction::A)
        );
        assert!(
            mapping
                .map
                .iter()
                .any(|entry| entry.key_id == InputKeyId::West && entry.map_to == InputAction::B)
        );
    }
}
