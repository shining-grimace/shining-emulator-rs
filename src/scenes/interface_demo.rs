use bevy::prelude::*;

use crate::app_state::AppState;

const INTERFACE_DEMO_TRANSITION_SECONDS: f32 = 2.0;

pub struct InterfaceDemoScenePlugin;

impl Plugin for InterfaceDemoScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InterfaceDemo), spawn_interface_demo_scene)
            .add_systems(
                Update,
                transition_to_input_mapping.run_if(in_state(AppState::InterfaceDemo)),
            );
    }
}

fn spawn_interface_demo_scene(mut commands: Commands) {
    commands.insert_resource(InterfaceDemoTimer(Timer::from_seconds(
        INTERFACE_DEMO_TRANSITION_SECONDS,
        TimerMode::Once,
    )));
    commands.spawn_scene(interface_demo_scene());
}

#[derive(Resource)]
struct InterfaceDemoTimer(Timer);

fn transition_to_input_mapping(
    time: Res<Time>,
    mut timer: ResMut<InterfaceDemoTimer>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        next_state.set(AppState::InputMapping);
    }
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
