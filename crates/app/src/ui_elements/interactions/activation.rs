use bevy::input::ButtonInput;
use bevy::prelude::*;

use crate::app_state::AppState;
use crate::ui_elements::back_button::UiBackButton;
use crate::ui_elements::file_picker::{UiFilePicker, UiFilePickerActivated};
use crate::ui_elements::list_view::{VirtualListRow, VirtualListSelection};

use super::focus::FocusedUiElement;
use super::multi_select::{
    MultiSelectQuery, UiMultiSelectLabel, UiMultiSelectOption, choose_multi_select_option,
    dismiss_open_elements_for_outside_click, entities_are_inside_open_element,
    focus_selected_option, set_open,
};
use super::picking::{HoveredUiElement, UiPointerClicked};
use super::tree::contains_entity;
use super::ui_input::{UiInputCapture, UiInputState};
use super::visual_state::{
    ActivatedUiElement, DisabledUiElement, SelectedUiElement, UiElementKind,
};

pub(super) fn activate_controls(
    mut commands: Commands,
    mut clicked: MessageReader<UiPointerClicked>,
    input: Res<UiInputState>,
    mut multi_selects: MultiSelectQuery,
    options: Query<(Entity, &UiMultiSelectOption)>,
    kinds: Query<&UiElementKind>,
    focused: Query<(Entity, &UiElementKind), With<FocusedUiElement>>,
    selected_rows: Query<Entity, With<SelectedUiElement>>,
    back_buttons: Query<(), With<UiBackButton>>,
    file_pickers: Query<
        (Entity, Option<&Children>),
        (With<UiFilePicker>, Without<DisabledUiElement>),
    >,
    mut file_picker_activated: MessageWriter<UiFilePickerActivated>,
    mut app_exit: MessageWriter<AppExit>,
    mut texts: Query<&mut Text, With<UiMultiSelectLabel>>,
    child_query: Query<&Children>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let keyboard_activation = input.select;
    let state_value = *state.get();
    let back_activation = input.back && back_input_enabled(state_value);
    let clicked_entities = clicked.read().map(|click| click.entity).collect::<Vec<_>>();
    if clicked_entities.is_empty() && !input.select && !input.back && !input.quit_app {
        return;
    }

    if input.quit_app || back_activation {
        match back_navigation_target(state_value) {
            Some(target) => next_state.set(target),
            None => {
                app_exit.write(AppExit::Success);
            }
        }
    }

    if clicked_entities
        .iter()
        .any(|entity| back_buttons.get(*entity).is_ok())
    {
        match back_navigation_target(*state.get()) {
            Some(target) => next_state.set(target),
            None => {
                app_exit.write(AppExit::Success);
            }
        }
    }

    for (entity, kind) in &focused {
        if !keyboard_activation || *kind == UiElementKind::TextInput {
            continue;
        }
        commands.entity(entity).insert(ActivatedUiElement);

        match kind {
            UiElementKind::MultiSelect => {
                if let Ok((_, multi_select, open, children)) = multi_selects.get_mut(entity) {
                    set_open(&mut commands, entity, !open);
                    if !open {
                        focus_selected_option(
                            &mut commands,
                            &focused,
                            children,
                            multi_select.selected,
                            &options,
                            &child_query,
                        );
                    }
                }
            }
            UiElementKind::MultiSelectOption => {
                if let Ok((_, option)) = options.get(entity) {
                    let option_index = option.option_index;
                    let label = option.label.clone();
                    choose_multi_select_option(
                        &mut commands,
                        &focused,
                        entity,
                        option_index,
                        &label,
                        &mut multi_selects,
                        &mut texts,
                        &child_query,
                    );
                }
            }
            _ => {}
        }
    }

    for entity in clicked_entities
        .iter()
        .copied()
        .filter(|entity| kinds.get(*entity).is_ok())
    {
        commands.entity(entity).insert(ActivatedUiElement);
    }

    for (entity, multi_select, open, children) in &mut multi_selects {
        if clicked_entities.contains(&entity) {
            let next_open = !open;
            set_open(&mut commands, entity, next_open);
            if next_open {
                focus_selected_option(
                    &mut commands,
                    &focused,
                    children,
                    multi_select.selected,
                    &options,
                    &child_query,
                );
            }
        }
    }

    for clicked_entity in clicked_entities.iter().copied() {
        let Some((entity, option)) = options.iter().find(|(entity, _)| {
            *entity == clicked_entity
                || child_query
                    .get(*entity)
                    .is_ok_and(|children| contains_entity(children, clicked_entity, &child_query))
        }) else {
            continue;
        };

        let label = option.label.clone();
        choose_multi_select_option(
            &mut commands,
            &focused,
            entity,
            option.option_index,
            &label,
            &mut multi_selects,
            &mut texts,
            &child_query,
        );
    }

    for clicked_entity in clicked_entities.iter().copied().filter(|entity| {
        kinds
            .get(*entity)
            .is_ok_and(|kind| *kind == UiElementKind::ListItem)
    }) {
        for selected_entity in &selected_rows {
            if selected_entity != clicked_entity {
                commands
                    .entity(selected_entity)
                    .remove::<SelectedUiElement>();
            }
        }
        commands
            .entity(clicked_entity)
            .insert((SelectedUiElement, ActivatedUiElement));
    }

    for (entity, children) in &file_pickers {
        if clicked_entities.iter().any(|clicked| {
            *clicked == entity
                || children
                    .is_some_and(|children| contains_entity(children, *clicked, &child_query))
        }) || (keyboard_activation
            && focused.iter().any(|(focused_entity, kind)| {
                focused_entity == entity && *kind == UiElementKind::Button
            }))
        {
            commands.entity(entity).insert(ActivatedUiElement);
            file_picker_activated.write(UiFilePickerActivated { picker: entity });
        }
    }
}

