mod frame_buffer;
mod systems;

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::game_boy::frame_buffer::{GAME_BOY_FRAME_RATE_HZ, GameBoyFrameRing};
use crate::game_boy::systems::write_placeholder_game_boy_frame;

pub struct GameBoyPlugin;

impl Plugin for GameBoyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(GAME_BOY_FRAME_RATE_HZ))
            .init_resource::<GameBoyFrameRing>()
            .add_systems(
                FixedUpdate,
                write_placeholder_game_boy_frame.run_if(in_state(AppState::Gameplay)),
            );
    }
}
