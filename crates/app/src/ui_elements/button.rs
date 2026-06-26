use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::dimensions::{UI_BUTTON_WIDTH, UI_CONTROL_FONT_SIZE, UI_ELEMENT_HEIGHT};
use crate::ui_elements::interactions::{
    IgnorePicking, UiElementColors, UiElementKind, UiElementLabel, UiFocusNav,
};
use crate::ui_elements::styles::{control_fill, hover_fill, ui_border, ui_radius};
use crate::ui_elements::theme::{UiElementTheme, UiThemeBorderColor, UiThemeTextColor};

pub fn button(
    font: Handle<Font>,
    label: impl Into<String>,
    theme: ActiveTheme,
    nav: UiFocusNav,
) -> impl Scene {
    button_with_width(font, label, theme, nav, px(UI_BUTTON_WIDTH))
}

pub fn button_with_width(
    font: Handle<Font>,
    label: impl Into<String>,
    theme: ActiveTheme,
    nav: UiFocusNav,
    width: Val,
) -> impl Scene {
    let label = label.into();
    let background = control_fill(&theme);
    let hover_background = hover_fill(&theme);
    bsn! {
        Node {
            width: {width},
            height: px(UI_ELEMENT_HEIGHT),
            border: ui_border(),
            border_radius: ui_radius(),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Button
        BorderColor::all(theme.primary)
        UiThemeBorderColor::Primary
        BackgroundColor({background})
        UiFocusNav { up: {nav.up}, right: {nav.right}, down: {nav.down}, left: {nav.left} }
        UiElementKind::Button
        UiElementTheme::Control
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: {background}, hover_fill: {hover_background} }
        Children [
            (
                Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                IgnorePicking
                Children [
                    (
                        Text({label})
                        TextFont {
                            font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                            font_size: px(UI_CONTROL_FONT_SIZE),
                        }
                        TextColor({theme.primary})
                        UiThemeTextColor::Primary
                        UiElementLabel
                        IgnorePicking
                        TextLayout::new(Justify::Center, LineBreak::NoWrap)
                    ),
                ]
            ),
        ]
    }
}
