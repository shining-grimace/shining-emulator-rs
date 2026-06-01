use bevy::picking::{Pickable, hover::PickingInteraction};
use bevy::prelude::*;

use super::multi_select::UiMultiSelectPopup;
use super::scroll::{UiScrollArea, UiScrollContent};
use super::visual_state::ActivatedUiElement;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct HoveredUiElement;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct PressedUiElement;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct ModalUiElement;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct DraggableUiElement;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct IgnorePicking;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct BlockPickingOnly;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiPointerState {
    pub hovered: bool,
    pub pressed: bool,
}

#[derive(Clone, Copy, Debug, Message)]
pub struct UiPointerClicked {
    pub entity: Entity,
}

pub(super) fn setup_pointer_tracking(
    mut commands: Commands,
    buttons: Query<Entity, (Added<Button>, Without<UiPointerState>)>,
    scroll_areas: Query<
        Entity,
        (
            Added<UiScrollArea>,
            Without<Button>,
            Without<UiPointerState>,
        ),
    >,
    draggables: Query<Entity, (Added<DraggableUiElement>, Without<UiPointerState>)>,
) {
    for entity in buttons
        .iter()
        .chain(scroll_areas.iter())
        .chain(draggables.iter())
    {
        commands
            .entity(entity)
            .insert(UiPointerState::default())
            .observe(pointer_over)
            .observe(pointer_out)
            .observe(pointer_press)
            .observe(pointer_release)
            .observe(pointer_click);
    }
}

pub(super) fn apply_picking_markers(
    mut commands: Commands,
    ignored: Query<Entity, Added<IgnorePicking>>,
    blockers: Query<Entity, Added<BlockPickingOnly>>,
    modals: Query<Entity, Added<ModalUiElement>>,
    scroll_contents: Query<Entity, Added<UiScrollContent>>,
    passive_layouts: Query<
        Entity,
        (
            Added<Node>,
            Without<Button>,
            Without<UiScrollArea>,
            Without<UiMultiSelectPopup>,
            Without<DraggableUiElement>,
            Without<IgnorePicking>,
            Without<BlockPickingOnly>,
            Without<ModalUiElement>,
        ),
    >,
) {
    for entity in ignored
        .iter()
        .chain(scroll_contents.iter())
        .chain(passive_layouts.iter())
    {
        commands.entity(entity).insert(Pickable::IGNORE);
    }

    for entity in blockers.iter().chain(modals.iter()) {
        commands.entity(entity).insert(Pickable {
            should_block_lower: true,
            is_hoverable: false,
        });
    }
}

pub(super) fn sync_pointer_states(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut states: Query<(
        Entity,
        &PickingInteraction,
        &mut UiPointerState,
        Has<HoveredUiElement>,
        Has<PressedUiElement>,
    )>,
) {
    for (entity, interaction, mut state, has_hovered, has_pressed) in &mut states {
        let hovered = *interaction != PickingInteraction::None;
        if !mouse_buttons.pressed(MouseButton::Left) {
            state.pressed = false;
        }

        set_marker::<HoveredUiElement>(
            &mut commands,
            entity,
            hovered || state.hovered,
            has_hovered,
        );
        set_marker::<PressedUiElement>(&mut commands, entity, state.pressed, has_pressed);
    }
}

pub(super) fn set_marker<T: Component + Default>(
    commands: &mut Commands,
    entity: Entity,
    should_have_marker: bool,
    has_marker: bool,
) {
    if should_have_marker && !has_marker {
        commands.entity(entity).insert(T::default());
    } else if !should_have_marker && has_marker {
        commands.entity(entity).remove::<T>();
    }
}

pub(super) fn clear_activation_markers(
    mut commands: Commands,
    activated: Query<Entity, With<ActivatedUiElement>>,
) {
    for entity in &activated {
        commands.entity(entity).remove::<ActivatedUiElement>();
    }
}

fn pointer_over(event: On<Pointer<Over>>, mut states: Query<&mut UiPointerState>) {
    if let Ok(mut state) = states.get_mut(event.entity) {
        state.hovered = true;
    }
}

fn pointer_out(event: On<Pointer<Out>>, mut states: Query<&mut UiPointerState>) {
    if let Ok(mut state) = states.get_mut(event.entity) {
        state.hovered = false;
    }
}

fn pointer_press(event: On<Pointer<Press>>, mut states: Query<&mut UiPointerState>) {
    if let Ok(mut state) = states.get_mut(event.entity) {
        state.pressed = true;
    }
}

fn pointer_release(event: On<Pointer<Release>>, mut states: Query<&mut UiPointerState>) {
    if let Ok(mut state) = states.get_mut(event.entity) {
        state.pressed = false;
    }
}

fn pointer_click(event: On<Pointer<Click>>, mut clicked: MessageWriter<UiPointerClicked>) {
    clicked.write(UiPointerClicked {
        entity: event.entity,
    });
}
