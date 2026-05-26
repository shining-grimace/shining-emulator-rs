use bevy::prelude::*;
use bevy::ui::UiScale;

use crate::storage::LocalStorage;

pub const UI_SCALE_LABELS: [&str; 5] = ["80%", "90%", "100%", "110%", "120%"];

const UI_SCALE_VALUES: [f32; 5] = [0.8, 0.9, 1.0, 1.1, 1.2];

pub struct AppUiScalePlugin;

impl Plugin for AppUiScalePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiScale>()
            .add_systems(Startup, apply_stored_ui_scale);
    }
}

pub fn default_ui_scale() -> u8 {
    2
}

pub fn apply_ui_scale_setting(value: u8, ui_scale: &mut UiScale) {
    ui_scale.0 = ui_scale_for_setting(value);
}

fn apply_stored_ui_scale(storage: Res<LocalStorage>, mut ui_scale: ResMut<UiScale>) {
    apply_ui_scale_setting(storage.data.settings.ui_scale, &mut ui_scale);
}

fn ui_scale_for_setting(value: u8) -> f32 {
    UI_SCALE_VALUES
        .get(value as usize)
        .copied()
        .unwrap_or(UI_SCALE_VALUES[default_ui_scale() as usize])
}
