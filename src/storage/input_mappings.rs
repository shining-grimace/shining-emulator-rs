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
    KeyZ,
    KeyX,
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InputAction {
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

pub(super) fn default_input_mappings() -> Vec<InputDeviceMapping> {
    vec![InputDeviceMapping {
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
                key_id: InputKeyId::Escape,
                map_to: InputAction::QuitRom,
            },
            InputMapEntry {
                key_id: InputKeyId::ControlLeft,
                map_to: InputAction::SaveStateModifier,
            },
        ],
    }]
}
