use bevy::prelude::*;

use crate::app_theme::ActiveTheme;
use crate::dimensions::UI_HINT_MARGIN;
use crate::ui_elements::loading_indicator::{
    LOADING_INDICATOR_GRID_SIZE, LoadingIndicatorPlugin, spawn_loading_indicator,
};

pub struct LoadingScenePlugin;

impl Plugin for LoadingScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LoadingIndicatorPlugin)
            .add_systems(Startup, spawn_loading_scene);
    }
}

fn spawn_loading_scene(mut commands: Commands, theme: Res<ActiveTheme>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(UI_HINT_MARGIN),
                bottom: Val::Px(UI_HINT_MARGIN),
                width: Val::Px(LOADING_INDICATOR_GRID_SIZE),
                height: Val::Px(LOADING_INDICATOR_GRID_SIZE),
                ..default()
            },
            Name::new("Loading scene"),
        ))
        .with_children(|parent| spawn_loading_indicator(parent, &theme));
}
