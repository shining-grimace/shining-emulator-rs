use bevy::prelude::*;

use crate::app_theme::ActiveTheme;
use crate::dimensions::UI_HINT_MARGIN;
use crate::ui_elements::loading_indicator::{
    LOADING_INDICATOR_GRID_SIZE, LoadingIndicatorPlugin, loading_indicator_scene,
};

pub struct LoadingScenePlugin;

impl Plugin for LoadingScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LoadingIndicatorPlugin)
            .add_systems(Startup, spawn_loading_scene);
    }
}

fn spawn_loading_scene(mut commands: Commands, theme: Res<ActiveTheme>) {
    commands.spawn_scene(loading_scene(*theme));
}

fn loading_scene(theme: ActiveTheme) -> impl Scene {
    bsn! {
        #LoadingScene
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
