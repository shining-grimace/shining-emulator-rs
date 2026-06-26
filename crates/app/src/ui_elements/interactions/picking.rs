use bevy::picking::{
    Pickable,
    events::{Pointer, Press, Release},
    pointer::{PointerButton, PointerId},
};
use bevy::prelude::*;
use std::collections::HashMap;

use crate::ui_elements::back_button::UiBackButton;

use super::multi_select::UiMultiSelectPopup;
use super::scroll::{UiScrollArea, UiScrollContent};
use super::ui_input::UiInputCapture;
use super::visual_state::{ActivatedUiElement, UiElementKind};

const TOUCH_TAP_SLOP_PIXELS: f32 = 18.0;

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

#[derive(Default, Resource)]
pub(super) struct UiPointerPressState {
    primary_targets: HashMap<PointerId, UiPointerTarget>,
}

#[derive(Clone, Copy, Debug)]
struct UiPointerTarget {
    entity: Entity,
    depth: usize,
    position: Vec2,
    touch: bool,
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
            .observe(pointer_release);
    }
}

pub(super) fn resolve_pointer_activations(
    mut pointer_presses: MessageReader<Pointer<Press>>,
    mut pointer_releases: MessageReader<Pointer<Release>>,
    capture: Res<UiInputCapture>,
    mut press_state: ResMut<UiPointerPressState>,
    interactive: Query<(), Or<(With<UiElementKind>, With<UiBackButton>)>>,
    parents: Query<&ChildOf>,
    mut clicked: MessageWriter<UiPointerClicked>,
) {
    if capture.active {
        for _ in pointer_presses.read() {}
        for _ in pointer_releases.read() {}
        press_state.primary_targets.clear();
        return;
    }

    let mut pressed_pointers = Vec::new();
    let mut press_targets = HashMap::new();
    for press in pointer_presses.read() {
        if press.button != PointerButton::Primary {
            continue;
        }
        pressed_pointers.push(press.pointer_id);
        if let Some(target) = interactive_ancestor(
            press.entity,
            press.pointer_location.position,
            press.pointer_id.is_touch(),
            &interactive,
            &parents,
        ) {
            retain_deepest_target(&mut press_targets, press.pointer_id, target);
        }
    }

    for pointer_id in pressed_pointers {
        if let Some(target) = press_targets.get(&pointer_id).copied() {
            press_state.primary_targets.insert(pointer_id, target);
        } else {
            press_state.primary_targets.remove(&pointer_id);
        }
    }

    let mut released_pointers = Vec::new();
    let mut release_targets = HashMap::new();
    for release in pointer_releases.read() {
        if release.button != PointerButton::Primary {
            continue;
        }
        released_pointers.push(release.pointer_id);
        if let Some(target) = interactive_ancestor(
            release.entity,
            release.pointer_location.position,
            release.pointer_id.is_touch(),
            &interactive,
            &parents,
        ) {
            retain_deepest_target(&mut release_targets, release.pointer_id, target);
        }
    }

    for pointer_id in released_pointers {
        let Some(pressed_target) = press_state.primary_targets.remove(&pointer_id) else {
            continue;
        };
        let Some(release_target) = release_targets.get(&pointer_id) else {
            continue;
        };
        if targets_match_for_activation(pressed_target, *release_target, &parents) {
            clicked.write(UiPointerClicked {
                entity: pressed_target.entity,
            });
        }
    }
}

fn retain_deepest_target(
    targets: &mut HashMap<PointerId, UiPointerTarget>,
    pointer_id: PointerId,
    target: UiPointerTarget,
) {
    let should_insert = targets
        .get(&pointer_id)
        .is_none_or(|current| target.depth > current.depth);
    if should_insert {
        targets.insert(pointer_id, target);
    }
}

fn interactive_ancestor(
    entity: Entity,
    position: Vec2,
    touch: bool,
    interactive: &Query<(), Or<(With<UiElementKind>, With<UiBackButton>)>>,
    parents: &Query<&ChildOf>,
) -> Option<UiPointerTarget> {
    let mut current = entity;
    let mut depth = hierarchy_depth(entity, parents);
    loop {
        if interactive.get(current).is_ok() {
            return Some(UiPointerTarget {
                entity: current,
                depth,
                position,
                touch,
            });
        }
        current = parents.get(current).ok()?.0;
        depth = depth.saturating_sub(1);
    }
}

fn targets_match_for_activation(
    pressed: UiPointerTarget,
    released: UiPointerTarget,
    parents: &Query<&ChildOf>,
) -> bool {
    if pressed.entity == released.entity {
        return true;
    }

    pressed.touch
        && released.touch
        && pressed.position.distance(released.position) <= TOUCH_TAP_SLOP_PIXELS
        && entity_has_ancestor(pressed.entity, released.entity, parents)
}

fn entity_has_ancestor(entity: Entity, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    let mut current = entity;
    while let Ok(parent) = parents.get(current) {
        if parent.0 == ancestor {
            return true;
        }
        current = parent.0;
    }
    false
}

fn hierarchy_depth(entity: Entity, parents: &Query<&ChildOf>) -> usize {
    let mut depth = 0;
    let mut current = entity;
    while let Ok(parent) = parents.get(current) {
        depth += 1;
        current = parent.0;
    }
    depth
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
    states: Query<(
        Entity,
        &UiPointerState,
        Has<HoveredUiElement>,
        Has<PressedUiElement>,
    )>,
) {
    for (entity, state, has_hovered, has_pressed) in &states {
        set_marker::<HoveredUiElement>(&mut commands, entity, state.hovered, has_hovered);
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
    if !event.pointer_id.is_mouse() {
        return;
    }
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
