use bevy::prelude::*;
use bevy::ui::UiScale;

use crate::storage::LocalStorage;

pub const FONT_SIZE_LABELS: [&str; 5] = ["14 pt", "16 pt", "18 pt", "20 pt", "22 pt"];

const FONT_SIZE_SCALE_VALUES: [f32; 5] = [0.7, 0.8, 0.9, 1.0, 1.1];

pub struct AppUiScalePlugin;

impl Plugin for AppUiScalePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiScale>()
            .add_systems(Startup, apply_stored_font_size);
    }
}

pub fn default_font_size() -> u8 {
    2
}

pub fn apply_font_size_setting(value: u8, ui_scale: &mut UiScale) {
    ui_scale.0 = font_size_scale_for_setting(value);
}

fn apply_stored_font_size(storage: Res<LocalStorage>, mut ui_scale: ResMut<UiScale>) {
    apply_font_size_setting(storage.data.settings.font_size, &mut ui_scale);
}

fn font_size_scale_for_setting(value: u8) -> f32 {
    FONT_SIZE_SCALE_VALUES
        .get(value as usize)
        .copied()
        .unwrap_or(FONT_SIZE_SCALE_VALUES[default_font_size() as usize])
}
