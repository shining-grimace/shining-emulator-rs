use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::interactions::{
    IgnorePicking, UiElementColors, UiElementKind, UiElementLabel, UiFocusNav,
};
use crate::ui_elements::styles::{
    UI_BUTTON_WIDTH, UI_CONTROL_FONT_SIZE, UI_ELEMENT_HEIGHT, control_fill, hover_fill, ui_border,
    ui_radius,
};

pub fn button(
    font: Handle<Font>,
    label: &'static str,
    theme: ActiveTheme,
    nav: UiFocusNav,
) -> impl Scene {
    let background = control_fill(&theme);
    let hover_background = hover_fill(&theme);
    bsn! {
        Node {
            width: px(UI_BUTTON_WIDTH),
            height: px(UI_ELEMENT_HEIGHT),
            border: ui_border(),
            border_radius: ui_radius(),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Button
        BorderColor::all(theme.primary)
        BackgroundColor({background})
        UiFocusNav { up: {nav.up}, right: {nav.right}, down: {nav.down}, left: {nav.left} }
        UiElementKind::Button
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: {background}, hover_fill: {hover_background} }
        Children [
            (
                Text({label})
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_CONTROL_FONT_SIZE),
                }
                TextColor({theme.primary})
                UiElementLabel
                IgnorePicking
                TextLayout::new(Justify::Center, LineBreak::NoWrap)
            )
        ]
    }
}
