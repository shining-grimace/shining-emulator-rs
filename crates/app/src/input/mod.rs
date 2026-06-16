#![allow(dead_code)]

pub mod events;
pub mod game_boy;
pub mod mappings;
pub mod selection;
pub(crate) mod touch_overlay;

pub(crate) mod controller;
pub(crate) mod key_ids;
mod systems;

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::input::controller::ConnectedControllers;
use crate::input::events::MappedInputEvent;
use crate::input::game_boy::GameBoyInputState;
use crate::input::mappings::RuntimeInputMappings;
use crate::input::selection::{InputMappingEditTarget, PrimaryInputDevice};
use crate::input::systems::{
    collect_controller_input, collect_keyboard_input, register_connected_controllers,
    update_game_boy_input_state,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, SystemSet)]
pub enum InputSet {
    Collect,
    UpdateGameBoyState,
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RuntimeInputMappings>()
            .init_resource::<ConnectedControllers>()
            .init_resource::<GameBoyInputState>()
            .init_resource::<PrimaryInputDevice>()
            .init_resource::<InputMappingEditTarget>()
            .init_resource::<touch_overlay::TouchControllerOverlayInput>()
            .add_message::<MappedInputEvent>()
            .add_systems(
                Update,
                (
                    register_connected_controllers,
                    collect_keyboard_input,
                    collect_controller_input,
                )
                    .chain()
                    .in_set(InputSet::Collect),
            )
            .add_systems(
                Update,
                touch_overlay::collect_touch_controller_overlay_input
                    .in_set(InputSet::Collect)
                    .run_if(in_state(AppState::Gameplay)),
            )
            .add_systems(
                Update,
                update_game_boy_input_state
                    .in_set(InputSet::UpdateGameBoyState)
                    .after(InputSet::Collect),
            )
            .add_systems(
                Update,
                touch_overlay::update_touch_controller_overlay_visuals
                    .after(InputSet::UpdateGameBoyState)
                    .run_if(in_state(AppState::Gameplay)),
            )
            .add_systems(
                OnEnter(AppState::Gameplay),
                touch_overlay::spawn_touch_controller_overlay,
            )
            .add_systems(
                OnExit(AppState::Gameplay),
                (
                    touch_overlay::release_touch_controller_overlay_input,
                    touch_overlay::despawn_touch_controller_overlay,
                )
                    .chain(),
            );
    }
}
