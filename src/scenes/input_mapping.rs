use bevy::prelude::*;

use crate::app_state::AppState;

pub struct InputMappingScenePlugin;

impl Plugin for InputMappingScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InputMapping), spawn_input_mapping_scene);
    }
}

fn spawn_input_mapping_scene(mut commands: Commands) {
    commands.spawn_scene(input_mapping_scene());
}

fn input_mapping_scene() -> impl Scene {
    bsn! {
        #InputMappingScene
        DespawnOnExit::<AppState>(AppState::InputMapping)
        Node {
            width: percent(100),
            height: percent(100),
        }
    }
}
