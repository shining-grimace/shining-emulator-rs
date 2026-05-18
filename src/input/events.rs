use bevy::input::ButtonState;
use bevy::prelude::*;

use crate::storage::input_mappings::InputAction;

#[derive(Message, Clone, Debug)]
pub struct MappedInputEvent {
    pub action: InputAction,
    pub state: ButtonState,
}
