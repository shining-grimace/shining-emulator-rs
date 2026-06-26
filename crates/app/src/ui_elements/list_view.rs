use bevy::asset::HandleTemplate;
use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::dimensions::{UI_BODY_FONT_SIZE, UI_LIST_HEIGHT, UI_LIST_ROW_HEIGHT};
use crate::ui_elements::interactions::{
    AutoScrollFocusedChild, IgnorePicking, UiElementAccent, UiElementColors, UiElementKind,
    UiElementLabel, UiFocusNav, UiListCellText, UiListViewFocus, UiScrollArea, UiScrollContent,
};
use crate::ui_elements::scrollbar::scrollbar;
use crate::ui_elements::styles::{
    control_fill, hover_fill, transparent, ui_border, ui_padding, ui_radius,
};
use crate::ui_elements::theme::{
    UiElementTheme, UiThemeBackgroundColor, UiThemeBorderColor, UiThemeTextColor,
};

#[derive(Clone, Copy)]
pub struct ListColumn {
    pub heading: &'static str,
    pub width_percent: f32,
}

pub struct ListRow {
    pub cells: Vec<String>,
    pub nav: UiFocusNav,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct ListRowIndex {
    pub index: usize,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct ListCellIndex {
    pub index: usize,
}

pub struct ListViewConfig {
    pub nav: UiFocusNav,
    pub scrollbar_nav: UiFocusNav,
    pub columns: Vec<ListColumn>,
    pub rows: Vec<ListRow>,
    pub virtual_total_rows: Option<usize>,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct VirtualListScrollArea;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct VirtualListContent;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct VirtualListSelection {
    pub selected_row_index: Option<usize>,
    pub selected_item_index: Option<usize>,
}

#[derive(Clone, Copy, Component, Debug)]
pub struct VirtualListRow {
    pub slot: usize,
    pub row_index: usize,
    pub item_index: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct VirtualListWindow {
    pub first_row: usize,
    pub content_offset: f32,
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
        .enumerate()
        .map(|(index, row)| {
            list_row(
                font.clone(),
                index,
                row,
                columns.clone(),
                theme,
                hover_background,
            )
        })
        .collect::<Vec<_>>();
    let content_rows = config
        .virtual_total_rows
        .unwrap_or(rows.len())
        .max(rows.len());
    let content_height = content_rows as f32 * UI_LIST_ROW_HEIGHT;

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
        UiThemeBorderColor::Primary
        BackgroundColor({background})
        UiFocusNav { up: {config.nav.up}, right: {config.nav.right}, down: {config.nav.down}, left: {config.nav.left} }
        UiElementKind::List
        UiElementTheme::List
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
                UiThemeBackgroundColor::Primary
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
                VirtualListScrollArea
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
                                    height: px(content_height),
                                    flex_direction: FlexDirection::Column,
                                }
                                UiScrollContent
                                VirtualListContent
                                VirtualListSelection::default()
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
                                UiElementTheme::ScrollBar
                                UiElementColors { primary: {theme.secondary}, secondary: {theme.secondary}, tertiary: {control_background}, fill: Color::NONE, hover_fill: Color::NONE }
                            ),
                        ]
                    ),
                ]
            ),
        ]
    }
}

pub fn virtual_list_rows<T>(
    items: &[T],
    order: &[usize],
    pool_size: usize,
    column_count: usize,
    row: impl Fn(&T) -> ListRow,
) -> Vec<ListRow> {
    (0..pool_size)
        .map(|slot| {
            order
                .get(slot)
                .and_then(|index| items.get(*index))
                .map(&row)
                .unwrap_or_else(|| blank_list_row(column_count))
        })
        .collect()
}

pub fn blank_list_row(column_count: usize) -> ListRow {
    ListRow {
        cells: (0..column_count).map(|_| String::new()).collect(),
        nav: UiFocusNav::default(),
    }
}

pub fn virtual_list_window(area: &UiScrollArea) -> VirtualListWindow {
    VirtualListWindow {
        first_row: (area.offset / UI_LIST_ROW_HEIGHT).floor() as usize,
        content_offset: area.offset % UI_LIST_ROW_HEIGHT,
    }
}

