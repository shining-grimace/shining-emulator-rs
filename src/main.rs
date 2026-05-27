mod app_assets;
mod app_state;
mod app_theme;
mod app_ui_scale;
mod background;
mod binary_text;
mod circuit_board;
mod dimensions;
mod input;
mod scenes;
mod storage;
mod ui_elements;
mod visual_effects;

use bevy::prelude::*;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;

const WINDOW_TITLE: &str = "Shining Emulator";

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<AppAssets>()
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
        .add_plugins(storage::StoragePlugin)
        .add_plugins(app_ui_scale::AppUiScalePlugin)
        .init_resource::<ActiveTheme>()
        .add_systems(Startup, camera_scene.spawn())
        .add_plugins((
            input::InputPlugin,
            background::BackgroundPlugin,
            circuit_board::CircuitBoardPlugin,
            binary_text::BinaryTextPlugin,
            ui_elements::theme::UiThemePlugin,
            ui_elements::info_message::InfoMessagePlugin,
            ui_elements::interactions::UiInteractionsPlugin,
            ui_elements::choice_popup::ChoicePopupPlugin,
            scenes::home::HomeScenePlugin,
            scenes::interface_demo::InterfaceDemoScenePlugin,
            scenes::input_mapping::InputMappingScenePlugin,
            scenes::loading::LoadingScenePlugin,
            scenes::rom_provider::RomProviderScenePlugin,
            scenes::settings::SettingsScenePlugin,
            scenes::splash::SplashScenePlugin,
        ))
        .run();
}

fn camera_scene() -> impl Scene {
    bsn! {
        Camera2d
    }
}
