use std::collections::HashMap;

use bevy::prelude::*;

use crate::storage::input_mappings::{
    InputAction, InputDeviceMapping, InputDeviceType, InputKeyId, InputMapEntry,
};

#[derive(Resource, Clone, Debug, Default)]
pub(super) struct ConnectedControllers {
    devices: HashMap<Entity, ConnectedController>,
}

impl ConnectedControllers {
    pub(super) fn controller(&self, entity: Entity) -> Option<&ConnectedController> {
        self.devices.get(&entity)
    }

    pub(super) fn insert(&mut self, controller: ConnectedController) {
        self.devices.insert(controller.entity, controller);
    }

    pub(super) fn remove(&mut self, entity: Entity) {
        self.devices.remove(&entity);
    }
}

#[derive(Clone, Debug)]
pub(super) struct ConnectedController {
    pub entity: Entity,
    pub name: String,
    pub model_id: String,
}

pub(super) fn controller_model_id(
    name: &str,
    vendor_id: Option<u16>,
    product_id: Option<u16>,
) -> String {
    match (vendor_id, product_id) {
        (Some(vendor_id), Some(product_id)) => {
            format!("{vendor_id:04x}:{product_id:04x}:{name}")
        }
        _ => name.to_string(),
    }
}

pub(super) fn ensure_controller_mapping(
    mappings: &mut Vec<InputDeviceMapping>,
    model_id: &str,
) -> bool {
    if mappings.iter().any(|mapping| {
        mapping.r#type == InputDeviceType::Controller
            && mapping.controller_model_id.as_deref() == Some(model_id)
    }) {
        false
    } else {
        mappings.push(default_controller_mapping(model_id));
        true
    }
}

fn default_controller_mapping(model_id: &str) -> InputDeviceMapping {
    InputDeviceMapping {
        r#type: InputDeviceType::Controller,
        controller_model_id: Some(model_id.to_string()),
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
                key_id: InputKeyId::South,
                map_to: InputAction::B,
            },
            InputMapEntry {
                key_id: InputKeyId::East,
                map_to: InputAction::A,
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
                key_id: InputKeyId::Mode,
                map_to: InputAction::QuitRom,
            },
            InputMapEntry {
                key_id: InputKeyId::LeftTrigger,
                map_to: InputAction::LoadState0,
            },
            InputMapEntry {
                key_id: InputKeyId::RightTrigger,
                map_to: InputAction::SaveState0,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connected_controller_mapping_is_added_once() {
        let mut mappings = Vec::new();

        assert!(ensure_controller_mapping(&mut mappings, "controller-a"));
        assert!(!ensure_controller_mapping(&mut mappings, "controller-a"));

        assert_eq!(mappings.len(), 1);
        assert_eq!(
            mappings[0].controller_model_id.as_deref(),
            Some("controller-a")
        );
    }
}
