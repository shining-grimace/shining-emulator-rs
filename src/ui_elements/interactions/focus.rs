use bevy::prelude::*;

use crate::input::events::MappedInputEvent;
use crate::storage::input_mappings::InputAction;

use super::list_view::SuppressListItemFocusRedirect;
use super::picking::UiPointerClicked;
use super::scroll::UiScrollArea;
use super::visual_state::{DisabledUiElement, UiElementKind};

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiFocusNav {
    pub up: Entity,
    pub right: Entity,
    pub down: Entity,
    pub left: Entity,
}

pub const UI_FOCUS_NONE: u16 = u16::MAX;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiFocusId {
    pub id: u16,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiFocusNavIds {
    pub up: u16,
    pub right: u16,
    pub down: u16,
    pub left: u16,
}

impl Default for UiFocusNav {
    fn default() -> Self {
        Self {
            up: Entity::PLACEHOLDER,
            right: Entity::PLACEHOLDER,
            down: Entity::PLACEHOLDER,
            left: Entity::PLACEHOLDER,
        }
    }
}

pub(super) fn bind_focus_nav_ids(
    _added: On<Add, UiFocusNavIds>,
    mut targets: ParamSet<(
        Query<(Entity, &UiFocusId)>,
        Query<(&UiFocusNavIds, &mut UiFocusNav)>,
    )>,
) {
    let target_entities = targets
        .p0()
        .iter()
        .map(|(entity, target)| (target.id, entity))
        .collect::<Vec<_>>();
    let target = |id| {
        if id == UI_FOCUS_NONE {
            return Entity::PLACEHOLDER;
        }
        target_entities
            .iter()
            .find_map(|(target_id, entity)| (*target_id == id).then_some(*entity))
            .unwrap_or(Entity::PLACEHOLDER)
    };

    for (nav_ids, mut nav) in &mut targets.p1() {
        *nav = UiFocusNav {
            up: target(nav_ids.up),
            right: target(nav_ids.right),
            down: target(nav_ids.down),
            left: target(nav_ids.left),
        };
    }
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct InitialFocus {
    pub enabled: bool,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct DefaultFocusTarget;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct FocusedUiElement;

#[derive(Default, Resource)]
pub struct LastFocusedUiElement {
    entity: Option<Entity>,
}

pub(super) fn ensure_initial_focus(
    mut commands: Commands,
    focused: Query<(), With<FocusedUiElement>>,
    candidates: Query<
        (Entity, &InitialFocus),
        (
            Added<InitialFocus>,
            With<UiFocusNav>,
            Without<DisabledUiElement>,
        ),
    >,
) {
    if !focused.is_empty() {
        return;
    }

    if let Some((entity, _)) = candidates
        .iter()
        .find(|(_, initial_focus)| initial_focus.enabled)
    {
        commands.entity(entity).insert(FocusedUiElement);
    }
}

pub(super) fn restore_focus_from_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut mapped_events: MessageReader<MappedInputEvent>,
    focused: Query<(), With<FocusedUiElement>>,
    candidates: Query<
        (
            Entity,
            &UiElementKind,
            Option<&InitialFocus>,
            Has<DefaultFocusTarget>,
            Has<DisabledUiElement>,
            Option<&Node>,
            Option<&UiScrollArea>,
        ),
        With<UiFocusNav>,
    >,
    mut last_focused: ResMut<LastFocusedUiElement>,
) {
    if !focused.is_empty() || !focus_recovery_requested(&keys, &mut mapped_events) {
        return;
    }

    let target = last_focused
        .entity
        .and_then(|entity| focusable_entity(entity, &candidates))
        .or_else(|| default_focus_target(&candidates))
        .or_else(|| initial_focus_target(&candidates));

    let Some(target) = target else {
        last_focused.entity = None;
        return;
    };

    commands.entity(target).insert(FocusedUiElement);
    last_focused.entity = Some(target);
}

pub(super) fn remember_focused_element(
    focused: Query<Entity, With<FocusedUiElement>>,
    mut last_focused: ResMut<LastFocusedUiElement>,
) {
    if let Some(entity) = focused.iter().next() {
        last_focused.entity = Some(entity);
    }
}

pub(super) fn navigate_focus(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    focused: Query<(Entity, &UiFocusNav, &UiElementKind), With<FocusedUiElement>>,
    candidates: Query<(
        &UiFocusNav,
        &UiElementKind,
        Has<DisabledUiElement>,
        Option<&Node>,
        Option<&UiScrollArea>,
    )>,
    child_query: Query<&Children>,
    parents: Query<&ChildOf>,
) {
    let Some(request) = requested_target(&keys, &focused) else {
        return;
    };

    let target_entity = if request.target == Entity::PLACEHOLDER {
        resolve_placeholder_target(
            request.focused_entity,
            request.direction,
            request.kind,
            &candidates,
            &child_query,
            &parents,
        )
    } else {
        resolve_target(
            request.target,
            request.direction,
            &candidates,
            &child_query,
            &parents,
        )
    };

    let Some(target_entity) = target_entity else {
        return;
    };
    if request.kind == UiElementKind::ListItem
        && target_kind(target_entity, &candidates) == Some(UiElementKind::List)
    {
        commands
            .entity(target_entity)
            .insert(SuppressListItemFocusRedirect);
    }

    for (entity, _, _) in &focused {
        commands.entity(entity).remove::<FocusedUiElement>();
    }
    commands.entity(target_entity).insert(FocusedUiElement);
}

fn target_kind(
    entity: Entity,
    candidates: &Query<(
        &UiFocusNav,
        &UiElementKind,
        Has<DisabledUiElement>,
        Option<&Node>,
        Option<&UiScrollArea>,
    )>,
) -> Option<UiElementKind> {
    candidates.get(entity).ok().map(|(_, kind, _, _, _)| *kind)
}

pub(super) fn focus_pressed_element(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut clicked: MessageReader<UiPointerClicked>,
    focusable: Query<&UiElementKind, (With<UiFocusNav>, Without<DisabledUiElement>)>,
    focused: Query<Entity, With<FocusedUiElement>>,
) {
    let clicked = clicked.read().map(|click| click.entity).collect::<Vec<_>>();

    if mouse_buttons.just_pressed(MouseButton::Left)
        && !clicked.iter().any(|entity| focusable.get(*entity).is_ok())
    {
        for entity in &focused {
            commands.entity(entity).remove::<FocusedUiElement>();
        }
        return;
    }

    for entity in clicked {
        let Ok(kind) = focusable.get(entity) else {
            continue;
        };
        if *kind == UiElementKind::MultiSelectOption {
            continue;
        };

        for entity in &focused {
            commands.entity(entity).remove::<FocusedUiElement>();
        }
        commands.entity(entity).insert(FocusedUiElement);
        return;
    }
}

fn focus_recovery_requested(
    keys: &ButtonInput<KeyCode>,
    mapped_events: &mut MessageReader<MappedInputEvent>,
) -> bool {
    if keys.any_just_pressed([
        KeyCode::ArrowUp,
        KeyCode::ArrowRight,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
    ]) {
        return true;
    }

    mapped_events.read().any(|event| {
        event.state == bevy::input::ButtonState::Pressed
            && matches!(
                event.action,
                InputAction::Dup
                    | InputAction::Dright
                    | InputAction::Ddown
                    | InputAction::Dleft
                    | InputAction::A
            )
    })
}

fn focusable_entity(
    entity: Entity,
    candidates: &Query<
        (
            Entity,
            &UiElementKind,
            Option<&InitialFocus>,
            Has<DefaultFocusTarget>,
            Has<DisabledUiElement>,
            Option<&Node>,
            Option<&UiScrollArea>,
        ),
        With<UiFocusNav>,
    >,
) -> Option<Entity> {
    candidates
        .get(entity)
        .is_ok_and(|(_, kind, _, _, disabled, node, scroll_area)| {
            focus_available(*kind, disabled, node, scroll_area)
        })
        .then_some(entity)
}

fn default_focus_target(
    candidates: &Query<
        (
            Entity,
            &UiElementKind,
            Option<&InitialFocus>,
            Has<DefaultFocusTarget>,
            Has<DisabledUiElement>,
            Option<&Node>,
            Option<&UiScrollArea>,
        ),
        With<UiFocusNav>,
    >,
) -> Option<Entity> {
    candidates
        .iter()
        .find_map(|(entity, kind, _, default, disabled, node, scroll_area)| {
            (default && focus_available(*kind, disabled, node, scroll_area)).then_some(entity)
        })
}

fn initial_focus_target(
    candidates: &Query<
        (
            Entity,
            &UiElementKind,
            Option<&InitialFocus>,
            Has<DefaultFocusTarget>,
            Has<DisabledUiElement>,
            Option<&Node>,
            Option<&UiScrollArea>,
        ),
        With<UiFocusNav>,
    >,
) -> Option<Entity> {
    candidates
        .iter()
        .find_map(|(entity, kind, initial, _, disabled, node, scroll_area)| {
            initial
                .is_some_and(|initial| initial.enabled)
                .then_some(entity)
                .filter(|_| focus_available(*kind, disabled, node, scroll_area))
        })
}

fn requested_target(
    keys: &ButtonInput<KeyCode>,
    focused: &Query<(Entity, &UiFocusNav, &UiElementKind), With<FocusedUiElement>>,
) -> Option<FocusRequest> {
    let (entity, nav, kind) = focused.iter().next()?;

    let (direction, target) = if keys.just_pressed(KeyCode::ArrowUp) {
        focus_target(FocusDirection::Up, nav.up)
    } else if *kind != UiElementKind::TextInput && keys.just_pressed(KeyCode::ArrowRight) {
        focus_target(FocusDirection::Right, nav.right)
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        focus_target(FocusDirection::Down, nav.down)
    } else if *kind != UiElementKind::TextInput && keys.just_pressed(KeyCode::ArrowLeft) {
        focus_target(FocusDirection::Left, nav.left)
    } else {
        None
    }?;

    Some(FocusRequest {
        focused_entity: entity,
        kind: *kind,
        direction,
        target,
    })
}

fn focus_target(direction: FocusDirection, target: Entity) -> Option<(FocusDirection, Entity)> {
    Some((direction, target))
}

struct FocusRequest {
    focused_entity: Entity,
    kind: UiElementKind,
    direction: FocusDirection,
    target: Entity,
}

fn resolve_target(
    target: Entity,
    direction: FocusDirection,
    candidates: &Query<(
        &UiFocusNav,
        &UiElementKind,
        Has<DisabledUiElement>,
        Option<&Node>,
        Option<&UiScrollArea>,
    )>,
    child_query: &Query<&Children>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let mut next = target;
    let mut hops_remaining = candidates.iter().len();

    while next != Entity::PLACEHOLDER {
        if hops_remaining == 0 {
            return None;
        }
        hops_remaining -= 1;

        let Ok((nav, kind, disabled, node, scroll_area)) = candidates.get(next) else {
            return None;
        };

        if focus_available(*kind, disabled, node, scroll_area) {
            return Some(next);
        }

        let skipped_entity = next;
        next = direction.target(nav);
        if next == Entity::PLACEHOLDER {
            return resolve_placeholder_target(
                skipped_entity,
                direction,
                *kind,
                candidates,
                child_query,
                parents,
            );
        }
    }

    None
}

fn resolve_placeholder_target(
    focused_entity: Entity,
    direction: FocusDirection,
    kind: UiElementKind,
    candidates: &Query<(
        &UiFocusNav,
        &UiElementKind,
        Has<DisabledUiElement>,
        Option<&Node>,
        Option<&UiScrollArea>,
    )>,
    child_query: &Query<&Children>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    match (kind, direction) {
        (UiElementKind::ScrollBar, FocusDirection::Left) => {
            let children = child_query.get(focused_entity).ok()?;
            first_focusable_descendant(children, candidates, child_query)
        }
        (UiElementKind::MultiSelectOption, FocusDirection::Up | FocusDirection::Down) => {
            adjacent_multi_select_option(
                focused_entity,
                direction,
                candidates,
                child_query,
                parents,
            )
        }
        (UiElementKind::ListItem, FocusDirection::Up | FocusDirection::Down) => {
            adjacent_list_item(focused_entity, direction, candidates, child_query, parents)
        }
        (UiElementKind::ListItem, FocusDirection::Left | FocusDirection::Right) => {
            containing_list(focused_entity, candidates, parents)
        }
        _ => None,
    }
}

fn first_focusable_descendant(
    children: &Children,
    candidates: &Query<(
        &UiFocusNav,
        &UiElementKind,
        Has<DisabledUiElement>,
        Option<&Node>,
        Option<&UiScrollArea>,
    )>,
    child_query: &Query<&Children>,
) -> Option<Entity> {
    for child in children {
        if candidates
            .get(*child)
            .is_ok_and(|(_, kind, disabled, node, scroll_area)| {
                focus_available(*kind, disabled, node, scroll_area)
            })
        {
            return Some(*child);
        }

        if let Ok(grandchildren) = child_query.get(*child) {
            if let Some(entity) = first_focusable_descendant(grandchildren, candidates, child_query)
            {
                return Some(entity);
            }
        }
    }

    None
}

fn adjacent_multi_select_option(
    focused_entity: Entity,
    direction: FocusDirection,
    candidates: &Query<(
        &UiFocusNav,
        &UiElementKind,
        Has<DisabledUiElement>,
        Option<&Node>,
        Option<&UiScrollArea>,
    )>,
    child_query: &Query<&Children>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let parent = parents.get(focused_entity).ok()?.0;
    let siblings = child_query.get(parent).ok()?;
    let options = siblings
        .iter()
        .filter_map(|sibling| {
            candidates
                .get(sibling)
                .is_ok_and(|(_, kind, disabled, node, scroll_area)| {
                    *kind == UiElementKind::MultiSelectOption
                        && focus_available(*kind, disabled, node, scroll_area)
                })
                .then_some(sibling)
        })
        .collect::<Vec<_>>();
    let index = options
        .iter()
        .position(|entity| *entity == focused_entity)?;
    match direction {
        FocusDirection::Up => index.checked_sub(1).and_then(|index| options.get(index)),
        FocusDirection::Down => options.get(index + 1),
        _ => None,
    }
    .copied()
}

fn adjacent_list_item(
    focused_entity: Entity,
    direction: FocusDirection,
    candidates: &Query<(
        &UiFocusNav,
        &UiElementKind,
        Has<DisabledUiElement>,
        Option<&Node>,
        Option<&UiScrollArea>,
    )>,
    child_query: &Query<&Children>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let parent = parents.get(focused_entity).ok()?.0;
    let siblings = child_query.get(parent).ok()?;
    let items = siblings
        .iter()
        .filter_map(|sibling| {
            candidates
                .get(sibling)
                .is_ok_and(|(_, kind, disabled, node, scroll_area)| {
                    *kind == UiElementKind::ListItem
                        && focus_available(*kind, disabled, node, scroll_area)
                })
                .then_some(sibling)
        })
        .collect::<Vec<_>>();
    let index = items.iter().position(|entity| *entity == focused_entity)?;
    match direction {
        FocusDirection::Up => index.checked_sub(1).and_then(|index| items.get(index)),
        FocusDirection::Down => items.get(index + 1),
        _ => None,
    }
    .copied()
}

fn containing_list(
    focused_entity: Entity,
    candidates: &Query<(
        &UiFocusNav,
        &UiElementKind,
        Has<DisabledUiElement>,
        Option<&Node>,
        Option<&UiScrollArea>,
    )>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let mut current = focused_entity;
    loop {
        let parent = parents.get(current).ok()?.0;
        if candidates
            .get(parent)
            .is_ok_and(|(_, kind, disabled, node, scroll_area)| {
                *kind == UiElementKind::List && focus_available(*kind, disabled, node, scroll_area)
            })
        {
            return Some(parent);
        }
        current = parent;
    }
}

fn focus_available(
    kind: UiElementKind,
    disabled: bool,
    node: Option<&Node>,
    scroll_area: Option<&UiScrollArea>,
) -> bool {
    if disabled {
        return false;
    }

    if kind != UiElementKind::ScrollBar {
        return true;
    }

    if node.is_some_and(|node| node.display == Display::None) {
        return false;
    }

    scroll_area.is_none_or(|area| area.max_offset > 0.0)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FocusDirection {
    Up,
    Right,
    Down,
    Left,
}

impl FocusDirection {
    fn target(self, nav: &UiFocusNav) -> Entity {
        match self {
            FocusDirection::Up => nav.up,
            FocusDirection::Right => nav.right,
            FocusDirection::Down => nav.down,
            FocusDirection::Left => nav.left,
        }
    }
}
