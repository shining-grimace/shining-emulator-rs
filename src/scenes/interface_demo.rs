use bevy::prelude::*;

use crate::app_state::AppState;

pub struct InterfaceDemoScenePlugin;

impl Plugin for InterfaceDemoScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InterfaceDemo), spawn_interface_demo_scene);
    }
}

fn spawn_interface_demo_scene(mut commands: Commands) {
    commands.spawn_scene(interface_demo_scene());
}

fn interface_demo_scene() -> impl Scene {
    bsn! {
        #InterfaceDemoScene
        DespawnOnExit::<AppState>(AppState::InterfaceDemo)
        Node {
            width: percent(100),
            height: percent(100),
        }
    }
}
