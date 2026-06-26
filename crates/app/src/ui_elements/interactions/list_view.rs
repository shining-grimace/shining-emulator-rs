use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::text::TextLayoutInfo;
use bevy::ui::UiGlobalTransform;

use crate::dimensions::UI_LIST_ROW_HEIGHT;
use crate::ui_elements::list_view::{VirtualListRow, VirtualListScrollArea, VirtualListSelection};

use super::focus::FocusedUiElement;
use super::scroll::UiScrollArea;
use super::tree::contains_entity;
use super::ui_input::{UiInputDirection, UiInputState};
use super::visual_state::{DisabledUiElement, SelectedUiElement, UiElementKind};

#[derive(Clone, Component, Debug, FromTemplate)]
pub struct UiListCellText {
    pub value: String,
    pub average_character_width: f32,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiListViewFocus {
    pub remembered_item: Entity,
}

#[derive(Clone, Copy, Component, Debug, Default)]
pub(super) struct SuppressListItemFocusRedirect;

pub(super) fn remember_focused_list_item(
    focused_items: Query<
        (Entity, Option<&VirtualListRow>),
        (With<FocusedUiElement>, Added<FocusedUiElement>),
    >,
    mut lists: Query<(&mut UiListViewFocus, &Children)>,
    mut selections: Query<&mut VirtualListSelection>,
    parents: Query<&ChildOf>,
    child_query: Query<&Children>,
) {
    let Some((focused_item, virtual_row)) = focused_items.iter().next() else {
        return;
    };

    for (mut list, children) in &mut lists {
        if contains_entity(children, focused_item, &child_query) {
            list.remembered_item = focused_item;
            if let Some(row) = virtual_row.filter(|row| row.item_index != usize::MAX) {
                set_virtual_selection(focused_item, *row, &mut selections, &parents);
            }
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
    virtual_rows: Query<&VirtualListRow>,
    virtual_selections: Query<&VirtualListSelection>,
    child_query: Query<&Children>,
) {
    for (list_entity, list, children, suppress_redirect) in &focused_lists {
        if suppress_redirect {
            commands
                .entity(list_entity)
                .remove::<SuppressListItemFocusRedirect>();
            continue;
        }

        let Some(target) = remembered_or_first_list_item(
            list,
            children,
            &focusable,
            &virtual_rows,
            &virtual_selections,
            &child_query,
        ) else {
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
    virtual_rows: Query<&VirtualListRow>,
    virtual_selections: Query<&VirtualListSelection>,
    child_query: Query<&Children>,
) {
    if !input.select {
        return;
    }

    let Some((list_entity, list, children)) = focused_lists.iter().next() else {
        return;
    };
    let Some(target) = remembered_or_first_list_item(
        list,
        children,
        &focusable,
        &virtual_rows,
        &virtual_selections,
        &child_query,
    ) else {
        return;
    };

    commands.entity(list_entity).remove::<FocusedUiElement>();
    commands.entity(target).insert(FocusedUiElement);
}

pub(super) fn navigate_virtual_list_by_keys(
    input: Res<UiInputState>,
    focused: Query<Entity, With<FocusedUiElement>>,
    kinds: Query<&UiElementKind>,
    parents: Query<&ChildOf>,
    child_query: Query<&Children>,
    rows: Query<&VirtualListRow>,
    mut selections: Query<(Entity, &mut VirtualListSelection, &Children)>,
    mut areas: Query<(Entity, &ComputedNode, &mut UiScrollArea), With<VirtualListScrollArea>>,
) {
    let Some(direction) = input.direction() else {
        return;
    };
    if !matches!(direction, UiInputDirection::Up | UiInputDirection::Down) {
        return;
    }

    let Some(focused_entity) = focused.iter().next() else {
        return;
    };
    if kinds.get(focused_entity) != Ok(&UiElementKind::List) {
        return;
    }

    for (selection_entity, mut selection, children) in &mut selections {
        if containing_list(selection_entity, &kinds, &parents) != Some(focused_entity) {
            continue;
        }

        let Some((_, area_node, mut area)) =
            containing_virtual_scroll_area(selection_entity, &areas, &parents)
                .and_then(|entity| areas.get_mut(entity).ok())
        else {
            continue;
        };
        let Some(row_count) = virtual_row_count(area_node, &area, children, &rows, &child_query)
        else {
            continue;
        };

        let current = selection
            .selected_row_index
            .or_else(|| {
                selected_visible_virtual_row(&selection, children, &rows, &child_query)
                    .map(|row| row.row_index)
            })
            .unwrap_or(0)
            .min(row_count.saturating_sub(1));
        let next = match direction {
            UiInputDirection::Up => current.saturating_sub(1),
            UiInputDirection::Down => (current + 1).min(row_count.saturating_sub(1)),
            _ => current,
        };

        selection.selected_row_index = Some(next);
        selection.selected_item_index =
            visible_item_index_at_row(next, children, &rows, &child_query);
        area.offset = scroll_offset_for_row(next, area_node, &area);
        return;
    }
}

pub(super) fn update_list_cell_text(
    mut cells: Query<(&ComputedNode, &mut UiListCellText, &Children)>,
    mut texts: Query<(&mut Text, &TextLayoutInfo)>,
) {
    for (node, mut cell, children) in &mut cells {
        let width = node.size().x;
        if width <= 0.0 {
            continue;
        }

        for child in children {
            let Ok((mut text, layout)) = texts.get_mut(*child) else {
                continue;
            };

            if text.0 == cell.value {
                update_average_character_width(&mut cell, layout);
            }

            let value = ellipsize(&cell.value, width, cell.average_character_width);
            if text.0 != value {
                text.0 = value;
            }
        }
    }
}

fn update_average_character_width(cell: &mut UiListCellText, layout: &TextLayoutInfo) {
    let char_count = cell.value.chars().count();
    if char_count == 0 || layout.size.x <= 0.0 {
        return;
    }

    cell.average_character_width = layout.size.x / char_count as f32;
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

pub(super) fn sync_virtual_list_selection(
    mut commands: Commands,
    mut selections: Query<(Entity, &mut VirtualListSelection, &Children)>,
    rows: Query<(
        Entity,
        &VirtualListRow,
        Has<SelectedUiElement>,
        Has<FocusedUiElement>,
    )>,
    focused: Query<Entity, With<FocusedUiElement>>,
    kinds: Query<&UiElementKind>,
    parents: Query<&ChildOf>,
    child_query: Query<&Children>,
) {
    for (selection_entity, mut selection, children) in &mut selections {
        let list_entity = containing_list(selection_entity, &kinds, &parents);
        let focus_in_list = list_entity.is_some_and(|entity| focused.get(entity).is_ok())
            || focused
                .iter()
                .any(|entity| contains_entity(children, entity, &child_query));

        let mut selected_row = None;
        for (entity, row, selected, row_focused) in &rows {
            if !contains_entity(children, entity, &child_query) {
                continue;
            }

            let should_select = row_matches_virtual_selection(&selection, row);
            if should_select {
                selected_row = Some((entity, row_focused));
                if selection.selected_row_index != Some(row.row_index) {
                    selection.selected_row_index = Some(row.row_index);
                }
                if row.item_index != usize::MAX
                    && selection.selected_item_index != Some(row.item_index)
                {
                    selection.selected_item_index = Some(row.item_index);
                }
            }

            if should_select && !selected {
                commands.entity(entity).insert(SelectedUiElement);
            } else if !should_select && selected {
                commands.entity(entity).remove::<SelectedUiElement>();
            }

            if !should_select && row_focused {
                commands.entity(entity).remove::<FocusedUiElement>();
            }
        }

        if !focus_in_list {
            continue;
        }

        if let Some((entity, focused)) = selected_row {
            if let Some(list_entity) = list_entity.filter(|_| focus_in_list) {
                commands.entity(list_entity).remove::<FocusedUiElement>();
            }
            if !focused {
                commands.entity(entity).insert(FocusedUiElement);
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
    list: &UiListViewFocus,
    children: &Children,
    focusable: &Query<(&UiElementKind, Has<DisabledUiElement>)>,
    virtual_rows: &Query<&VirtualListRow>,
    virtual_selections: &Query<&VirtualListSelection>,
    child_query: &Query<&Children>,
) -> Option<Entity> {
    if let Some(row_index) = selected_virtual_row_index(children, virtual_selections, child_query) {
        return virtual_list_item(row_index, children, focusable, virtual_rows, child_query);
    }

    focusable_list_item(list.remembered_item, focusable)
        .filter(|target| contains_entity(children, *target, child_query))
        .or_else(|| first_list_item(children, focusable, child_query))
}

fn selected_virtual_row_index(
    children: &Children,
    virtual_selections: &Query<&VirtualListSelection>,
    child_query: &Query<&Children>,
) -> Option<usize> {
    for child in children {
        if let Ok(selection) = virtual_selections.get(*child) {
            return selection.selected_row_index;
        }

        if let Ok(grandchildren) = child_query.get(*child)
            && let Some(row_index) =
                selected_virtual_row_index(grandchildren, virtual_selections, child_query)
        {
            return Some(row_index);
        }
    }

    None
}

fn virtual_list_item(
    row_index: usize,
    children: &Children,
    focusable: &Query<(&UiElementKind, Has<DisabledUiElement>)>,
    virtual_rows: &Query<&VirtualListRow>,
    child_query: &Query<&Children>,
) -> Option<Entity> {
    for child in children {
        if virtual_rows
            .get(*child)
            .is_ok_and(|row| row.row_index == row_index)
        {
            return focusable_list_item(*child, focusable);
        }

        if let Ok(grandchildren) = child_query.get(*child)
            && let Some(entity) = virtual_list_item(
                row_index,
                grandchildren,
                focusable,
                virtual_rows,
                child_query,
            )
        {
            return Some(entity);
        }
    }

    None
}

fn set_virtual_selection(
    row_entity: Entity,
    row: VirtualListRow,
    virtual_selections: &mut Query<&mut VirtualListSelection>,
    parents: &Query<&ChildOf>,
) {
    let mut current = Some(row_entity);
    while let Some(entity) = current {
        if let Ok(mut selection) = virtual_selections.get_mut(entity) {
            selection.selected_row_index = Some(row.row_index);
            selection.selected_item_index = Some(row.item_index);
            return;
        }
        current = parents.get(entity).ok().map(|parent| parent.0);
    }
}

fn selected_visible_virtual_row(
    selection: &VirtualListSelection,
    children: &Children,
    rows: &Query<&VirtualListRow>,
    child_query: &Query<&Children>,
) -> Option<VirtualListRow> {
    for child in children {
        if let Ok(row) = rows.get(*child)
            && row_matches_virtual_selection(selection, row)
        {
            return Some(*row);
        }

        if let Ok(grandchildren) = child_query.get(*child)
            && let Some(row) =
                selected_visible_virtual_row(selection, grandchildren, rows, child_query)
        {
            return Some(row);
        }
    }

    None
}

fn visible_item_index_at_row(
    row_index: usize,
    children: &Children,
    rows: &Query<&VirtualListRow>,
    child_query: &Query<&Children>,
) -> Option<usize> {
    for child in children {
        if let Ok(row) = rows.get(*child)
            && row.row_index == row_index
            && row.item_index != usize::MAX
        {
            return Some(row.item_index);
        }

        if let Ok(grandchildren) = child_query.get(*child)
            && let Some(item_index) =
                visible_item_index_at_row(row_index, grandchildren, rows, child_query)
        {
            return Some(item_index);
        }
    }

    None
}

fn visible_valid_virtual_row_count(
    children: &Children,
    rows: &Query<&VirtualListRow>,
    child_query: &Query<&Children>,
) -> usize {
    children
        .iter()
        .filter_map(|child| {
            rows.get(child)
                .ok()
                .filter(|row| row.item_index != usize::MAX)
                .map(|row| row.row_index + 1)
                .or_else(|| {
                    child_query.get(child).ok().map(|grandchildren| {
                        visible_valid_virtual_row_count(grandchildren, rows, child_query)
                    })
                })
        })
        .max()
        .unwrap_or(0)
}

fn virtual_row_count(
    area_node: &ComputedNode,
    area: &UiScrollArea,
    children: &Children,
    rows: &Query<&VirtualListRow>,
    child_query: &Query<&Children>,
) -> Option<usize> {
    if area.max_offset <= f32::EPSILON {
        return Some(visible_valid_virtual_row_count(children, rows, child_query))
            .filter(|count| *count > 0);
    }

    let total_height = area.max_offset + logical_height(area_node);
    (total_height > 0.0)
        .then(|| (total_height / UI_LIST_ROW_HEIGHT).ceil() as usize)
        .filter(|count| *count > 0)
}

fn scroll_offset_for_row(row_index: usize, area_node: &ComputedNode, area: &UiScrollArea) -> f32 {
    let viewport_height = logical_height(area_node);
    let row_top = row_index as f32 * UI_LIST_ROW_HEIGHT;
    let row_bottom = row_top + UI_LIST_ROW_HEIGHT;
    let viewport_top = area.offset;
    let viewport_bottom = viewport_top + viewport_height;

    if row_top < viewport_top {
        row_top
    } else if row_bottom > viewport_bottom {
        row_bottom - viewport_height
    } else {
        area.offset
    }
    .clamp(0.0, area.max_offset)
}

fn logical_height(node: &ComputedNode) -> f32 {
    node.size().y * node.inverse_scale_factor()
}

fn containing_virtual_scroll_area(
    entity: Entity,
    areas: &Query<(Entity, &ComputedNode, &mut UiScrollArea), With<VirtualListScrollArea>>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let mut current = entity;
    while let Ok(parent) = parents.get(current) {
        if areas.get(parent.0).is_ok() {
            return Some(parent.0);
        }
        current = parent.0;
    }

    None
}

fn row_matches_virtual_selection(selection: &VirtualListSelection, row: &VirtualListRow) -> bool {
    selection
        .selected_row_index
        .is_some_and(|selected| selected == row.row_index)
        || (selection.selected_row_index.is_none()
            && row.item_index != usize::MAX
            && selection
                .selected_item_index
                .is_some_and(|selected| selected == row.item_index))
}

fn containing_list(
    entity: Entity,
    kinds: &Query<&UiElementKind>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let mut current = entity;
    while let Ok(parent) = parents.get(current) {
        if kinds
            .get(parent.0)
            .is_ok_and(|kind| *kind == UiElementKind::List)
        {
            return Some(parent.0);
        }
        current = parent.0;
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;

    fn list_focus() -> UiListViewFocus {
        UiListViewFocus {
            remembered_item: Entity::PLACEHOLDER,
        }
    }

    #[test]
    fn focused_virtual_row_updates_virtual_selection() {
        let mut world = World::new();
        let row = world
            .spawn((
                VirtualListRow {
                    slot: 0,
                    row_index: 3,
                    item_index: 7,
                },
                FocusedUiElement,
            ))
            .id();
        let content = world.spawn(VirtualListSelection::default()).id();
        world.entity_mut(content).add_child(row);
        let list = world.spawn(list_focus()).id();
        world.entity_mut(list).add_child(content);

        let mut schedule = Schedule::default();
        schedule.add_systems(remember_focused_list_item);
        schedule.run(&mut world);

        assert_eq!(
            world
                .get::<VirtualListSelection>(content)
                .expect("selection should exist")
                .selected_row_index,
            Some(3)
        );
        assert_eq!(
            world
                .get::<VirtualListSelection>(content)
                .expect("selection should exist")
                .selected_item_index,
            Some(7)
        );
    }

    #[test]
    fn virtual_list_focus_does_not_redirect_to_stale_recycled_row() {
        let mut world = World::new();

        let row = world
            .spawn((
                VirtualListRow {
                    slot: 0,
                    row_index: 9,
                    item_index: 9,
                },
                UiElementKind::ListItem,
            ))
            .id();
        let content = world
            .spawn(VirtualListSelection {
                selected_row_index: Some(7),
                selected_item_index: Some(7),
            })
            .id();
        world.entity_mut(content).add_child(row);
        let list = world
            .spawn((
                UiListViewFocus {
                    remembered_item: row,
                },
                FocusedUiElement,
            ))
            .id();
        world.entity_mut(list).add_child(content);

        let mut schedule = Schedule::default();
        schedule.add_systems(focus_list_item_on_list_focus);
        schedule.run(&mut world);

        assert!(world.get::<FocusedUiElement>(list).is_some());
        assert!(world.get::<FocusedUiElement>(row).is_none());
    }

    #[test]
    fn selected_visible_virtual_row_does_not_steal_empty_focus() {
        let mut world = World::new();
        let row = world
            .spawn((
                VirtualListRow {
                    slot: 0,
                    row_index: 7,
                    item_index: 7,
                },
                UiElementKind::ListItem,
            ))
            .id();
        let content = world
            .spawn(VirtualListSelection {
                selected_row_index: Some(7),
                selected_item_index: Some(7),
            })
            .id();
        world.entity_mut(content).add_child(row);
        let list = world.spawn(UiElementKind::List).id();
        world.entity_mut(list).add_child(content);

        let mut schedule = Schedule::default();
        schedule.add_systems(sync_virtual_list_selection);
        schedule.run(&mut world);

        assert!(world.get::<SelectedUiElement>(row).is_some());
        assert!(world.get::<FocusedUiElement>(row).is_none());
    }

    #[test]
    fn recycled_virtual_row_loses_stale_focus_when_selection_is_offscreen() {
        let mut world = World::new();
        let row = world
            .spawn((
                VirtualListRow {
                    slot: 0,
                    row_index: 9,
                    item_index: 9,
                },
                UiElementKind::ListItem,
                FocusedUiElement,
            ))
            .id();
        let content = world
            .spawn(VirtualListSelection {
                selected_row_index: Some(7),
                selected_item_index: Some(7),
            })
            .id();
        world.entity_mut(content).add_child(row);
        let list = world.spawn(UiElementKind::List).id();
        world.entity_mut(list).add_child(content);

        let mut schedule = Schedule::default();
        schedule.add_systems(sync_virtual_list_selection);
        schedule.run(&mut world);

        assert!(world.get::<SelectedUiElement>(row).is_none());
        assert!(world.get::<FocusedUiElement>(row).is_none());
    }

    #[test]
    fn list_focused_key_navigation_moves_offscreen_virtual_selection() {
        let mut world = World::new();
        let row = world
            .spawn((
                VirtualListRow {
                    slot: 0,
                    row_index: 5,
                    item_index: 7,
                },
                UiElementKind::ListItem,
            ))
            .id();
        let content = world
            .spawn(VirtualListSelection {
                selected_row_index: Some(5),
                selected_item_index: Some(7),
            })
            .id();
        world.entity_mut(content).add_child(row);
        let area = world
            .spawn((
                VirtualListScrollArea,
                UiScrollArea {
                    offset: 0.0,
                    max_offset: UI_LIST_ROW_HEIGHT * 10.0,
                },
                ComputedNode::default(),
            ))
            .id();
        world.entity_mut(area).add_child(content);
        let list = world
            .spawn((UiElementKind::List, FocusedUiElement, list_focus()))
            .id();
        world.entity_mut(list).add_child(area);
        world.insert_resource(UiInputState {
            down: true,
            ..default()
        });

        let mut schedule = Schedule::default();
        schedule.add_systems(navigate_virtual_list_by_keys);
        schedule.run(&mut world);

        let selection = world
            .get::<VirtualListSelection>(content)
            .expect("selection should exist");
        assert_eq!(selection.selected_row_index, Some(6));
        assert_eq!(selection.selected_item_index, None);
        assert!(
            world
                .get::<UiScrollArea>(area)
                .expect("area should exist")
                .offset
                > 0.0
        );
    }

    #[test]
    fn ellipsize_keeps_full_value_when_it_fits() {
        assert_eq!(ellipsize("Last Played", 110.0, 10.0), "Last Played");
    }

    #[test]
    fn ellipsize_adds_suffix_at_available_width() {
        assert_eq!(
            ellipsize("abcdefghijklmnopqrstuvwxyz", 100.0, 10.0),
            "abcdefg..."
        );
    }

    #[test]
    fn ellipsize_leaves_text_unmodified_until_width_is_known() {
        assert_eq!(
            ellipsize("abcdefghijklmnopqrstuvwxyz", 100.0, 0.0),
            "abcdefghijklmnopqrstuvwxyz"
        );
    }
}

fn ellipsize(value: &str, available_width: f32, character_width: f32) -> String {
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
