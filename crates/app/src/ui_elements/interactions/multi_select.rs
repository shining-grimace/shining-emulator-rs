use bevy::prelude::*;

use super::focus::FocusedUiElement;
use super::scroll::{UiPopupScrollArea, UiScrollArea};
use super::tree::contains_entity;
use super::visual_state::{ActivatedUiElement, UiElementKind};

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct OpenUiElement;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct DismissOnOutsideClick;

#[derive(Clone, Component, Debug, FromTemplate)]
pub struct UiMultiSelect {
    pub selected: usize,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiMultiSelectPopup {
    pub parent: Entity,
}

#[derive(Clone, Copy, Component, Debug, Default)]
pub(super) struct OpenUiMultiSelectPopup;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiMultiSelectLabel;

#[derive(Clone, Component, Debug, FromTemplate)]
pub struct UiMultiSelectOption {
    pub option_index: usize,
    pub label: String,
}

pub(super) type MultiSelectQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut UiMultiSelect,
        Has<OpenUiElement>,
        &'static Children,
    ),
    With<DismissOnOutsideClick>,
>;

pub(super) fn set_open(commands: &mut Commands, entity: Entity, open: bool) {
    if open {
        commands
            .entity(entity)
            .try_insert((OpenUiElement, ActivatedUiElement));
    } else {
        commands.entity(entity).try_remove::<OpenUiElement>();
    }
}

pub(super) fn dismiss_open_elements_for_outside_click(
    commands: &mut Commands,
    clicked_entities: &[Entity],
    dismissible: &MultiSelectQuery,
    child_query: &Query<&Children>,
) {
    for (entity, _, open, children) in dismissible {
        if !open {
            continue;
        }
        let clicked_inside = clicked_entities
            .iter()
            .any(|clicked| *clicked == entity || contains_entity(children, *clicked, child_query));
        if !clicked_inside {
            commands.entity(entity).try_remove::<OpenUiElement>();
        }
    }
}

pub(super) fn entities_are_inside_open_element(
    entities: &[Entity],
    dismissible: &MultiSelectQuery,
    child_query: &Query<&Children>,
) -> bool {
    dismissible.iter().any(|(entity, _, open, children)| {
        open && entities
            .iter()
            .any(|target| *target == entity || contains_entity(children, *target, child_query))
    })
}

pub(super) fn focus_selected_option(
    commands: &mut Commands,
    focused: &Query<(Entity, &UiElementKind), With<FocusedUiElement>>,
    children: &Children,
    selected: usize,
    options: &Query<(Entity, &UiMultiSelectOption)>,
    child_query: &Query<&Children>,
) {
    let Some(option_entity) = find_multi_select_option(children, selected, options, child_query)
    else {
        return;
    };

    for (entity, _) in focused {
        commands.entity(entity).try_remove::<FocusedUiElement>();
    }
    commands.entity(option_entity).try_insert(FocusedUiElement);
}

pub(super) fn choose_multi_select_option(
    commands: &mut Commands,
    focused: &Query<(Entity, &UiElementKind), With<FocusedUiElement>>,
    option_entity: Entity,
    option_index: usize,
    label: &str,
    multi_selects: &mut MultiSelectQuery,
    texts: &mut Query<&mut Text, With<UiMultiSelectLabel>>,
    child_query: &Query<&Children>,
) {
    let Some((parent_entity, mut multi_select, open, children)) =
        multi_selects.iter_mut().find(|(entity, _, _, children)| {
            *entity == option_entity || contains_entity(children, option_entity, child_query)
        })
    else {
        return;
    };
    if !open {
        return;
    }

    multi_select.selected = option_index;
    commands.entity(parent_entity).try_remove::<OpenUiElement>();
    commands
        .entity(parent_entity)
        .try_insert(ActivatedUiElement);
    update_multi_select_label(children, label, texts, child_query);

    for (entity, _) in focused {
        commands.entity(entity).try_remove::<FocusedUiElement>();
    }
    commands.entity(parent_entity).try_insert(FocusedUiElement);
}

