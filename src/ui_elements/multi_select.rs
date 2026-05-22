use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::interactions::{
    DismissOnOutsideClick, IgnorePicking, UiElementColors, UiElementKind, UiElementLabel,
    UiFocusNav, UiMultiSelect, UiMultiSelectLabel, UiMultiSelectOption, UiMultiSelectPopup,
};
use crate::ui_elements::styles::{
    UI_CONTROL_FONT_SIZE, UI_ELEMENT_HEIGHT, UI_INNER_PADDING, UI_MULTI_SELECT_WIDTH, control_fill,
    hover_fill, ui_border, ui_radius,
};

pub struct MultiSelectConfig {
    pub selected: usize,
    pub options: Vec<&'static str>,
    pub nav: UiFocusNav,
}

pub fn multi_select(
    font: Handle<Font>,
    theme: ActiveTheme,
    config: MultiSelectConfig,
) -> impl Scene {
    let label_font = font.clone();
    let background = control_fill(&theme);
    let hover_background = hover_fill(&theme);
    let popup_background = Color::BLACK;
    let selected_label = config
        .options
        .get(config.selected)
        .copied()
        .unwrap_or_default()
        .to_string();
    let options = config
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| popup_option(font.clone(), index, option, theme, empty_focus_nav()))
        .collect::<Vec<_>>();

    bsn! {
        Node {
            width: px(UI_MULTI_SELECT_WIDTH),
            height: px(UI_ELEMENT_HEIGHT),
            border: ui_border(),
            border_radius: ui_radius(),
            padding: UiRect::horizontal(px(UI_INNER_PADDING)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
        }
        Button
        BorderColor::all(theme.primary)
        BackgroundColor({background})
        UiFocusNav { up: {config.nav.up}, right: {config.nav.right}, down: {config.nav.down}, left: {config.nav.left} }
        UiElementKind::MultiSelect
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: {background}, hover_fill: {hover_background} }
        UiMultiSelect { selected: {config.selected} }
        DismissOnOutsideClick
        Children [
            (
                Text({selected_label})
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(label_font)),
                    font_size: px(UI_CONTROL_FONT_SIZE),
                }
                TextColor({theme.primary})
                UiElementLabel
                IgnorePicking
                UiMultiSelectLabel
                TextLayout::new(Justify::Left, LineBreak::NoWrap)
            ),
            chevron_icon(theme.primary),
            (
                Node {
                    display: Display::None,
                    position_type: PositionType::Absolute,
                    top: px(UI_ELEMENT_HEIGHT + 8.0),
                    left: px(0.0),
                    width: px(UI_MULTI_SELECT_WIDTH),
                    border: ui_border(),
                    border_radius: ui_radius(),
                    padding: UiRect::all(px(UI_INNER_PADDING)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8.0),
                }
                GlobalZIndex(100)
                BorderColor::all(theme.primary)
                BackgroundColor({popup_background})
                UiMultiSelectPopup { parent: {Entity::PLACEHOLDER} }
                Children [
                    {options}
                ]
            ),
        ]
    }
}

fn popup_option(
    font: Handle<Font>,
    option_index: usize,
    label: &'static str,
    theme: ActiveTheme,
    nav: UiFocusNav,
) -> impl Scene {
    let hover_background = hover_fill(&theme);
    let popup_background = Color::BLACK;
    bsn! {
        Node {
            width: percent(100),
            height: px(UI_ELEMENT_HEIGHT),
            padding: UiRect::horizontal(px(UI_INNER_PADDING)),
            align_items: AlignItems::Center,
        }
        Button
        GlobalZIndex(101)
        BackgroundColor({popup_background})
        UiFocusNav { up: {nav.up}, right: {nav.right}, down: {nav.down}, left: {nav.left} }
        UiElementKind::MultiSelectOption
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: {popup_background}, hover_fill: {hover_background} }
        UiMultiSelectOption { option_index: {option_index}, label: {label.to_string()} }
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
                TextLayout::new(Justify::Left, LineBreak::NoWrap)
            )
        ]
    }
}

fn empty_focus_nav() -> UiFocusNav {
    UiFocusNav {
        up: Entity::PLACEHOLDER,
        right: Entity::PLACEHOLDER,
        down: Entity::PLACEHOLDER,
        left: Entity::PLACEHOLDER,
    }
}

fn chevron_icon(colour: Color) -> impl Scene {
    bsn! {
        Node {
            width: px(16.0),
            height: px(10.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        IgnorePicking
        Children [
            (
                Node {
                    width: px(0.0),
                    height: px(0.0),
                    border: UiRect {
                        left: px(8.0),
                        right: px(8.0),
                        top: px(10.0),
                        bottom: px(0.0),
                    },
                }
                BorderColor {
                    top: {colour},
                    right: Color::NONE,
                    bottom: Color::NONE,
                    left: Color::NONE,
                }
                BackgroundColor(Color::NONE)
                IgnorePicking
            )
        ]
    }
}
