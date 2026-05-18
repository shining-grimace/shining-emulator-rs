use std::collections::{HashMap, HashSet};

use bevy::input::gamepad::GamepadButton;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;

use crate::input::key_ids::{
    gamepad_button_from_id, gamepad_button_id, key_code_from_id, key_code_id,
};
use crate::storage::LocalStorage;
use crate::storage::errors::StorageError;
use crate::storage::input_mappings::{
    InputAction, InputDeviceMapping, InputDeviceType, InputKeyId, InputMapEntry,
};

#[derive(Resource, Clone, Debug)]
pub struct RuntimeInputMappings {
    keyboard: HashMap<KeyCode, InputAction>,
    controllers: HashMap<String, HashMap<GamepadButton, InputAction>>,
}

impl RuntimeInputMappings {
    pub fn empty() -> Self {
        Self {
            keyboard: HashMap::new(),
            controllers: HashMap::new(),
        }
    }

    pub fn from_storage_mappings(mappings: &[InputDeviceMapping]) -> Self {
        let mut runtime = Self::empty();
        runtime.rebuild(mappings);
        runtime
    }

    pub fn rebuild(&mut self, mappings: &[InputDeviceMapping]) {
        self.keyboard.clear();
        self.controllers.clear();

        for mapping in mappings {
            match mapping.r#type {
                InputDeviceType::Keyboard => {
                    for entry in &mapping.map {
                        if let Some(key_code) = key_code_from_id(entry.key_id) {
                            self.keyboard.insert(key_code, entry.map_to);
                        }
                    }
                }
                InputDeviceType::Controller => {
                    if let Some(model_id) = &mapping.controller_model_id {
                        let controller_map = self.controllers.entry(model_id.clone()).or_default();
                        for entry in &mapping.map {
                            if let Some(button) = gamepad_button_from_id(entry.key_id) {
                                controller_map.insert(button, entry.map_to);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn set_keyboard_mapping(&mut self, key_code: KeyCode, action: InputAction) {
        self.keyboard.insert(key_code, action);
    }

    pub fn set_controller_mapping(
        &mut self,
        model_id: impl Into<String>,
        button: GamepadButton,
        action: InputAction,
    ) {
        self.controllers
            .entry(model_id.into())
            .or_default()
            .insert(button, action);
    }

    pub(super) fn keyboard_action(&self, key_code: KeyCode) -> Option<InputAction> {
        self.keyboard.get(&key_code).copied()
    }

    pub(super) fn controller_action(
        &self,
        model_id: &str,
        button: GamepadButton,
    ) -> Option<InputAction> {
        self.controllers
            .get(model_id)
            .and_then(|controller| controller.get(&button))
            .copied()
    }
}

impl FromWorld for RuntimeInputMappings {
    fn from_world(world: &mut World) -> Self {
        world
            .get_resource::<LocalStorage>()
            .map(|storage| Self::from_storage_mappings(&storage.data.input_mappings))
            .unwrap_or_else(Self::empty)
    }
}

pub fn sync_storage_mappings_from_runtime(
    storage: &mut LocalStorage,
    runtime_mappings: &RuntimeInputMappings,
) -> Result<(), StorageError> {
    let mut mappings = Vec::new();
    mappings.push(InputDeviceMapping {
        r#type: InputDeviceType::Keyboard,
        controller_model_id: None,
        map: runtime_mappings
            .keyboard
            .iter()
            .filter_map(|(key_code, action)| {
                Some(InputMapEntry {
                    key_id: key_code_id(*key_code)?,
                    map_to: *action,
                })
            })
            .collect(),
    });

    for (model_id, controller_mapping) in &runtime_mappings.controllers {
        mappings.push(InputDeviceMapping {
            r#type: InputDeviceType::Controller,
            controller_model_id: Some(model_id.clone()),
            map: controller_mapping
                .iter()
                .filter_map(|(button, action)| {
                    Some(InputMapEntry {
                        key_id: gamepad_button_id(*button)?,
                        map_to: *action,
                    })
                })
                .collect(),
        });
    }

    storage.data.input_mappings = mappings;
    storage.save_input_mappings()
}

pub fn ensure_essential_navigation_mappings(mapping: &mut InputDeviceMapping) {
    let mut mapped_actions = mapping
        .map
        .iter()
        .map(|entry| entry.map_to)
        .collect::<HashSet<_>>();

    for entry in default_navigation_entries(mapping.r#type) {
        if !mapped_actions.contains(&entry.map_to) {
            mapped_actions.insert(entry.map_to);
            mapping.map.push(entry);
        }
    }
}

fn default_navigation_entries(device_type: InputDeviceType) -> Vec<InputMapEntry> {
    match device_type {
        InputDeviceType::Keyboard => vec![
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
                key_id: InputKeyId::KeyX,
                map_to: InputAction::A,
            },
            InputMapEntry {
                key_id: InputKeyId::KeyZ,
                map_to: InputAction::B,
            },
        ],
        InputDeviceType::Controller => vec![
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
                key_id: InputKeyId::East,
                map_to: InputAction::A,
            },
            InputMapEntry {
                key_id: InputKeyId::South,
                map_to: InputAction::B,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_mappings_are_built_from_stored_keyboard_mappings() {
        let mappings = vec![InputDeviceMapping {
            r#type: InputDeviceType::Keyboard,
            controller_model_id: None,
            map: vec![InputMapEntry {
                key_id: InputKeyId::KeyX,
                map_to: InputAction::A,
            }],
        }];

        let runtime = RuntimeInputMappings::from_storage_mappings(&mappings);

        assert_eq!(runtime.keyboard_action(KeyCode::KeyX), Some(InputAction::A));
    }

    #[test]
    fn runtime_mappings_are_built_from_stored_controller_mappings() {
        let mappings = vec![InputDeviceMapping {
            r#type: InputDeviceType::Controller,
            controller_model_id: Some("controller-a".to_string()),
            map: vec![InputMapEntry {
                key_id: InputKeyId::South,
                map_to: InputAction::B,
            }],
        }];

        let runtime = RuntimeInputMappings::from_storage_mappings(&mappings);

        assert_eq!(
            runtime.controller_action("controller-a", GamepadButton::South),
            Some(InputAction::B)
        );
    }
}
