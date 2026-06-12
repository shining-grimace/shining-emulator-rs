use bevy::prelude::*;

use crate::app_state::AppState;
use crate::game_boy::emulator::{GameBoyEmulator, GameBoyEmulatorBundle};

pub(crate) fn spawn_game_boy_emulator(
    mut commands: Commands,
    emulators: Query<(), With<GameBoyEmulator>>,
) {
    if !emulators.is_empty() {
        return;
    }

    commands.spawn((
        GameBoyEmulatorBundle::default(),
        DespawnOnExit::<AppState>(AppState::Gameplay),
    ));
}
