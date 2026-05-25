use bevy::prelude::*;

use super::focus::FocusedUiElement;
use super::scroll::{UiPopupScrollArea, UiScrollArea, UiScrollContent, UiScrollThumb, UiScrollbar};
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
            .insert((OpenUiElement, ActivatedUiElement));
    } else {
        commands.entity(entity).remove::<OpenUiElement>();
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
            commands.entity(entity).remove::<OpenUiElement>();
        }
    }
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
        commands.entity(entity).remove::<FocusedUiElement>();
    }
    commands.entity(option_entity).insert(FocusedUiElement);
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
    commands.entity(parent_entity).remove::<OpenUiElement>();
    commands.entity(parent_entity).insert(ActivatedUiElement);
    update_multi_select_label(children, label, texts, child_query);

    for (entity, _) in focused {
        commands.entity(entity).remove::<FocusedUiElement>();
    }
    commands.entity(parent_entity).insert(FocusedUiElement);
}

pub(super) fn update_multi_select_popups(
    mut commands: Commands,
    multi_selects: Query<(Entity, Has<OpenUiElement>, &Children), With<DismissOnOutsideClick>>,
    mut scroll_nodes: ParamSet<(
        Query<(
            Entity,
            &UiMultiSelectPopup,
            &mut Node,
            Has<OpenUiMultiSelectPopup>,
        )>,
        Query<(Entity, &mut UiScrollArea, Option<&UiPopupScrollArea>)>,
        Query<&mut Node, With<UiScrollContent>>,
        Query<&mut Node, With<UiScrollbar>>,
        Query<(&UiScrollThumb, &mut Node)>,
    )>,
    focused: Query<Entity, With<FocusedUiElement>>,
    child_query: Query<&Children>,
) {
    let mut reset_popups = Vec::new();
    let mut closed_popups = Vec::new();
    for (popup_entity, popup, mut node, was_open) in &mut scroll_nodes.p0() {
        let parent = multi_selects.iter().find_map(|(entity, open, children)| {
            (popup.parent == entity
                || popup.parent == Entity::PLACEHOLDER
                    && contains_entity(children, popup_entity, &child_query))
            .then_some((entity, open))
        });
        let open = parent.is_some_and(|(_, open)| open);
        node.display = if open { Display::Flex } else { Display::None };
        if open && !was_open {
            commands.entity(popup_entity).insert(OpenUiMultiSelectPopup);
            reset_popups.push(popup_entity);
        }
        if !open && was_open {
            commands
                .entity(popup_entity)
                .remove::<OpenUiMultiSelectPopup>();
            closed_popups.push((popup_entity, parent.map(|(entity, _)| entity)));
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
            commands.entity(entity).remove::<FocusedUiElement>();
        }
        if let Some(parent_entity) = parent_entity {
            commands.entity(parent_entity).insert(FocusedUiElement);
        }
    }

    let mut reset_areas = Vec::new();
    for (area_entity, mut area, popup_scroll) in &mut scroll_nodes.p1() {
        if !reset_popups
            .iter()
            .any(|popup| contains_descendant(*popup, area_entity, &child_query))
        {
            continue;
        }

        area.offset = 0.0;
        area.max_offset = 0.0;
        reset_areas.push((
            area_entity,
            popup_scroll
                .copied()
                .is_some_and(UiPopupScrollArea::has_overflow),
        ));
    }

    for (area_entity, _) in &reset_areas {
        if let Ok(children) = child_query.get(*area_entity) {
            reset_scroll_content(children, &mut scroll_nodes.p2(), &child_query);
        }
    }
    for (area_entity, visible) in &reset_areas {
        if let Ok(children) = child_query.get(*area_entity) {
            reset_scrollbar_visibility(children, *visible, &mut scroll_nodes.p3(), &child_query);
        }
    }
    for (area_entity, _) in &reset_areas {
        if let Ok(children) = child_query.get(*area_entity) {
            reset_scroll_thumbs(children, &mut scroll_nodes.p4(), &child_query);
        }
    }
}

fn contains_descendant(root: Entity, target: Entity, child_query: &Query<&Children>) -> bool {
    root == target
        || child_query
            .get(root)
            .is_ok_and(|children| contains_entity(children, target, child_query))
}

fn reset_scroll_content(
    children: &Children,
    scroll_contents: &mut Query<&mut Node, With<UiScrollContent>>,
    child_query: &Query<&Children>,
) {
    for child in children {
        if let Ok(mut node) = scroll_contents.get_mut(*child) {
            node.top = px(0.0);
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            reset_scroll_content(grandchildren, scroll_contents, child_query);
        }
    }
}

fn reset_scrollbar_visibility(
    children: &Children,
    visible: bool,
    scrollbars: &mut Query<&mut Node, With<UiScrollbar>>,
    child_query: &Query<&Children>,
) {
    for child in children {
        if let Ok(mut node) = scrollbars.get_mut(*child) {
            node.display = if visible {
                Display::Flex
            } else {
                Display::None
            };
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            reset_scrollbar_visibility(grandchildren, visible, scrollbars, child_query);
        }
    }
}

fn reset_scroll_thumbs(
    children: &Children,
    scroll_thumbs: &mut Query<(&UiScrollThumb, &mut Node)>,
    child_query: &Query<&Children>,
) {
    for child in children {
        if let Ok((_, mut node)) = scroll_thumbs.get_mut(*child) {
            node.top = px(6.0);
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            reset_scroll_thumbs(grandchildren, scroll_thumbs, child_query);
        }
    }
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
