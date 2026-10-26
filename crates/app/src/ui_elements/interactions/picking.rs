use bevy::picking::{
    Pickable,
    events::{PointerPress, PointerRelease},
    pointer::{PointerButton, PointerId},
};
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use std::collections::{HashMap, HashSet};

use crate::ui_elements::back_button::UiBackButton;

use super::multi_select::UiMultiSelectPopup;
use super::scroll::{UiScrollArea, UiScrollContent};
use super::ui_input::UiInputCapture;
use super::visual_state::{ActivatedUiElement, UiElementKind};

const TOUCH_TAP_SLOP_PIXELS: f32 = 18.0;
const MOUSE_TAP_SLOP_PIXELS: f32 = 5.0;

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

#[derive(Component, Default)]
pub(super) struct UiPressedPointers(HashSet<PointerId>);

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
            .insert((UiPointerState::default(), UiPressedPointers::default()))
            .observe(pointer_over)
            .observe(pointer_out)
            .observe(pointer_press);
    }
}

pub(super) fn resolve_pointer_activations(
    mut pointer_presses: MessageReader<PointerPress>,
    mut pointer_releases: MessageReader<PointerRelease>,
    mut pointer_cancels: MessageReader<PointerCancel>,
    mut pointer_drags: MessageReader<PointerDrag>,
    mut pointer_drag_ends: MessageReader<PointerDragEnd>,
    capture: Res<UiInputCapture>,
    mut press_state: ResMut<UiPointerPressState>,
    interactive: Query<(), Or<(With<UiElementKind>, With<UiBackButton>)>>,
    parents: Query<&ChildOf>,
    mut clicked: MessageWriter<UiPointerClicked>,
) {
    if capture.active {
        for _ in pointer_presses.read() {}
        for _ in pointer_releases.read() {}
        pointer_cancels.clear();
        pointer_drags.clear();
        pointer_drag_ends.clear();
        press_state.primary_targets.clear();
        return;
    }

    let mut pressed_pointers = Vec::new();
    let mut press_targets = HashMap::new();
    for press in pointer_presses.read() {
        if press.button != PointerButton::Primary {
            continue;
        }
        pressed_pointers.push(press.pointer.id);
        if let Some(target) = interactive_ancestor(
            press.entity,
            press.pointer.position,
            press.pointer.id.is_touch(),
            &interactive,
            &parents,
        ) {
            retain_deepest_target(&mut press_targets, press.pointer.id, target);
        }
    }

    for pointer_id in pressed_pointers {
        if let Some(target) = press_targets.get(&pointer_id).copied() {
            press_state.primary_targets.insert(pointer_id, target);
        } else {
            press_state.primary_targets.remove(&pointer_id);
        }
    }

    // Buffered messages also reach us when headless widgets stop event propagation.
    for cancel in pointer_cancels.read() {
        press_state.primary_targets.remove(&cancel.pointer.id);
    }
    for drag in pointer_drags.read() {
        let slop = if drag.pointer.id.is_touch() {
            TOUCH_TAP_SLOP_PIXELS
        } else {
            MOUSE_TAP_SLOP_PIXELS
        };
        if drag.button == PointerButton::Primary && drag.distance.length() > slop {
            press_state.primary_targets.remove(&drag.pointer.id);
        }
    }

    let mut released_pointers = Vec::new();
    let mut release_targets = HashMap::new();
    for release in pointer_releases.read() {
        if release.button != PointerButton::Primary {
            continue;
        }
        released_pointers.push(release.pointer.id);
        if let Some(target) = interactive_ancestor(
            release.entity,
            release.pointer.position,
            release.pointer.id.is_touch(),
            &interactive,
            &parents,
        ) {
            retain_deepest_target(&mut release_targets, release.pointer.id, target);
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
    // DragEnd targets the original press even when release occurs over empty space.
    for end in pointer_drag_ends.read() {
        if end.button == PointerButton::Primary {
            press_state.primary_targets.remove(&end.pointer.id);
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

fn pointer_over(event: On<PointerOver>, mut states: Query<&mut UiPointerState>) {
    if !event.pointer.id.is_mouse() {
        return;
    }
    if let Ok(mut state) = states.get_mut(event.entity) {
        state.hovered = true;
    }
}

fn pointer_out(event: On<PointerOut>, mut states: Query<&mut UiPointerState>) {
    if !event.pointer.id.is_mouse() {
        return;
    }
    if let Ok(mut state) = states.get_mut(event.entity) {
        state.hovered = false;
    }
}

fn pointer_press(
    event: On<PointerPress>,
    mut states: Query<(&mut UiPointerState, &mut UiPressedPointers)>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    if let Ok((mut state, mut pointers)) = states.get_mut(event.entity) {
        pointers.0.insert(event.pointer.id);
        state.pressed = true;
    }
}

pub(super) fn finish_pointer_presses(
    mut releases: MessageReader<PointerRelease>,
    mut cancels: MessageReader<PointerCancel>,
    mut drag_ends: MessageReader<PointerDragEnd>,
    capture: Res<UiInputCapture>,
    mut states: Query<(&mut UiPointerState, &mut UiPressedPointers)>,
) {
    let finished: HashSet<_> = releases
        .read()
        .filter(|event| event.button == PointerButton::Primary)
        .map(|event| event.pointer.id)
        .chain(cancels.read().map(|event| event.pointer.id))
        .chain(
            drag_ends
                .read()
                .filter(|event| event.button == PointerButton::Primary)
                .map(|event| event.pointer.id),
        )
        .collect();
    for (mut state, mut pointers) in &mut states {
        if capture.active {
            pointers.0.clear();
        } else {
            pointers.0.retain(|id| !finished.contains(id));
        }
        let pressed = !pointers.0.is_empty();
        if state.pressed != pressed {
            state.pressed = pressed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::ui_widgets::ButtonPlugin)
            .init_resource::<UiInputCapture>()
            .init_resource::<UiPointerPressState>()
            .add_message::<PointerPress>()
            .add_message::<PointerRelease>()
            .add_message::<PointerCancel>()
            .add_message::<PointerDrag>()
            .add_message::<PointerDragEnd>()
            .add_message::<UiPointerClicked>()
            .add_systems(
                Update,
                (
                    setup_pointer_tracking,
                    finish_pointer_presses,
                    resolve_pointer_activations,
                )
                    .chain(),
            );
        app
    }

    #[test]
    fn pointer_touch_leaving_does_not_clear_mouse_hover() {
        let mut app = app();
        let button = app.world_mut().spawn((Button, UiElementKind::Button)).id();
        app.update();
        let mut mouse = pointer(1);
        mouse.id = PointerId::Mouse;
        app.world_mut().trigger(PointerOver {
            entity: button,
            pointer: mouse.clone(),
            hit: hit(),
        });
        app.world_mut().trigger(PointerOut {
            entity: button,
            pointer: pointer(1),
            hit: hit(),
        });
        assert!(app.world().get::<UiPointerState>(button).unwrap().hovered);
        app.world_mut().trigger(PointerOut {
            entity: button,
            pointer: mouse,
            hit: hit(),
        });
        assert!(!app.world().get::<UiPointerState>(button).unwrap().hovered);
    }

    #[test]
    fn pointer_releasing_or_cancelling_one_finger_preserves_the_other_press() {
        let mut app = app();
        let button = app.world_mut().spawn((Button, UiElementKind::Button)).id();
        app.update();
        app.world_mut().trigger(press(button, 1));
        app.world_mut().trigger(press(button, 2));
        // Release over a different entity must still clear the original control.
        app.world_mut()
            .write_message(release(Entity::PLACEHOLDER, 1));
        app.update();
        assert!(app.world().get::<UiPointerState>(button).unwrap().pressed);
        app.world_mut().write_message(cancel(button, 2));
        app.update();
        assert!(!app.world().get::<UiPointerState>(button).unwrap().pressed);
    }

    #[test]
    fn pointer_cancelled_and_dragged_presses_do_not_click_or_cancel_another_finger() {
        for cancelled in [true, false] {
            let mut app = app();
            let button = app.world_mut().spawn(UiElementKind::Button).id();
            app.world_mut().write_message(press(button, 1));
            app.world_mut().write_message(press(button, 2));
            app.update();
            if cancelled {
                app.world_mut().write_message(cancel(button, 1));
            } else {
                app.world_mut()
                    .write_message(drag(button, 1, Vec2::new(0.0, 50.0)));
            }
            app.world_mut().write_message(release(button, 1));
            app.world_mut().write_message(release(button, 2));
            app.update();
            let clicks: Vec<_> = app
                .world_mut()
                .resource_mut::<Messages<UiPointerClicked>>()
                .drain()
                .collect();
            assert_eq!(clicks.len(), 1);
            assert_eq!(clicks[0].entity, button);
            assert!(
                app.world()
                    .resource::<UiPointerPressState>()
                    .primary_targets
                    .is_empty()
            );
        }
    }

    #[test]
    fn pointer_drag_ending_over_empty_space_clears_visuals_and_pending_click() {
        let mut app = app();
        let button = app.world_mut().spawn((Button, UiElementKind::Button)).id();
        app.update();
        app.world_mut().trigger(press(button, 1));
        app.world_mut().write_message(press(button, 1));
        app.update();
        app.world_mut().write_message(PointerDragEnd {
            entity: button,
            pointer: pointer(1),
            button: PointerButton::Primary,
            distance: Vec2::new(100.0, 0.0),
        });
        app.update();
        assert!(!app.world().get::<UiPointerState>(button).unwrap().pressed);
        assert!(
            app.world()
                .resource::<UiPointerPressState>()
                .primary_targets
                .is_empty()
        );
        assert!(
            app.world()
                .resource::<Messages<UiPointerClicked>>()
                .is_empty()
        );
    }

    #[test]
    fn pointer_small_touch_movement_still_allows_a_tap() {
        let mut app = app();
        let button = app.world_mut().spawn(UiElementKind::Button).id();
        app.world_mut().write_message(press(button, 1));
        app.world_mut().write_message(drag(button, 1, Vec2::ONE));
        app.world_mut().write_message(release(button, 1));
        app.update();
        assert_eq!(
            app.world().resource::<Messages<UiPointerClicked>>().len(),
            1
        );
    }
}
