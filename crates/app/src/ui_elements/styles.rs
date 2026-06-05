use bevy::color::Alpha;
use bevy::prelude::*;

use crate::app_theme::ActiveTheme;
pub const UI_FILL_ALPHA: f32 = 0.20;
use crate::dimensions::{UI_BORDER_WIDTH, UI_CORNER_RADIUS, UI_INNER_PADDING};

pub fn ui_radius() -> BorderRadius {
    BorderRadius::all(px(UI_CORNER_RADIUS))
}

pub fn ui_border() -> UiRect {
    UiRect::all(px(UI_BORDER_WIDTH))
}

pub fn ui_padding() -> UiRect {
    UiRect::all(px(UI_INNER_PADDING))
}

pub fn control_fill(theme: &ActiveTheme) -> Color {
    theme.primary.with_alpha(UI_FILL_ALPHA)
}

pub fn hover_fill(theme: &ActiveTheme) -> Color {
    theme.secondary.with_alpha(UI_FILL_ALPHA)
}

pub fn transparent() -> Color {
    Color::NONE
}
