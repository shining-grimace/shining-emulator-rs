#![allow(dead_code)]

pub mod effects;

mod components;
mod constants;
mod systems;
mod utils;

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::background::effects::BackgroundDisplay;
use crate::background::systems::{
    animate_particles, configure_background_image_sampler, fade_background_in, fade_background_out,
    resize_background_image, spawn_background_entities, update_background_opacity,
};

pub struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BackgroundDisplay>()
            .add_systems(OnEnter(AppState::Loading), fade_background_out)
            .add_systems(OnEnter(AppState::Splash), fade_background_out)
            .add_systems(OnEnter(AppState::InterfaceDemo), fade_background_in)
            .add_systems(OnEnter(AppState::Home), fade_background_in)
            .add_systems(OnEnter(AppState::Settings), fade_background_in)
            .add_systems(OnEnter(AppState::InputMapping), fade_background_in)
            .add_systems(OnEnter(AppState::RomProvider), fade_background_in)
            .add_systems(OnEnter(AppState::RomData), fade_background_in)
            .add_systems(OnEnter(AppState::AudioSettings), fade_background_in)
            .add_systems(OnEnter(AppState::Gameplay), fade_background_out)
            .add_systems(
                Update,
                (
                    spawn_background_entities,
                    configure_background_image_sampler,
                    update_background_opacity,
                    resize_background_image,
                    animate_particles,
                )
                    .chain(),
            );
    }
}
