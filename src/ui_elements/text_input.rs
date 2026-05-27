use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::interactions::{
    EditableUiElement, IgnorePicking, UiElementColors, UiElementKind, UiElementLabel, UiFocusNav,
    UiTextInput, UiTextInputText,
};
use crate::ui_elements::styles::{
    UI_CONTROL_FONT_SIZE, UI_ELEMENT_HEIGHT, UI_INNER_PADDING, UI_TEXT_INPUT_WIDTH, control_fill,
    hover_fill, ui_border, ui_radius,
};
use crate::ui_elements::theme::{UiElementTheme, UiThemeBorderColor, UiThemeTextColor};

pub fn text_input(
    font: Handle<Font>,
    label: &'static str,
    theme: ActiveTheme,
    nav: UiFocusNav,
) -> impl Scene {
    text_input_with_value(font, label, String::new(), theme, nav)
}

pub fn text_input_with_value(
    font: Handle<Font>,
    label: &'static str,
    value: String,
    theme: ActiveTheme,
    nav: UiFocusNav,
) -> impl Scene {
    text_input_with_value_width(font, label, value, theme, nav, px(UI_TEXT_INPUT_WIDTH))
}

pub fn text_input_with_value_width(
    font: Handle<Font>,
    label: &'static str,
    value: String,
    theme: ActiveTheme,
    nav: UiFocusNav,
    width: Val,
) -> impl Scene {
    let background = control_fill(&theme);
    let hover_background = hover_fill(&theme);
    let cursor = value.len();
    bsn! {
        Node {
            width: {width},
            flex_shrink: 1.0,
            height: px(UI_ELEMENT_HEIGHT),
            border: ui_border(),
            border_radius: ui_radius(),
            padding: UiRect::horizontal(px(UI_INNER_PADDING)),
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
        }
        Button
        BorderColor::all(theme.primary)
        UiThemeBorderColor::Primary
        BackgroundColor({background})
        UiFocusNav { up: {nav.up}, right: {nav.right}, down: {nav.down}, left: {nav.left} }
        UiElementKind::TextInput
        UiElementTheme::Control
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: {background}, hover_fill: {hover_background} }
        UiTextInput { value: {value}, placeholder: {label.to_string()}, cursor: {cursor} }
        EditableUiElement
        Children [
            (
                Text({label})
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_CONTROL_FONT_SIZE),
                }
                TextColor({theme.tertiary})
                UiThemeTextColor::Tertiary
                UiElementLabel
                IgnorePicking
                UiTextInputText
                TextLayout::new(Justify::Left, LineBreak::NoWrap)
            )
        ]
    }
}
