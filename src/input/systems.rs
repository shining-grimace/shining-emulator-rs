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
use crate::storage::LocalStorage;

pub(super) fn register_connected_controllers(
    mut connection_events: MessageReader<GamepadConnectionEvent>,
    mut controllers: ResMut<ConnectedControllers>,
    mut storage: ResMut<LocalStorage>,
    mut runtime_mappings: ResMut<RuntimeInputMappings>,
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
            }
            GamepadConnection::Disconnected => {
                controllers.remove(event.gamepad);
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

pub(super) fn collect_keyboard_input(
    mut keyboard_events: MessageReader<KeyboardInput>,
    mappings: Res<RuntimeInputMappings>,
    mut mapped_events: MessageWriter<MappedInputEvent>,
) {
    for event in keyboard_events.read() {
        if event.repeat {
            continue;
        }

        if let Some(action) = mappings.keyboard_action(event.key_code) {
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

        if let Some(action) = mappings.controller_action(&controller.model_id, event.button) {
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