pub(super) fn select_virtual_list_rows(
    mut clicked: MessageReader<UiPointerClicked>,
    capture: Res<UiInputCapture>,
    virtual_rows: Query<&VirtualListRow>,
    mut virtual_selections: Query<&mut VirtualListSelection>,
    parents: Query<&ChildOf>,
) {
    if capture.active {
        for _ in clicked.read() {}
        return;
    }

    for clicked_entity in clicked.read().map(|click| click.entity) {
        let Ok(row) = virtual_rows.get(clicked_entity) else {
            continue;
        };
        if row.item_index == usize::MAX {
            continue;
        }
        select_virtual_list_row(clicked_entity, *row, &mut virtual_selections, &parents);
    }
}

fn select_virtual_list_row(
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

pub(super) fn dismiss_multi_selects_on_pointer_release(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut clicked: MessageReader<UiPointerClicked>,
    capture: Res<UiInputCapture>,
    hovered: Query<Entity, With<HoveredUiElement>>,
    multi_selects: MultiSelectQuery,
    child_query: Query<&Children>,
    mut press_started_inside_open_element: Local<bool>,
) {
    if capture.active {
        for _ in clicked.read() {}
        *press_started_inside_open_element = false;
        return;
    }

    let hovered_entities = hovered.iter().collect::<Vec<_>>();
    if mouse_buttons.just_pressed(MouseButton::Left) {
        *press_started_inside_open_element =
            entities_are_inside_open_element(&hovered_entities, &multi_selects, &child_query);
    }

    let clicked_entities = clicked.read().map(|click| click.entity).collect::<Vec<_>>();
    if !mouse_buttons.just_released(MouseButton::Left) {
        return;
    }

    if *press_started_inside_open_element {
        *press_started_inside_open_element = false;
        return;
    }

    dismiss_open_elements_for_outside_click(
        &mut commands,
        &clicked_entities,
        &multi_selects,
        &child_query,
    );
}

fn back_navigation_target(state: AppState) -> Option<AppState> {
    match state {
        AppState::Home => None,
        AppState::AudioSettings | AppState::InputMapping | AppState::RomProvider => {
            Some(AppState::Settings)
        }
        _ => Some(AppState::Home),
    }
}

fn back_input_enabled(state: AppState) -> bool {
    !matches!(state, AppState::Gameplay | AppState::InputMapping)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_navigation_from_rom_provider_returns_to_settings() {
        assert_eq!(
            back_navigation_target(AppState::RomProvider),
            Some(AppState::Settings)
        );
    }

    #[test]
    fn back_navigation_from_input_mapping_returns_to_settings() {
        assert_eq!(
            back_navigation_target(AppState::InputMapping),
            Some(AppState::Settings)
        );
    }

    #[test]
    fn back_navigation_from_audio_settings_returns_to_settings() {
        assert_eq!(
            back_navigation_target(AppState::AudioSettings),
            Some(AppState::Settings)
        );
    }

    #[test]
    fn back_navigation_from_rom_data_returns_to_home() {
        assert_eq!(
            back_navigation_target(AppState::RomData),
            Some(AppState::Home)
        );
    }

    #[test]
    fn back_navigation_from_settings_returns_to_home() {
        assert_eq!(
            back_navigation_target(AppState::Settings),
            Some(AppState::Home)
        );
    }

    #[test]
    fn back_navigation_from_gameplay_returns_to_home() {
        assert_eq!(
            back_navigation_target(AppState::Gameplay),
            Some(AppState::Home)
        );
    }

    #[test]
    fn back_input_is_disabled_for_gameplay() {
        assert!(!back_input_enabled(AppState::Gameplay));
    }

    #[test]
    fn back_navigation_from_home_exits_app() {
        assert_eq!(back_navigation_target(AppState::Home), None);
    }
}