pub(super) fn update_multi_select_popups(
    mut commands: Commands,
    multi_selects: Query<
        (Entity, &UiMultiSelect, Has<OpenUiElement>, &Children),
        With<DismissOnOutsideClick>,
    >,
    mut scroll_nodes: ParamSet<(
        Query<(Entity, &UiMultiSelectPopup, Has<OpenUiMultiSelectPopup>)>,
        Query<&mut Node>,
        Query<(Entity, &mut UiScrollArea, Option<&UiPopupScrollArea>)>,
        Query<&Node>,
    )>,
    focused: Query<Entity, With<FocusedUiElement>>,
    child_query: Query<&Children>,
    parents: Query<&ChildOf>,
) {
    let mut reset_popups = Vec::new();
    let mut closed_popups = Vec::new();

    let popup_states = {
        let popups = scroll_nodes
            .p0()
            .iter()
            .map(|(entity, popup, was_open)| (entity, *popup, was_open))
            .collect::<Vec<_>>();
        let nodes = scroll_nodes.p3();
        popups
            .into_iter()
            .map(|(popup_entity, popup, was_open)| {
                let parent =
                    multi_selects
                        .iter()
                        .find_map(|(entity, multi_select, open, children)| {
                            (popup.parent == entity
                                || popup.parent == Entity::PLACEHOLDER
                                    && contains_entity(children, popup_entity, &child_query))
                            .then_some((entity, multi_select.selected, open))
                        });
                let parent_visible =
                    parent.is_some_and(|(entity, _, _)| entity_visible(entity, &nodes, &parents));
                if let Some((entity, _, true)) = parent
                    && !parent_visible
                {
                    commands.entity(entity).try_remove::<OpenUiElement>();
                }
                let open = parent.is_some_and(|(_, _, open)| open) && parent_visible;
                (
                    popup_entity,
                    open,
                    was_open,
                    parent.map(|(_, selected, _)| selected),
                    parent.map(|(entity, _, _)| entity),
                )
            })
            .collect::<Vec<_>>()
    };

    {
        let mut popup_nodes = scroll_nodes.p1();
        for (popup_entity, open, was_open, selected, parent_entity) in popup_states {
            let Ok(mut node) = popup_nodes.get_mut(popup_entity) else {
                continue;
            };
            let display = if open { Display::Flex } else { Display::None };
            if node.display != display {
                node.display = display;
            }
            if open && !was_open {
                commands
                    .entity(popup_entity)
                    .try_insert(OpenUiMultiSelectPopup);
                reset_popups.push((popup_entity, selected));
            }
            if !open && was_open {
                commands
                    .entity(popup_entity)
                    .try_remove::<OpenUiMultiSelectPopup>();
                closed_popups.push((popup_entity, parent_entity));
            }
        }
    }

    for (popup_entity, parent_entity) in closed_popups {
        if !focused
            .iter()
            .any(|entity| contains_descendant(popup_entity, entity, &child_query))
        {
            continue;
        }

        for entity in &focused {
            commands.entity(entity).try_remove::<FocusedUiElement>();
        }
        if let Some(parent_entity) = parent_entity {
            commands.entity(parent_entity).try_insert(FocusedUiElement);
        }
    }

    for (area_entity, mut area, popup_scroll) in &mut scroll_nodes.p2() {
        let selected = reset_popups.iter().find_map(|(popup, selected)| {
            contains_descendant(*popup, area_entity, &child_query).then_some(*selected)
        });
        let Some(selected) = selected else {
            continue;
        };

        let popup_scroll = popup_scroll.copied();
        area.max_offset = popup_scroll
            .map(UiPopupScrollArea::max_offset)
            .unwrap_or_default();
        area.offset = popup_scroll
            .and_then(|popup_scroll| {
                selected.map(|selected| selected_popup_option_scroll_offset(popup_scroll, selected))
            })
            .unwrap_or_default()
            .clamp(0.0, area.max_offset);
    }
}

fn entity_visible(entity: Entity, nodes: &Query<&Node>, parents: &Query<&ChildOf>) -> bool {
    let mut current = Some(entity);
    while let Some(entity) = current {
        if nodes
            .get(entity)
            .is_ok_and(|node| node.display == Display::None)
        {
            return false;
        }
        current = parents.get(entity).ok().map(|parent| parent.0);
    }
    true
}

fn selected_popup_option_scroll_offset(popup_scroll: UiPopupScrollArea, selected: usize) -> f32 {
    let viewport_height = popup_scroll.visible_height();
    let option_top = selected as f32 * (popup_scroll.option_height + popup_scroll.option_gap);
    let option_bottom = option_top + popup_scroll.option_height;

    if option_bottom > viewport_height {
        option_bottom - viewport_height
    } else {
        0.0
    }
}

fn contains_descendant(root: Entity, target: Entity, child_query: &Query<&Children>) -> bool {
    root == target
        || child_query
            .get(root)
            .is_ok_and(|children| contains_entity(children, target, child_query))
}

fn find_multi_select_option(
    children: &Children,
    selected: usize,
    options: &Query<(Entity, &UiMultiSelectOption)>,
    child_query: &Query<&Children>,
) -> Option<Entity> {
    for child in children {
        if let Ok((entity, option)) = options.get(*child) {
            if option.option_index == selected {
                return Some(entity);
            }
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            if let Some(entity) =
                find_multi_select_option(grandchildren, selected, options, child_query)
            {
                return Some(entity);
            }
        }
    }
    None
}

fn update_multi_select_label(
    children: &Children,
    label: &str,
    texts: &mut Query<&mut Text, With<UiMultiSelectLabel>>,
    child_query: &Query<&Children>,
) {
    for child in children {
        if let Ok(mut text) = texts.get_mut(*child) {
            text.0 = label.to_string();
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            update_multi_select_label(grandchildren, label, texts, child_query);
        }
    }
}
