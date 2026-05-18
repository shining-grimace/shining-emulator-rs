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
    pub key_id: String,
    pub map_to: InputAction,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
    S,
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
                key_id: "ArrowLeft".to_string(),
                map_to: InputAction::Dleft,
            },
            InputMapEntry {
                key_id: "ArrowRight".to_string(),
                map_to: InputAction::Dright,
            },
            InputMapEntry {
                key_id: "ArrowUp".to_string(),
                map_to: InputAction::Dup,
            },
            InputMapEntry {
                key_id: "ArrowDown".to_string(),
                map_to: InputAction::Ddown,
            },
            InputMapEntry {
                key_id: "KeyZ".to_string(),
                map_to: InputAction::B,
            },
            InputMapEntry {
                key_id: "KeyX".to_string(),
                map_to: InputAction::S,
            },
            InputMapEntry {
                key_id: "Enter".to_string(),
                map_to: InputAction::Start,
            },
            InputMapEntry {
                key_id: "ShiftRight".to_string(),
                map_to: InputAction::Select,
            },
            InputMapEntry {
                key_id: "Escape".to_string(),
                map_to: InputAction::QuitRom,
            },
            InputMapEntry {
                key_id: "ControlLeft".to_string(),
                map_to: InputAction::SaveStateModifier,
            },
        ],
    }]
}
