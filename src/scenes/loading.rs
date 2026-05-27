use bevy::prelude::*;

use crate::app_assets::{
    AppAssets, HEROES_PATH, ICONS_PATH, SHINING_EMULATOR_LOGO_PATH, SHINING_GRIMACE_LOGO_PATH,
    UBUNTU_MONO_FONT_PATH,
};
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::dimensions::UI_HINT_MARGIN;
use crate::ui_elements::loading_indicator::{
    LOADING_INDICATOR_GRID_SIZE, LoadingIndicatorPlugin, loading_indicator_scene,
};

pub struct LoadingScenePlugin;

impl Plugin for LoadingScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LoadingIndicatorPlugin)
            .add_systems(
                OnEnter(AppState::Loading),
                (request_assets, spawn_loading_scene).chain(),
            )
            .add_systems(
                Update,
                check_assets_loaded.run_if(in_state(AppState::Loading)),
            );
    }
}

fn request_assets(
    asset_server: Res<AssetServer>,
    theme: Res<ActiveTheme>,
    mut assets: ResMut<AppAssets>,
) {
    assets.shining_grimace_logo = asset_server.load(SHINING_GRIMACE_LOGO_PATH);
    assets.shining_emulator_logo = asset_server.load(SHINING_EMULATOR_LOGO_PATH);
    assets.ubuntu_mono_font = asset_server.load(UBUNTU_MONO_FONT_PATH);
    assets.icons = asset_server.load(ICONS_PATH);
    assets.heroes = asset_server.load(HEROES_PATH);
    assets.theme_background = theme
        .background_asset_path
        .map(|path| asset_server.load(path));
}

fn check_assets_loaded(
    asset_server: Res<AssetServer>,
    assets: Res<AppAssets>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if required_assets_loaded(&asset_server, &assets) {
        next_state.set(AppState::Splash);
    }
}

fn required_assets_loaded(asset_server: &AssetServer, assets: &AppAssets) -> bool {
    asset_server.is_loaded(&assets.shining_grimace_logo)
        && asset_server.is_loaded(&assets.shining_emulator_logo)
        && asset_server.is_loaded(&assets.ubuntu_mono_font)
        && asset_server.is_loaded(&assets.icons)
        && asset_server.is_loaded(&assets.heroes)
        && assets
            .theme_background
            .as_ref()
            .is_none_or(|handle| asset_server.is_loaded(handle))
}

fn spawn_loading_scene(mut commands: Commands, theme: Res<ActiveTheme>) {
    commands.spawn_scene(loading_scene(*theme));
}

fn loading_scene(theme: ActiveTheme) -> impl Scene {
    bsn! {
        #LoadingScene
        DespawnOnExit::<AppState>(AppState::Loading)
        Node {
            position_type: PositionType::Absolute,
            right: px(UI_HINT_MARGIN),
            bottom: px(UI_HINT_MARGIN),
            width: px(LOADING_INDICATOR_GRID_SIZE),
            height: px(LOADING_INDICATOR_GRID_SIZE),
        }
        Children [
            {bsn_list![loading_indicator_scene(theme)]}
        ]
    }
}
