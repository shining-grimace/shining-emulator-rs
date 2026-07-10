mod components;
mod constants;
mod effects;
mod systems;

use bevy::prelude::*;

use crate::binary_text::effects::BinaryTextEffects;
use crate::binary_text::systems::{
    animate_binary_text, spawn_binary_text_pool, update_binary_text_grid,
};
use crate::settings_transition::SettingsTransitionTimeline;

pub struct BinaryTextPlugin;

impl Plugin for BinaryTextPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BinaryTextEffects>().add_systems(
            Update,
            (
                update_binary_text_grid,
                spawn_binary_text_pool,
                animate_binary_text,
            )
                .chain()
                .after(SettingsTransitionTimeline),
        );
    }
}
