use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::interactions::{
    AutoScrollFocusedChild, DismissOnOutsideClick, IgnorePicking, UiElementColors, UiElementKind,
    UiElementLabel, UiFocusNav, UiMultiSelect, UiMultiSelectLabel, UiMultiSelectOption,
    UiMultiSelectPopup, UiPopupScrollArea, UiScrollArea, UiScrollContent,
};
use crate::ui_elements::scrollbar::scrollbar_with_display;
use crate::ui_elements::styles::{
    UI_CONTROL_FONT_SIZE, UI_ELEMENT_HEIGHT, UI_INNER_PADDING, UI_MULTI_SELECT_WIDTH, control_fill,
    hover_fill, ui_border, ui_radius,
};
use crate::ui_elements::theme::{UiElementTheme, UiThemeBorderColor, UiThemeTextColor};

const MAX_VISIBLE_POPUP_OPTIONS: usize = 5;
const POPUP_OPTION_GAP: f32 = 8.0;
const POPUP_SCROLLBAR_GAP: f32 = 10.0;
const POPUP_SCROLLBAR_THUMB_HEIGHT: f32 = 56.0;

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
    let visible_options = config.options.len().min(MAX_VISIBLE_POPUP_OPTIONS).max(1);
    let scrollbar_display = if config.options.len() > MAX_VISIBLE_POPUP_OPTIONS {
        Display::Flex
    } else {
        Display::None
    };
    let popup_options_height = visible_options as f32 * UI_ELEMENT_HEIGHT
        + visible_options.saturating_sub(1) as f32 * POPUP_OPTION_GAP;
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
        .map(|(index, option)| {
            popup_option(font.clone(), index, option, theme, UiFocusNav::default())
        })
        .collect::<Vec<_>>();

    bsn! {
        Node {
            width: px(UI_MULTI_SELECT_WIDTH),
            flex_shrink: 0.0,
            height: px(UI_ELEMENT_HEIGHT),
            border: ui_border(),
            border_radius: ui_radius(),
            padding: UiRect::horizontal(px(UI_INNER_PADDING)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
        }
        Button
        BorderColor::all(theme.primary)
        UiThemeBorderColor::Primary
        BackgroundColor({background})
        UiFocusNav { up: {config.nav.up}, right: {config.nav.right}, down: {config.nav.down}, left: {config.nav.left} }
        UiElementKind::MultiSelect
        UiElementTheme::Control
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
                UiThemeTextColor::Primary
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
                }
                GlobalZIndex(100)
                BorderColor::all(theme.primary)
                UiThemeBorderColor::Primary
                BackgroundColor({popup_background})
                UiMultiSelectPopup { parent: {Entity::PLACEHOLDER} }
                Children [
                    (
                        Node {
                            width: percent(100),
                            height: px(popup_options_height),
                            flex_direction: FlexDirection::Row,
                            column_gap: px(POPUP_SCROLLBAR_GAP),
                            overflow: Overflow::clip(),
                        }
                        UiScrollArea { offset: 0.0, max_offset: 0.0 }
                        UiPopupScrollArea {
                            option_count: {config.options.len()},
                            max_visible_options: {MAX_VISIBLE_POPUP_OPTIONS},
                            option_height: {UI_ELEMENT_HEIGHT},
                            option_gap: {POPUP_OPTION_GAP},
                        }
                        AutoScrollFocusedChild
                        Children [
                            (
                                Node {
                                    position_type: PositionType::Relative,
                                    flex_grow: 1.0,
                                    height: percent(100),
                                    overflow: Overflow::clip(),
                                }
                                Children [
                                    (
                                        Node {
                                            position_type: PositionType::Absolute,
                                            top: px(0.0),
                                            left: px(0.0),
                                            width: percent(100),
                                            flex_direction: FlexDirection::Column,
                                            row_gap: px(POPUP_OPTION_GAP),
                                        }
                                        UiScrollContent
                                        Children [
                                            {options}
                                        ]
                                    )
                                ]
                            ),
                            (
                                Node {
                                    height: percent(100),
                                    overflow: Overflow::clip(),
                                    flex_direction: FlexDirection::Column,
                                }
                                Children [
                                    scrollbar_with_display(
                                        theme,
                                        POPUP_SCROLLBAR_THUMB_HEIGHT,
                                        0.0,
                                        scrollbar_display
                                    )
                                ]
                            ),
                        ]
                    )
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
        UiElementTheme::PopupOption
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
                UiThemeTextColor::Primary
                UiElementLabel
                IgnorePicking
                TextLayout::new(Justify::Left, LineBreak::NoWrap)
            )
        ]
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
