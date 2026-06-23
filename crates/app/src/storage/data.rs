use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage::input_mappings::InputDeviceMapping;
use crate::storage::providers::RomProvider;

#[derive(Clone, Debug, Default)]
pub struct LocalStorageData {
    pub settings: GeneralSettings,
    pub providers: Vec<RomProvider>,
    pub roms: Vec<RomMetadata>,
    pub timestamps: LastPlayedTimestamps,
    pub input_mappings: Vec<InputDeviceMapping>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {
    pub force_button_overlay: u8,
    pub upscaling_mode: u8,
    pub emulation_model: u8,
    pub sgb_overlay_enable: u8,
    #[serde(default = "crate::app_ui_scale::default_font_size")]
    pub font_size: u8,
    pub ui_theme: u8,
    pub audio_preset: u8,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            force_button_overlay: 0,
            upscaling_mode: 0,
            emulation_model: 0,
            sgb_overlay_enable: 0,
            font_size: crate::app_ui_scale::default_font_size(),
            ui_theme: 0,
            audio_preset: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RomMetadata {
    pub id: Option<String>,
    pub provider_id: Uuid,
    pub file_name: String,
    pub friendly_name: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub remote_provider_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastPlayedTimestamps {
    pub last_played: Vec<LastPlayedTimestamp>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LastPlayedTimestamp {
    pub id: String,
    pub timestamp: u64,
}
