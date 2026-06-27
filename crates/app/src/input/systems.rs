use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadButtonStateChangedEvent, GamepadConnection, GamepadConnectionEvent,
};
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::input::controller::{
    ConnectedController, ConnectedControllers, controller_model_id, ensure_controller_mapping,
};
use crate::input::events::MappedInputEvent;
use crate::input::game_boy::{GameBoyButton, GameBoyInputState};
use crate::input::mappings::RuntimeInputMappings;
use crate::input::selection::{PrimaryInputDevice, selected_mapping_index};
use crate::storage::LocalStorage;
use crate::storage::input_mappings::{InputDeviceMapping, InputDeviceType};

pub(super) fn register_connected_controllers(
    mut connection_events: MessageReader<GamepadConnectionEvent>,
    mut controllers: ResMut<ConnectedControllers>,
    mut storage: ResMut<LocalStorage>,
    mut runtime_mappings: ResMut<RuntimeInputMappings>,
    mut primary_input: ResMut<PrimaryInputDevice>,
) {
    let mut mappings_changed = false;

    for event in connection_events.read() {
        match &event.connection {
            GamepadConnection::Connected {
                name,
                vendor_id,
                product_id,
            } => {
                let model_id = controller_model_id(name, *vendor_id, *product_id);
                controllers.insert(ConnectedController {
                    entity: event.gamepad,
                    name: name.clone(),
                    model_id: model_id.clone(),
                });

                if ensure_controller_mapping(&mut storage.data.input_mappings, &model_id) {
                    mappings_changed = true;
                }
                select_connected_controller_mapping(
                    &mut primary_input,
                    &storage.data.input_mappings,
                    &model_id,
                );
            }
            GamepadConnection::Disconnected => {
                let removed_model_id = controllers
                    .controller(event.gamepad)
                    .map(|controller| controller.model_id.clone());
                controllers.remove(event.gamepad);
                if removed_model_id.as_deref().is_some_and(|model_id| {
                    primary_input_matches_model(&primary_input, &storage, model_id)
                }) && removed_model_id
                    .as_deref()
                    .is_none_or(|model_id| !controllers.contains_model_id(model_id))
                {
                    select_available_input_mapping(
                        &mut primary_input,
                        &storage.data.input_mappings,
                        &controllers,
                    );
                }
            }
        }
    }

    if mappings_changed {
        if let Err(error) = storage.save_input_mappings() {
            eprintln!("failed to save controller input mappings: {error}");
        }
        runtime_mappings.rebuild(&storage.data.input_mappings);
    }
}

fn select_connected_controller_mapping(
    primary_input: &mut PrimaryInputDevice,
    mappings: &[InputDeviceMapping],
    model_id: &str,
) {
    let selected_is_keyboard_or_missing = mappings
        .get(primary_input.mapping_index)
        .is_none_or(|mapping| mapping.r#type == InputDeviceType::Keyboard);
    if !selected_is_keyboard_or_missing {
        return;
    }

    if let Some(index) = controller_mapping_index(mappings, model_id) {
        primary_input.mapping_index = index;
    }
}

fn select_available_input_mapping(
    primary_input: &mut PrimaryInputDevice,
    mappings: &[InputDeviceMapping],
    controllers: &ConnectedControllers,
) {
    if let Some(model_id) = controllers.first_model_id()
        && let Some(index) = controller_mapping_index(mappings, model_id)
    {
        primary_input.mapping_index = index;
        return;
    }

    primary_input.mapping_index = mappings
        .iter()
        .position(|mapping| mapping.r#type == InputDeviceType::Keyboard)
        .unwrap_or(0);
}

fn primary_input_matches_model(
    primary_input: &PrimaryInputDevice,
    storage: &LocalStorage,
    model_id: &str,
) -> bool {
    storage
        .data
        .input_mappings
        .get(selected_mapping_index(primary_input, storage))
        .and_then(|mapping| mapping.controller_model_id.as_deref())
        == Some(model_id)
}

fn controller_mapping_index(mappings: &[InputDeviceMapping], model_id: &str) -> Option<usize> {
    mappings.iter().position(|mapping| {
        mapping.r#type == InputDeviceType::Controller
            && mapping.controller_model_id.as_deref() == Some(model_id)
    })
}

pub(super) fn collect_keyboard_input(
    mut keyboard_events: MessageReader<KeyboardInput>,
    mappings: Res<RuntimeInputMappings>,
    mut mapped_events: MessageWriter<MappedInputEvent>,
) {
    for event in keyboard_events.read() {
        if event.repeat {
            continue;
        }

        for action in mappings.keyboard_actions(event.key_code) {
            mapped_events.write(MappedInputEvent {
                action,
                state: event.state,
            });
        }
    }
}

pub(super) fn collect_controller_input(
    mut controller_events: MessageReader<GamepadButtonStateChangedEvent>,
    controllers: Res<ConnectedControllers>,
    mappings: Res<RuntimeInputMappings>,
    mut mapped_events: MessageWriter<MappedInputEvent>,
) {
    for event in controller_events.read() {
        let Some(controller) = controllers.controller(event.entity) else {
            continue;
        };

        for action in mappings.controller_actions(&controller.model_id, event.button) {
            mapped_events.write(MappedInputEvent {
                action,
                state: event.state,
            });
        }
    }
}

pub(super) fn update_game_boy_input_state(
    mut mapped_events: MessageReader<MappedInputEvent>,
    mut input_state: ResMut<GameBoyInputState>,
) {
    for event in mapped_events.read() {
        if let Ok(button) = GameBoyButton::try_from(event.action) {
            input_state.set_button(button, event.state == ButtonState::Pressed);
        }
    }
}
