mod frame_buffer;
mod frame_renderer;
mod systems;

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::game_boy::frame_buffer::{GAME_BOY_FRAME_RATE_HZ, GameBoyFrameRing};
use crate::game_boy::frame_renderer::{
    GameBoyFrameTexture, resize_game_boy_frame_display, spawn_game_boy_frame_display,
    update_game_boy_frame_texture,
};
use crate::game_boy::systems::write_placeholder_game_boy_frame;

pub struct GameBoyPlugin;

impl Plugin for GameBoyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(GAME_BOY_FRAME_RATE_HZ))
            .init_resource::<GameBoyFrameRing>()
            .init_resource::<GameBoyFrameTexture>()
            .add_systems(OnEnter(AppState::Gameplay), spawn_game_boy_frame_display)
            .add_systems(
                FixedUpdate,
                write_placeholder_game_boy_frame.run_if(in_state(AppState::Gameplay)),
            )
            .add_systems(
                Update,
                (update_game_boy_frame_texture, resize_game_boy_frame_display)
                    .run_if(in_state(AppState::Gameplay)),
            );
    }
}
