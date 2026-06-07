mod app_assets;
mod app_state;
mod app_theme;
mod app_ui_scale;
mod audio;
mod background;
mod binary_text;
mod circuit_board;
mod dimensions;
mod game_boy;
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

#[inline]
pub fn run_app<T>(platform_plugin: T)
where
    T: Plugin,
{
    #[cfg(not(debug_assertions))]
    let asset_plugin = AssetPlugin {
        file_path: "assets".to_owned(),
        ..default()
    };
    #[cfg(debug_assertions)]
    let asset_plugin = AssetPlugin {
        file_path: "../../assets".to_owned(),
        ..default()
    };

    let mut app = App::new();
    app.insert_resource(ClearColor(Color::BLACK))
        .init_resource::<AppAssets>()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: WINDOW_TITLE.to_owned(),
                        fit_canvas_to_parent: true,
                        resolution: (1280, 720).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(asset_plugin),
            platform_plugin,
        ))
        .init_state::<AppState>()
        .add_plugins(storage::StoragePlugin)
        .add_plugins(app_ui_scale::AppUiScalePlugin)
        .init_resource::<ActiveTheme>()
        .add_systems(Startup, camera_scene.spawn())
        .add_plugins(ui_elements::action_hint::ActionHintPlugin)
        .add_plugins(scenes::audio_settings::AudioSettingsScenePlugin)
        .add_plugins(scenes::gameplay::GameplayScenePlugin)
        .add_plugins(game_boy::GameBoyPlugin)
        .add_plugins(audio::AudioPlugin)
        .add_plugins((
            input::InputPlugin,
            background::BackgroundPlugin,
            circuit_board::CircuitBoardPlugin,
            binary_text::BinaryTextPlugin,
            ui_elements::theme::UiThemePlugin,
            ui_elements::info_message::InfoMessagePlugin,
            ui_elements::interactions::UiInteractionsPlugin,
            ui_elements::responsive::ResponsiveUiPlugin,
            ui_elements::choice_popup::ChoicePopupPlugin,
            scenes::home::HomeScenePlugin,
            scenes::input_mapping::InputMappingScenePlugin,
            scenes::loading::LoadingScenePlugin,
            scenes::rom_provider::RomProviderScenePlugin,
            scenes::settings::SettingsScenePlugin,
            scenes::splash::SplashScenePlugin,
        ))
        .add_plugins(scenes::rom_data::RomDataScenePlugin)
        .run();
}

pub mod platform {
    use std::sync::{LazyLock, Mutex};

    pub use crate::ui_elements::file_picker::{
        UiAudioFilePicker, UiDirectoryPicker, UiFilePickerActivated, UiFilePickerResult,
    };
    pub use crate::ui_elements::interactions::{
        DisabledUiElement, EditableUiElement, FocusedUiElement, UiTextInput,
    };

    pub type AndroidLocalDirectoryReader =
        fn(&str) -> Result<Vec<AndroidLocalDirectoryRomFile>, String>;

    #[derive(Clone, Debug)]
    pub struct AndroidLocalDirectoryRomFile {
        pub file_name: String,
        pub bytes: Vec<u8>,
    }

    static ANDROID_LOCAL_DIRECTORY_READER: LazyLock<Mutex<Option<AndroidLocalDirectoryReader>>> =
        LazyLock::new(|| Mutex::new(None));

    pub fn set_android_local_directory_reader(reader: AndroidLocalDirectoryReader) {
        let Ok(mut current) = ANDROID_LOCAL_DIRECTORY_READER.lock() else {
            eprintln!("failed to lock Android local directory reader");
            return;
        };
        *current = Some(reader);
    }

    pub fn read_android_local_directory_roms(
        uri: &str,
    ) -> Result<Vec<AndroidLocalDirectoryRomFile>, String> {
        let Ok(reader) = ANDROID_LOCAL_DIRECTORY_READER.lock() else {
            return Err("Android local directory reader is unavailable.".to_string());
        };
        let Some(reader) = *reader else {
            return Err("Android local directory reader is not registered.".to_string());
        };
        reader(uri)
    }
}

fn camera_scene() -> impl Scene {
    bsn! {
        Camera2d
    }
}
