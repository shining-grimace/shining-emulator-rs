use bevy::prelude::*;

use super::focus::FocusedUiElement;
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
    let Some((parent_entity, mut multi_select, _, children)) =
        multi_selects.iter_mut().find(|(entity, _, _, children)| {
            *entity == option_entity || contains_entity(children, option_entity, child_query)
        })
    else {
        return;
    };

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
    multi_selects: Query<(Entity, Has<OpenUiElement>, &Children), With<DismissOnOutsideClick>>,
    mut popups: Query<(Entity, &UiMultiSelectPopup, &mut Node)>,
    child_query: Query<&Children>,
) {
    for (popup_entity, popup, mut node) in &mut popups {
        let open = multi_selects
            .iter()
            .find(|(entity, _, children)| {
                popup.parent == *entity
                    || popup.parent == Entity::PLACEHOLDER
                        && contains_entity(children, popup_entity, &child_query)
            })
            .is_some_and(|(_, open, _)| open);
        node.display = if open { Display::Flex } else { Display::None };
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
