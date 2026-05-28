#![allow(dead_code)]

pub mod events;
pub mod game_boy;
pub mod mappings;
pub mod selection;

mod controller;
pub(crate) mod key_ids;
mod systems;

use bevy::prelude::*;

use crate::input::controller::ConnectedControllers;
use crate::input::events::MappedInputEvent;
use crate::input::game_boy::GameBoyInputState;
use crate::input::mappings::RuntimeInputMappings;
use crate::input::selection::{InputMappingEditTarget, PrimaryInputDevice};
use crate::input::systems::{
    collect_controller_input, collect_keyboard_input, register_connected_controllers,
    update_game_boy_input_state,
};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RuntimeInputMappings>()
            .init_resource::<ConnectedControllers>()
            .init_resource::<GameBoyInputState>()
            .init_resource::<PrimaryInputDevice>()
            .init_resource::<InputMappingEditTarget>()
            .add_message::<MappedInputEvent>()
            .add_systems(
                Update,
                (
                    register_connected_controllers,
                    collect_keyboard_input,
                    collect_controller_input,
                    update_game_boy_input_state,
                )
                    .chain(),
            );
    }
}