pub fn virtual_list_content_rows(total_rows: usize, row_pool_size: usize) -> usize {
    total_rows.max(row_pool_size)
}

pub fn virtual_list_content_height(total_rows: usize, row_pool_size: usize) -> f32 {
    virtual_list_content_rows(total_rows, row_pool_size) as f32 * UI_LIST_ROW_HEIGHT
}

pub fn collect_list_item_entities(
    children: &Children,
    kinds: &Query<&UiElementKind>,
    child_query: &Query<&Children>,
) -> Vec<Entity> {
    let mut items = Vec::new();
    collect_list_item_entities_recursive(children, kinds, child_query, &mut items);
    items
}

fn collect_list_item_entities_recursive(
    children: &Children,
    kinds: &Query<&UiElementKind>,
    child_query: &Query<&Children>,
    items: &mut Vec<Entity>,
) {
    for child in children {
        if kinds
            .get(*child)
            .is_ok_and(|kind| *kind == UiElementKind::ListItem)
        {
            items.push(*child);
            continue;
        }

        if let Ok(grandchildren) = child_query.get(*child) {
            collect_list_item_entities_recursive(grandchildren, kinds, child_query, items);
        }
    }
}

pub fn set_list_row_cells<F: QueryFilter>(
    values: &[&str],
    children: &Children,
    cells: &mut Query<(&mut UiListCellText, &Children)>,
    texts: &mut Query<&mut Text, F>,
    child_query: &Query<&Children>,
) {
    for (cell_index, cell_entity) in collect_list_cell_entities(children, cells, child_query)
        .into_iter()
        .enumerate()
    {
        let Some(value) = values.get(cell_index) else {
            continue;
        };
        let Ok((mut cell, text_children)) = cells.get_mut(cell_entity) else {
            continue;
        };
        cell.value = (*value).to_string();
        for child in text_children {
            if let Ok(mut text) = texts.get_mut(*child) {
                text.0 = (*value).to_string();
            }
        }
    }
}

fn collect_list_cell_entities(
    children: &Children,
    cells: &Query<(&mut UiListCellText, &Children)>,
    child_query: &Query<&Children>,
) -> Vec<Entity> {
    let mut entities = Vec::new();
    collect_list_cell_entities_recursive(children, cells, child_query, &mut entities);
    entities
}

fn collect_list_cell_entities_recursive(
    children: &Children,
    cells: &Query<(&mut UiListCellText, &Children)>,
    child_query: &Query<&Children>,
    entities: &mut Vec<Entity>,
) {
    for child in children {
        if cells.contains(*child) {
            entities.push(*child);
            continue;
        }

        if let Ok(grandchildren) = child_query.get(*child) {
            collect_list_cell_entities_recursive(grandchildren, cells, child_query, entities);
        }
    }
}

fn list_row(
    font: Handle<Font>,
    index: usize,
    row: ListRow,
    columns: Vec<ListColumn>,
    theme: ActiveTheme,
    highlight_background: Color,
) -> impl Scene {
    let cells = row
        .cells
        .into_iter()
        .zip(columns)
        .enumerate()
        .map(|(index, (label, column))| {
            list_cell(
                font.clone(),
                index,
                label,
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
        Button
        BackgroundColor(Color::NONE)
        ListRowIndex { index }
        UiFocusNav { up: {row.nav.up}, right: {row.nav.right}, down: {row.nav.down}, left: {row.nav.left} }
        UiElementKind::ListItem
        UiElementTheme::ListItem
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
        .enumerate()
        .map(|(index, column)| {
            list_cell(
                font.clone(),
                index,
                column.heading.to_string(),
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
    index: usize,
    label: String,
    colour: Color,
    width_percent: f32,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(width_percent),
            overflow: Overflow::clip(),
        }
        ListCellIndex { index }
        UiListCellText { value: {label.clone()}, average_character_width: 0.0 }
        IgnorePicking
        Children [
            (
                Text({label})
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_BODY_FONT_SIZE),
                }
                TextColor({colour})
                UiThemeTextColor::Primary
                UiElementLabel
                IgnorePicking
                TextLayout::new(Justify::Left, LineBreak::NoWrap)
            )
        ]
    }
}
