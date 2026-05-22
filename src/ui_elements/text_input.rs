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

pub fn text_input(
    font: Handle<Font>,
    label: &'static str,
    theme: ActiveTheme,
    nav: UiFocusNav,
) -> impl Scene {
    let background = control_fill(&theme);
    let hover_background = hover_fill(&theme);
    bsn! {
        Node {
            width: px(UI_TEXT_INPUT_WIDTH),
            height: px(UI_ELEMENT_HEIGHT),
            border: ui_border(),
            border_radius: ui_radius(),
            padding: UiRect::horizontal(px(UI_INNER_PADDING)),
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
        }
        Button
        BorderColor::all(theme.primary)
        BackgroundColor({background})
        UiFocusNav { up: {nav.up}, right: {nav.right}, down: {nav.down}, left: {nav.left} }
        UiElementKind::TextInput
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: {background}, hover_fill: {hover_background} }
        UiTextInput { value: {String::new()}, placeholder: {label.to_string()}, cursor: 0 }
        EditableUiElement
        Children [
            (
                Text({label})
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_CONTROL_FONT_SIZE),
                }
                TextColor({theme.tertiary})
                UiElementLabel
                IgnorePicking
                UiTextInputText
                TextLayout::new(Justify::Left, LineBreak::NoWrap)
            )
        ]
    }
}
