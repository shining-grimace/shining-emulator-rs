use bevy::prelude::*;

use crate::storage::LocalStorage;
use crate::storage::input_mappings::{InputDeviceMapping, InputDeviceType};

#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct PrimaryInputDevice {
    pub mapping_index: usize,
}

#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct InputMappingEditTarget {
    pub mapping_index: usize,
}

pub fn selected_mapping_index(selection: &PrimaryInputDevice, storage: &LocalStorage) -> usize {
    if storage.data.input_mappings.is_empty() {
        0
    } else {
        selection
            .mapping_index
            .min(storage.data.input_mappings.len().saturating_sub(1))
    }
}

pub fn selected_mapping<'a>(
    selection: &PrimaryInputDevice,
    storage: &'a LocalStorage,
) -> Option<&'a InputDeviceMapping> {
    storage
        .data
        .input_mappings
        .get(selected_mapping_index(selection, storage))
}

pub fn mapping_label(mapping: &InputDeviceMapping) -> String {
    match mapping.r#type {
        InputDeviceType::Keyboard => "Keyboard".to_string(),
        InputDeviceType::Controller => mapping
            .controller_model_id
            .as_deref()
            .map(controller_display_name)
            .unwrap_or_else(|| "Controller".to_string()),
    }
}

fn controller_display_name(model_id: &str) -> String {
    let mut parts = model_id.splitn(3, ':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(_), Some(_), Some(name)) => name.to_string(),
        _ => model_id.to_string(),
    }
}
