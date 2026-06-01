use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use super::focus::FocusedUiElement;
use super::scroll::UiScrollArea;
use super::tree::contains_entity;
use super::ui_input::UiInputState;
use super::visual_state::{DisabledUiElement, UiElementKind};

const MONOSPACE_CHARACTER_WIDTH_RATIO: f32 = 0.62;

#[derive(Clone, Component, Debug, FromTemplate)]
pub struct UiListCellText {
    pub value: String,
    pub font_size: f32,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiListViewFocus {
    pub remembered_item: Entity,
}

#[derive(Clone, Copy, Component, Debug, Default)]
pub(super) struct SuppressListItemFocusRedirect;

pub(super) fn remember_focused_list_item(
    focused_items: Query<Entity, (With<FocusedUiElement>, Added<FocusedUiElement>)>,
    mut lists: Query<(&mut UiListViewFocus, &Children)>,
    child_query: Query<&Children>,
) {
    let Some(focused_item) = focused_items.iter().next() else {
        return;
    };

    for (mut list, children) in &mut lists {
        if contains_entity(children, focused_item, &child_query) {
            list.remembered_item = focused_item;
            return;
        }
    }
}

pub(super) fn focus_list_item_on_list_focus(
    mut commands: Commands,
    focused_lists: Query<
        (
            Entity,
            &UiListViewFocus,
            &Children,
            Has<SuppressListItemFocusRedirect>,
        ),
        (With<FocusedUiElement>, Added<FocusedUiElement>),
    >,
    focusable: Query<(&UiElementKind, Has<DisabledUiElement>)>,
    child_query: Query<&Children>,
) {
    for (list_entity, list, children, suppress_redirect) in &focused_lists {
        if suppress_redirect {
            commands
                .entity(list_entity)
                .remove::<SuppressListItemFocusRedirect>();
            continue;
        }

        let Some(target) =
            remembered_or_first_list_item(list.remembered_item, children, &focusable, &child_query)
        else {
            continue;
        };

        commands.entity(list_entity).remove::<FocusedUiElement>();
        commands.entity(target).insert(FocusedUiElement);
    }
}

pub(super) fn enter_focused_list_item(
    mut commands: Commands,
    input: Res<UiInputState>,
    focused_lists: Query<(Entity, &UiListViewFocus, &Children), With<FocusedUiElement>>,
    focusable: Query<(&UiElementKind, Has<DisabledUiElement>)>,
    child_query: Query<&Children>,
) {
    if !input.select {
        return;
    }

    let Some((list_entity, list, children)) = focused_lists.iter().next() else {
        return;
    };
    let Some(target) =
        remembered_or_first_list_item(list.remembered_item, children, &focusable, &child_query)
    else {
        return;
    };

    commands.entity(list_entity).remove::<FocusedUiElement>();
    commands.entity(target).insert(FocusedUiElement);
}

pub(super) fn update_list_cell_text(
    cells: Query<
        (&ComputedNode, &UiListCellText, &Children),
        Or<(Changed<ComputedNode>, Changed<UiListCellText>)>,
    >,
    mut texts: Query<&mut Text>,
) {
    for (node, cell, children) in &cells {
        let width = node.size().x;
        if width <= 0.0 {
            continue;
        }

        let value = ellipsize(&cell.value, width, cell.font_size);
        for child in children {
            let Ok(mut text) = texts.get_mut(*child) else {
                continue;
            };
            if text.0 != value {
                text.0 = value.clone();
            }
        }
    }
}

pub(super) fn update_list_item_pickability(
    mut commands: Commands,
    areas: Query<(&ComputedNode, &UiGlobalTransform, &Children), With<UiScrollArea>>,
    items: Query<
        (Entity, &ComputedNode, &UiGlobalTransform, Option<&Pickable>),
        With<UiElementKind>,
    >,
    kinds: Query<&UiElementKind>,
    child_query: Query<&Children>,
) {
    for (area_node, area_transform, children) in &areas {
        let item_entities = collect_clipped_focus_items(children, &kinds, &child_query);
        for item in item_entities {
            let Ok((entity, item_node, item_transform, pickable)) = items.get(item) else {
                continue;
            };
            let visible =
                vertical_bounds_intersect(item_node, item_transform, area_node, area_transform);
            let desired = if visible {
                Pickable::default()
            } else {
                Pickable::IGNORE
            };

            if pickable != Some(&desired) {
                commands.entity(entity).insert(desired);
            }
        }
    }
}

fn collect_clipped_focus_items(
    children: &Children,
    kinds: &Query<&UiElementKind>,
    child_query: &Query<&Children>,
) -> Vec<Entity> {
    let mut items = Vec::new();
    collect_clipped_focus_items_recursive(children, kinds, child_query, &mut items);
    items
}

fn collect_clipped_focus_items_recursive(
    children: &Children,
    kinds: &Query<&UiElementKind>,
    child_query: &Query<&Children>,
    items: &mut Vec<Entity>,
) {
    for child in children {
        if kinds.get(*child).is_ok_and(|kind| {
            matches!(
                kind,
                UiElementKind::ListItem | UiElementKind::MultiSelectOption
            )
        }) {
            items.push(*child);
            continue;
        }

        if let Ok(grandchildren) = child_query.get(*child) {
            collect_clipped_focus_items_recursive(grandchildren, kinds, child_query, items);
        }
    }
}

fn vertical_bounds_intersect(
    item_node: &ComputedNode,
    item_transform: &UiGlobalTransform,
    area_node: &ComputedNode,
    area_transform: &UiGlobalTransform,
) -> bool {
    let (_, _, item_center) = item_transform.to_scale_angle_translation();
    let (_, _, area_center) = area_transform.to_scale_angle_translation();
    let item_half_height = item_node.size().y * 0.5;
    let area_half_height = area_node.size().y * 0.5;
    let item_top = item_center.y - item_half_height;
    let item_bottom = item_center.y + item_half_height;
    let area_top = area_center.y - area_half_height;
    let area_bottom = area_center.y + area_half_height;

    item_bottom > area_top && item_top < area_bottom
}

fn remembered_or_first_list_item(
    remembered_item: Entity,
    children: &Children,
    focusable: &Query<(&UiElementKind, Has<DisabledUiElement>)>,
    child_query: &Query<&Children>,
) -> Option<Entity> {
    focusable_list_item(remembered_item, focusable)
        .filter(|target| contains_entity(children, *target, child_query))
        .or_else(|| first_list_item(children, focusable, child_query))
}

fn focusable_list_item(
    entity: Entity,
    focusable: &Query<(&UiElementKind, Has<DisabledUiElement>)>,
) -> Option<Entity> {
    focusable
        .get(entity)
        .is_ok_and(|(kind, disabled)| *kind == UiElementKind::ListItem && !disabled)
        .then_some(entity)
}

fn first_list_item(
    children: &Children,
    focusable: &Query<(&UiElementKind, Has<DisabledUiElement>)>,
    child_query: &Query<&Children>,
) -> Option<Entity> {
    for child in children {
        if let Some(entity) = focusable_list_item(*child, focusable) {
            return Some(entity);
        }

        if let Ok(grandchildren) = child_query.get(*child) {
            if let Some(entity) = first_list_item(grandchildren, focusable, child_query) {
                return Some(entity);
            }
        }
    }

    None
}

fn ellipsize(value: &str, available_width: f32, font_size: f32) -> String {
    let character_width = font_size * MONOSPACE_CHARACTER_WIDTH_RATIO;
    if character_width <= 0.0 {
        return value.to_string();
    }

    let max_chars = (available_width / character_width).floor() as usize;
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    value
        .chars()
        .take(max_chars - 3)
        .chain("...".chars())
        .collect()
}
