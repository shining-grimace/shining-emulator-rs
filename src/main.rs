mod app_assets;
mod app_state;
mod app_theme;
mod dimensions;
mod input;
mod scenes;
mod storage;
mod ui_elements;

use bevy::prelude::*;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;

const WINDOW_TITLE: &str = "Shining Emulator";

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<AppAssets>()
        .init_resource::<ActiveTheme>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: WINDOW_TITLE.to_string(),
                resolution: (1280, 720).into(),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        .add_systems(Startup, camera_scene.spawn())
        .add_plugins((
            storage::StoragePlugin,
            input::InputPlugin,
            scenes::loading::LoadingScenePlugin,
            scenes::splash::SplashScenePlugin,
        ))
        .run();
}

fn camera_scene() -> impl Scene {
    bsn! {
        Camera2d
    }
}
