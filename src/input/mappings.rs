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
    keyboard: HashMap<KeyCode, Vec<InputAction>>,
    controllers: HashMap<String, HashMap<GamepadButton, Vec<InputAction>>>,
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
                            self.keyboard
                                .entry(key_code)
                                .or_default()
                                .push(entry.map_to);
                        }
                    }
                }
                InputDeviceType::Controller => {
                    if let Some(model_id) = &mapping.controller_model_id {
                        let controller_map = self.controllers.entry(model_id.clone()).or_default();
                        for entry in &mapping.map {
                            if let Some(button) = gamepad_button_from_id(entry.key_id) {
                                controller_map.entry(button).or_default().push(entry.map_to);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn set_keyboard_mapping(&mut self, key_code: KeyCode, action: InputAction) {
        self.keyboard.entry(key_code).or_default().push(action);
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
            .entry(button)
            .or_default()
            .push(action);
    }

    pub fn keyboard_action(&self, key_code: KeyCode) -> Option<InputAction> {
        self.keyboard
            .get(&key_code)
            .and_then(|actions| actions.first().copied())
    }

    pub fn keyboard_actions(&self, key_code: KeyCode) -> impl Iterator<Item = InputAction> + '_ {
        self.keyboard.get(&key_code).into_iter().flatten().copied()
    }

    pub fn keyboard_key_for_action(&self, action: InputAction) -> Option<KeyCode> {
        self.keyboard
            .iter()
            .find_map(|(key_code, actions)| actions.contains(&action).then_some(*key_code))
    }

    pub(super) fn controller_actions(
        &self,
        model_id: &str,
        button: GamepadButton,
    ) -> impl Iterator<Item = InputAction> + '_ {
        self.controllers
            .get(model_id)
            .and_then(move |controller| controller.get(&button))
            .into_iter()
            .flatten()
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
            .flat_map(|(key_code, actions)| {
                actions.iter().filter_map(|action| {
                    Some(InputMapEntry {
                        key_id: key_code_id(*key_code)?,
                        map_to: *action,
                    })
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
                .flat_map(|(button, actions)| {
                    actions.iter().filter_map(|action| {
                        Some(InputMapEntry {
                            key_id: gamepad_button_id(*button)?,
                            map_to: *action,
                        })
                    })
                })
                .collect(),
        });
    }

    storage.data.input_mappings = mappings;
    storage.save_input_mappings()
}

pub fn ensure_essential_navigation_mappings(mapping: &mut InputDeviceMapping) -> bool {
    let mut changed = ensure_keyboard_quit_app_mapping(mapping);
    let mut mapped_actions = mapping
        .map
        .iter()
        .map(|entry| entry.map_to)
        .collect::<HashSet<_>>();
    let mut mapped_keys = mapping
        .map
        .iter()
        .map(|entry| entry.key_id)
        .collect::<HashSet<_>>();
    for entry in default_navigation_entries(mapping.r#type) {
        if !mapped_actions.contains(&entry.map_to) && !mapped_keys.contains(&entry.key_id) {
            mapped_actions.insert(entry.map_to);
            mapped_keys.insert(entry.key_id);
            mapping.map.push(entry);
            changed = true;
        }
    }

    changed
}

fn ensure_keyboard_quit_app_mapping(mapping: &mut InputDeviceMapping) -> bool {
    if mapping.r#type != InputDeviceType::Keyboard
        || mapping
            .map
            .iter()
            .any(|entry| entry.map_to == InputAction::QuitApp)
    {
        return false;
    }

    if let Some(entry) = mapping
        .map
        .iter_mut()
        .find(|entry| entry.key_id == InputKeyId::Escape)
    {
        entry.map_to = InputAction::QuitApp;
        true
    } else {
        false
    }
}

fn default_navigation_entries(device_type: InputDeviceType) -> Vec<InputMapEntry> {
    match device_type {
        InputDeviceType::Keyboard => vec![
            InputMapEntry {
                key_id: InputKeyId::Escape,
                map_to: InputAction::QuitApp,
            },
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
            runtime
                .controller_actions("controller-a", GamepadButton::South)
                .collect::<Vec<_>>(),
            vec![InputAction::B]
        );
    }

    #[test]
    fn essential_navigation_mappings_do_not_duplicate_existing_keys() {
        let mut mapping = InputDeviceMapping {
            r#type: InputDeviceType::Keyboard,
            controller_model_id: None,
            map: vec![InputMapEntry {
                key_id: InputKeyId::KeyX,
                map_to: InputAction::Select,
            }],
        };

        assert!(ensure_essential_navigation_mappings(&mut mapping));

        assert_eq!(
            mapping
                .map
                .iter()
                .filter(|entry| entry.key_id == InputKeyId::KeyX)
                .count(),
            1
        );
        assert!(
            !mapping
                .map
                .iter()
                .any(|entry| entry.map_to == InputAction::A)
        );
    }

    #[test]
    fn old_escape_quit_rom_mapping_is_migrated_to_quit_app() {
        let mut mapping = InputDeviceMapping {
            r#type: InputDeviceType::Keyboard,
            controller_model_id: None,
            map: vec![InputMapEntry {
                key_id: InputKeyId::Escape,
                map_to: InputAction::QuitRom,
            }],
        };

        assert!(ensure_essential_navigation_mappings(&mut mapping));

        assert!(mapping.map.iter().any(
            |entry| entry.key_id == InputKeyId::Escape && entry.map_to == InputAction::QuitApp
        ));
        assert_eq!(
            mapping
                .map
                .iter()
                .filter(|entry| entry.key_id == InputKeyId::Escape)
                .count(),
            1
        );
    }
}
