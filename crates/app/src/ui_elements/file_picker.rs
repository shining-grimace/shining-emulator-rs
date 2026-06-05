use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::dimensions::{
    FILE_PICKER_GAP, UI_BUTTON_WIDTH, UI_CONTROL_FONT_SIZE, UI_ELEMENT_HEIGHT,
    UI_FILE_PICKER_WIDTH, UI_INNER_PADDING,
};
use crate::ui_elements::interactions::{
    DisabledUiElement, HoveredUiElement, IgnorePicking, UiElementColors, UiElementKind,
    UiElementLabel, UiFocusNav, tree::contains_entity,
};
use crate::ui_elements::styles::{control_fill, hover_fill, ui_border, ui_radius};
use crate::ui_elements::theme::{UiElementTheme, UiThemeBorderColor, UiThemeTextColor};

const FILE_PICKER_VISIBLE_CHARS: usize = 20;

#[derive(Clone, Component, Debug, Default, FromTemplate)]
pub struct UiFilePicker {
    pub value: String,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiDirectoryPicker;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiAudioFilePicker;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiFilePickerValue {
    pub picker: Entity,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiFilePickerHoverFill {
    pub fill: Color,
    pub hover_fill: Color,
}

#[derive(Clone, Copy, Debug, Message)]
pub struct UiFilePickerActivated {
    pub picker: Entity,
}

#[derive(Clone, Debug, Message)]
pub struct UiFilePickerResult {
    pub picker: Entity,
    pub value: String,
}

pub fn file_picker_with_value(
    font: Handle<Font>,
    placeholder: &'static str,
    value: String,
    theme: ActiveTheme,
    nav: UiFocusNav,
) -> impl Scene {
    let value_font = font.clone();
    let background = control_fill(&theme);
    let hover_background = hover_fill(&theme);
    let display_value = picker_display_value(placeholder, &value);

    bsn! {
        Node {
            width: px(UI_FILE_PICKER_WIDTH),
            height: px(UI_ELEMENT_HEIGHT),
            flex_direction: FlexDirection::Row,
            column_gap: px(FILE_PICKER_GAP),
        }
        Button
        UiFocusNav { up: {nav.up}, right: {nav.right}, down: {nav.down}, left: {nav.left} }
        UiElementKind::Button
        UiElementTheme::TransparentControl
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: Color::NONE, hover_fill: Color::NONE }
        UiFilePicker { value: {value} }
        Children [
            (
                Node {
                    width: px(UI_FILE_PICKER_WIDTH - UI_BUTTON_WIDTH - FILE_PICKER_GAP),
                    flex_shrink: 0.0,
                    height: px(UI_ELEMENT_HEIGHT),
                    border: ui_border(),
                    border_radius: ui_radius(),
                    padding: UiRect::horizontal(px(UI_INNER_PADDING)),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                }
                BorderColor { top: Color::NONE, right: Color::NONE, bottom: Color::NONE, left: Color::NONE }
                BackgroundColor({background})
                UiFilePickerHoverFill { fill: {background}, hover_fill: {hover_background} }
                DisabledUiElement
                Children [
                    (
                        Text({display_value})
                        TextFont {
                            font: FontSourceTemplate::Handle(HandleTemplate::Handle(value_font)),
                            font_size: px(UI_CONTROL_FONT_SIZE),
                        }
                        TextColor({theme.tertiary})
                        UiThemeTextColor::Tertiary
                        TextLayout::new(Justify::Left, LineBreak::NoWrap)
                        IgnorePicking
                        UiFilePickerValue { picker: Entity::PLACEHOLDER }
                    )
                ]
            ),
            (
                Node {
                    width: px(UI_BUTTON_WIDTH),
                    height: px(UI_ELEMENT_HEIGHT),
                    border: ui_border(),
                    border_radius: ui_radius(),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                BorderColor::all(theme.primary)
                UiThemeBorderColor::Primary
                BackgroundColor({background})
                UiFilePickerHoverFill { fill: {background}, hover_fill: {hover_background} }
                IgnorePicking
                Children [
                    (
                        Text("Browse Files")
                        TextFont {
                            font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                            font_size: px(UI_CONTROL_FONT_SIZE),
                        }
                        TextColor({theme.primary})
                        UiThemeTextColor::Primary
                        UiElementLabel
                        IgnorePicking
                        TextLayout::new(Justify::Center, LineBreak::NoWrap)
                    )
                ]
            )
        ]
    }
}

pub fn directory_picker_with_value(
    font: Handle<Font>,
    placeholder: &'static str,
    value: String,
    theme: ActiveTheme,
    nav: UiFocusNav,
) -> impl Scene {
    let value_font = font.clone();
    let background = control_fill(&theme);
    let hover_background = hover_fill(&theme);
    let display_value = picker_display_value(placeholder, &value);

    bsn! {
        Node {
            width: percent(100),
            height: px(UI_ELEMENT_HEIGHT),
            flex_direction: FlexDirection::Row,
            column_gap: px(FILE_PICKER_GAP),
        }
        Button
        UiFocusNav { up: {nav.up}, right: {nav.right}, down: {nav.down}, left: {nav.left} }
        UiElementKind::Button
        UiElementTheme::TransparentControl
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: Color::NONE, hover_fill: Color::NONE }
        UiFilePicker { value: {value} }
        UiDirectoryPicker
        Children [
            (
                Node {
                    width: px(0.0),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    height: px(UI_ELEMENT_HEIGHT),
                    border: ui_border(),
                    border_radius: ui_radius(),
                    padding: UiRect::horizontal(px(UI_INNER_PADDING)),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                }
                BorderColor { top: Color::NONE, right: Color::NONE, bottom: Color::NONE, left: Color::NONE }
                BackgroundColor({background})
                UiFilePickerHoverFill { fill: {background}, hover_fill: {hover_background} }
                DisabledUiElement
                Children [
                    (
                        Text({display_value})
                        TextFont {
                            font: FontSourceTemplate::Handle(HandleTemplate::Handle(value_font)),
                            font_size: px(UI_CONTROL_FONT_SIZE),
                        }
                        TextColor({theme.tertiary})
                        UiThemeTextColor::Tertiary
                        TextLayout::new(Justify::Left, LineBreak::NoWrap)
                        IgnorePicking
                        UiFilePickerValue { picker: Entity::PLACEHOLDER }
                    )
                ]
            ),
            (
                Node {
                    width: px(UI_BUTTON_WIDTH),
                    height: px(UI_ELEMENT_HEIGHT),
                    border: ui_border(),
                    border_radius: ui_radius(),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                BorderColor::all(theme.primary)
                UiThemeBorderColor::Primary
                BackgroundColor({background})
                UiFilePickerHoverFill { fill: {background}, hover_fill: {hover_background} }
                IgnorePicking
                Children [
                    (
                        Text("Browse Folder")
                        TextFont {
                            font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                            font_size: px(UI_CONTROL_FONT_SIZE),
                        }
                        TextColor({theme.primary})
                        UiThemeTextColor::Primary
                        UiElementLabel
                        IgnorePicking
                        TextLayout::new(Justify::Center, LineBreak::NoWrap)
                    )
                ]
            )
        ]
    }
}

pub(crate) fn update_file_picker_hover_colours(
    pickers: Query<(Entity, Has<HoveredUiElement>, &Children), With<UiFilePicker>>,
    mut hover_fills: Query<(Entity, &UiFilePickerHoverFill, &mut BackgroundColor)>,
    child_query: Query<&Children>,
) {
    for (picker_entity, hovered, picker_children) in &pickers {
        for (entity, fill, mut background) in &mut hover_fills {
            if picker_entity == entity || contains_entity(picker_children, entity, &child_query) {
                let next_background = if hovered { fill.hover_fill } else { fill.fill };
                if background.0 != next_background {
                    background.0 = next_background;
                }
            }
        }
    }
}

pub(crate) fn apply_file_picker_results(
    mut results: MessageReader<UiFilePickerResult>,
    mut pickers: Query<(
        &mut UiFilePicker,
        Has<UiDirectoryPicker>,
        Has<UiAudioFilePicker>,
        Option<&Children>,
    )>,
    mut values: Query<(Entity, &UiFilePickerValue, &mut Text)>,
    child_query: Query<&Children>,
) {
    for result in results.read() {
        let Ok((mut picker, _, _, picker_children)) = pickers.get_mut(result.picker) else {
            continue;
        };
        apply_file_picker_value(
            result.picker,
            &mut picker,
            picker_children,
            &result.value,
            &mut values,
            &child_query,
        );
    }
}

fn apply_file_picker_value(
    picker_entity: Entity,
    picker: &mut UiFilePicker,
    picker_children: Option<&Children>,
    value: &str,
    values: &mut Query<(Entity, &UiFilePickerValue, &mut Text)>,
    child_query: &Query<&Children>,
) {
    picker.value = value.to_string();
    for (entity, picker_value, mut text) in values {
        if picker_value.picker == picker_entity
            || picker_children
                .is_some_and(|children| contains_entity(children, entity, child_query))
        {
            text.0 = trailing_text(value, FILE_PICKER_VISIBLE_CHARS);
        }
    }
}

fn picker_display_value(placeholder: &str, value: &str) -> String {
    if value.trim().is_empty() {
        placeholder.to_string()
    } else {
        trailing_text(value, FILE_PICKER_VISIBLE_CHARS)
    }
}

fn trailing_text(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars || max_chars <= 3 {
        return value.to_string();
    }

    format!(
        "...{}",
        value
            .chars()
            .skip(char_count - (max_chars - 3))
            .collect::<String>()
    )
}
