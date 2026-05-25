use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::interactions::{
    AutoScrollFocusedChild, IgnorePicking, UiElementAccent, UiElementColors, UiElementKind,
    UiElementLabel, UiFocusNav, UiListCellText, UiListViewFocus, UiScrollArea, UiScrollContent,
};
use crate::ui_elements::scrollbar::scrollbar;
use crate::ui_elements::styles::{
    UI_BODY_FONT_SIZE, UI_LIST_HEIGHT, UI_LIST_ROW_HEIGHT, control_fill, hover_fill, transparent,
    ui_border, ui_padding, ui_radius,
};

#[derive(Clone, Copy)]
pub struct ListColumn {
    pub heading: &'static str,
    pub width_percent: f32,
}

pub struct ListRow {
    pub cells: Vec<&'static str>,
    pub nav: UiFocusNav,
}

pub struct ListViewConfig {
    pub nav: UiFocusNav,
    pub scrollbar_nav: UiFocusNav,
    pub columns: Vec<ListColumn>,
    pub rows: Vec<ListRow>,
}

pub fn list_view(font: Handle<Font>, theme: ActiveTheme, config: ListViewConfig) -> impl Scene {
    let columns = config.columns;
    let background = transparent();
    let control_background = control_fill(&theme);
    let hover_background = hover_fill(&theme);
    let header = list_header(font.clone(), theme, background, columns.clone());
    let rows = config
        .rows
        .into_iter()
        .map(|row| list_row(font.clone(), row, columns.clone(), theme, hover_background))
        .collect::<Vec<_>>();

    bsn! {
        Node {
            width: percent(100),
            height: px(UI_LIST_HEIGHT),
            min_height: px(0.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            border: ui_border(),
            border_radius: ui_radius(),
            padding: ui_padding(),
            flex_direction: FlexDirection::Column,
        }
        Button
        BorderColor::all(theme.primary)
        BackgroundColor({background})
        UiFocusNav { up: {config.nav.up}, right: {config.nav.right}, down: {config.nav.down}, left: {config.nav.left} }
        UiElementKind::List
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.primary}, fill: {background}, hover_fill: {background} }
        UiListViewFocus { remembered_item: {Entity::PLACEHOLDER} }
        Children [
            header,
            (
                Node {
                    width: percent(100),
                    height: px(2.0),
                }
                BackgroundColor({theme.primary})
                UiElementAccent
                IgnorePicking
            ),
            (
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(10.0),
                    overflow: Overflow::clip(),
                }
                UiScrollArea { offset: 0.0, max_offset: 0.0 }
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
                                }
                                UiScrollContent
                                Children [
                                    {rows}
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
                            (
                                scrollbar(theme, 56.0, 0.0)
                                Button
                                UiFocusNav { up: {config.scrollbar_nav.up}, right: {config.scrollbar_nav.right}, down: {config.scrollbar_nav.down}, left: {config.scrollbar_nav.left} }
                                UiElementKind::ScrollBar
                                UiElementColors { primary: {theme.secondary}, secondary: {theme.secondary}, tertiary: {control_background}, fill: Color::NONE, hover_fill: Color::NONE }
                            ),
                        ]
                    ),
                ]
            ),
        ]
    }
}

fn list_row(
    font: Handle<Font>,
    row: ListRow,
    columns: Vec<ListColumn>,
    theme: ActiveTheme,
    highlight_background: Color,
) -> impl Scene {
    let cells = row
        .cells
        .into_iter()
        .zip(columns)
        .map(|(label, column)| list_cell(font.clone(), label, theme.primary, column.width_percent))
        .collect::<Vec<_>>();

    bsn! {
        Node {
            width: percent(100),
            height: px(UI_LIST_ROW_HEIGHT),
            padding: UiRect::horizontal(px(8.0)),
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Row,
            column_gap: px(12.0),
        }
        Button
        BackgroundColor(Color::NONE)
        UiFocusNav { up: {row.nav.up}, right: {row.nav.right}, down: {row.nav.down}, left: {row.nav.left} }
        UiElementKind::ListItem
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.primary}, fill: Color::NONE, hover_fill: {highlight_background} }
        Children [
            {cells}
        ]
    }
}

fn list_header(
    font: Handle<Font>,
    theme: ActiveTheme,
    background: Color,
    columns: Vec<ListColumn>,
) -> impl Scene {
    let cells = columns
        .into_iter()
        .map(|column| {
            list_cell(
                font.clone(),
                column.heading,
                theme.primary,
                column.width_percent,
            )
        })
        .collect::<Vec<_>>();

    bsn! {
        Node {
            width: percent(100),
            height: px(UI_LIST_ROW_HEIGHT),
            padding: UiRect::horizontal(px(8.0)),
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Row,
            column_gap: px(12.0),
        }
        BackgroundColor({background})
        Children [
            {cells}
        ]
    }
}

fn list_cell(
    font: Handle<Font>,
    label: &'static str,
    colour: Color,
    width_percent: f32,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(width_percent),
            overflow: Overflow::clip(),
        }
        UiListCellText { value: {label.to_string()}, font_size: UI_BODY_FONT_SIZE }
        IgnorePicking
        Children [
            (
                Text({label})
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_BODY_FONT_SIZE),
                }
                TextColor({colour})
                UiElementLabel
                IgnorePicking
                TextLayout::new(Justify::Left, LineBreak::NoWrap)
            )
        ]
    }
}
