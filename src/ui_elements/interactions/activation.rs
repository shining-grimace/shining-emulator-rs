use bevy::input::{ButtonInput, ButtonState};
use bevy::prelude::*;

use crate::app_state::AppState;
use crate::input::events::MappedInputEvent;
use crate::storage::input_mappings::InputAction;
use crate::ui_elements::file_picker::{UiFilePicker, UiFilePickerActivated};

use super::focus::FocusedUiElement;
use super::multi_select::{
    MultiSelectQuery, UiMultiSelectLabel, UiMultiSelectOption, choose_multi_select_option,
    dismiss_open_elements_for_outside_click, focus_selected_option, set_open,
};
use super::picking::UiPointerClicked;
use super::tree::contains_entity;
use super::visual_state::{
    ActivatedUiElement, DisabledUiElement, SelectedUiElement, UiElementKind,
};

pub(super) fn activate_controls(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut clicked: MessageReader<UiPointerClicked>,
    mut mapped_events: MessageReader<MappedInputEvent>,
    mut multi_selects: MultiSelectQuery,
    options: Query<(Entity, &UiMultiSelectOption)>,
    kinds: Query<&UiElementKind>,
    focused: Query<(Entity, &UiElementKind), With<FocusedUiElement>>,
    selected_rows: Query<Entity, With<SelectedUiElement>>,
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
    let mut mapped_select = false;
    let mut mapped_quit = false;
    for event in mapped_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match event.action {
            InputAction::A => mapped_select = true,
            InputAction::QuitApp => mapped_quit = true,
            _ => {}
        }
    }
    let keyboard_activation = mapped_select;
    let clicked_entities = clicked.read().map(|click| click.entity).collect::<Vec<_>>();
    let pointer_released = mouse_buttons.just_released(MouseButton::Left);

    if mapped_quit {
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

    for (entity, multi_select, _, children) in &mut multi_selects {
        if clicked_entities.contains(&entity) {
            set_open(&mut commands, entity, true);
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
            commands
                .entity(selected_entity)
                .remove::<SelectedUiElement>();
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

    if pointer_released {
        dismiss_open_elements_for_outside_click(
            &mut commands,
            &clicked_entities,
            &multi_selects,
            &child_query,
        );
    }
}

fn back_navigation_target(state: AppState) -> Option<AppState> {
    match state {
        AppState::Home => None,
        AppState::RomProvider => Some(AppState::Settings),
        _ => Some(AppState::Home),
    }
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
    fn back_navigation_from_settings_returns_to_home() {
        assert_eq!(
            back_navigation_target(AppState::Settings),
            Some(AppState::Home)
        );
    }

    #[test]
    fn back_navigation_from_home_exits_app() {
        assert_eq!(back_navigation_target(AppState::Home), None);
    }
}
