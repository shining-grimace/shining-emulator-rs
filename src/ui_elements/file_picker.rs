use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::interactions::{
    DisabledUiElement, HoveredUiElement, IgnorePicking, UiElementColors, UiElementKind,
    UiElementLabel, UiFocusNav, tree::contains_entity,
};
use crate::ui_elements::styles::{
    UI_BUTTON_WIDTH, UI_CONTROL_FONT_SIZE, UI_ELEMENT_HEIGHT, UI_FILE_PICKER_WIDTH,
    UI_INNER_PADDING, control_fill, hover_fill, ui_border, ui_radius,
};

const FILE_PICKER_VISIBLE_CHARS: usize = 20;
const FILE_PICKER_GAP: f32 = 12.0;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiFilePicker;

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

pub fn file_picker(
    font: Handle<Font>,
    value: &'static str,
    theme: ActiveTheme,
    nav: UiFocusNav,
) -> impl Scene {
    let value_font = font.clone();
    let background = control_fill(&theme);
    let hover_background = hover_fill(&theme);
    bsn! {
        #FilePickerRoot
        Node {
            width: px(UI_FILE_PICKER_WIDTH),
            height: px(UI_ELEMENT_HEIGHT),
            flex_direction: FlexDirection::Row,
            column_gap: px(FILE_PICKER_GAP),
        }
        Button
        UiFocusNav { up: {nav.up}, right: {nav.right}, down: {nav.down}, left: {nav.left} }
        UiElementKind::Button
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: Color::NONE, hover_fill: Color::NONE }
        UiFilePicker
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
                        Text({value})
                        TextFont {
                            font: FontSourceTemplate::Handle(HandleTemplate::Handle(value_font)),
                            font_size: px(UI_CONTROL_FONT_SIZE),
                        }
                        TextColor({theme.tertiary})
                        TextLayout::new(Justify::Left, LineBreak::NoWrap)
                        IgnorePicking
                        UiFilePickerValue { picker: #FilePickerRoot }
                    )
                ]
            ),
            (
                #FilePickerButton
                Node {
                    width: px(UI_BUTTON_WIDTH),
                    height: px(UI_ELEMENT_HEIGHT),
                    border: ui_border(),
                    border_radius: ui_radius(),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                BorderColor::all(theme.primary)
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
                background.0 = if hovered { fill.hover_fill } else { fill.fill };
            }
        }
    }
}

pub(crate) fn drain_file_picker_activations(
    mut activations: MessageReader<UiFilePickerActivated>,
    mut values: Query<(Entity, &UiFilePickerValue, &mut Text)>,
    child_query: Query<&Children>,
) {
    for activation in activations.read() {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Choose GameBoy ROM (*.gb, *.gbc)")
            .add_filter("GameBoy ROM", &["gb", "gbc"])
            .pick_file()
        else {
            continue;
        };

        let path = path.canonicalize().unwrap_or(path);
        let path = path.display().to_string();
        for (entity, value, mut text) in &mut values {
            if value.picker == activation.picker
                || child_query
                    .get(activation.picker)
                    .is_ok_and(|children| contains_entity(children, entity, &child_query))
            {
                text.0 = trailing_text(&path, FILE_PICKER_VISIBLE_CHARS);
            }
        }
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
