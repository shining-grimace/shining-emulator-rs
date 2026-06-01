use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::interactions::IgnorePicking;
use crate::ui_elements::styles::UI_HEADING_FONT_SIZE;
use crate::ui_elements::theme::UiThemeTextColor;

pub fn heading(font: Handle<Font>, theme: ActiveTheme, label: impl Into<String>) -> impl Scene {
    let label = label.into();
    bsn! {
        Text({label})
        TextFont {
            font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
            font_size: px(UI_HEADING_FONT_SIZE),
        }
        TextColor({theme.primary})
        UiThemeTextColor::Primary
        IgnorePicking
        TextLayout::new(Justify::Left, LineBreak::WordBoundary)
    }
}
