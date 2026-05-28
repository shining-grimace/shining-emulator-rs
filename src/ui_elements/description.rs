use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::interactions::{IgnorePicking, UiElementLabel};
use crate::ui_elements::styles::UI_BODY_FONT_SIZE;
use crate::ui_elements::theme::UiThemeTextColor;

pub fn description(font: Handle<Font>, theme: ActiveTheme, text: impl Into<String>) -> impl Scene {
    let text = text.into();
    bsn! {
        Text({text})
        TextFont {
            font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
            font_size: px(UI_BODY_FONT_SIZE),
        }
        TextColor({theme.primary})
        UiThemeTextColor::Primary
        UiElementLabel
        IgnorePicking
        TextLayout::new(Justify::Left, LineBreak::WordBoundary)
    }
}
