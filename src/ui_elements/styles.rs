use bevy::color::Alpha;
use bevy::prelude::*;

use crate::app_theme::ActiveTheme;

pub const UI_CORNER_RADIUS: f32 = 8.0;
pub const UI_FILL_ALPHA: f32 = 0.20;
pub const UI_BORDER_WIDTH: f32 = 2.0;
pub const UI_ELEMENT_HEIGHT: f32 = 48.0;
pub const UI_LIST_ROW_HEIGHT: f32 = UI_ELEMENT_HEIGHT;
pub const UI_BUTTON_WIDTH: f32 = 208.0;
pub const UI_TEXT_INPUT_WIDTH: f32 = 360.0;
pub const UI_MULTI_SELECT_WIDTH: f32 = 260.0;
pub const UI_FILE_PICKER_WIDTH: f32 = 480.0;
pub const UI_LIST_HEIGHT: f32 = 248.0;
pub const UI_SCROLLBAR_WIDTH: f32 = 10.0;
pub const UI_INNER_PADDING: f32 = 14.0;
pub const UI_PANEL_GAP: f32 = 18.0;
pub const UI_SCREEN_PADDING: f32 = 52.0;
pub const UI_MAX_CONTENT_WIDTH: f32 = 1100.0;
pub const UI_HEADING_FONT_SIZE: f32 = 32.0;
pub const UI_BODY_FONT_SIZE: f32 = 20.0;
pub const UI_CONTROL_FONT_SIZE: f32 = 22.0;

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
