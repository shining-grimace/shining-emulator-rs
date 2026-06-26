use bevy::prelude::*;

use crate::ui_elements::list_view::VirtualListRow;

use super::list_view::SuppressListItemFocusRedirect;
use super::picking::{HoveredUiElement, UiPointerClicked};
use super::scroll::UiScrollArea;
use super::ui_input::{UiInputCapture, UiInputDirection, UiInputState};
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

impl LastFocusedUiElement {
    pub fn clear(&mut self) {
        self.entity = None;
    }
}

pub(super) fn ensure_initial_focus(
    mut commands: Commands,
    focused: Query<Entity, With<FocusedUiElement>>,
    candidates: Query<
        (Entity, &InitialFocus),
        (
            Added<InitialFocus>,
            With<UiFocusNav>,
            Without<DisabledUiElement>,
        ),
    >,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
) {
    let Some((target, _)) = candidates.iter().find(|(entity, initial_focus)| {
        initial_focus.enabled && entity_visible(*entity, &nodes, &parents)
    }) else {
        return;
    };

    for entity in &focused {
        if entity != target {
            commands.entity(entity).remove::<FocusedUiElement>();
        }
    }
    commands.entity(target).insert(FocusedUiElement);
}

pub(super) fn restore_focus_from_input(
    mut commands: Commands,
    input: Res<UiInputState>,
    focused: Query<(), With<FocusedUiElement>>,
    added_initial_focus: Query<
        (Entity, &InitialFocus),
        (
            Added<InitialFocus>,
            With<UiFocusNav>,
            Without<DisabledUiElement>,
        ),
    >,
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
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
    virtual_rows: Query<(), With<VirtualListRow>>,
    mut last_focused: ResMut<LastFocusedUiElement>,
) {
    if !focused.is_empty() || !input.focus_recovery_requested() {
        return;
    }

    let target = added_initial_focus
        .iter()
        .find_map(|(entity, initial)| {
            (initial.enabled && entity_visible(entity, &nodes, &parents)).then_some(entity)
        })
        .and_then(|entity| focusable_entity(entity, &candidates, &nodes, &parents))
        .or_else(|| {
            last_focused.entity.and_then(|entity| {
                if virtual_rows.contains(entity) {
                    containing_recoverable_list(entity, &candidates, &nodes, &parents)
                } else {
                    focusable_entity(entity, &candidates, &nodes, &parents)
                }
            })
        })
        .or_else(|| default_focus_target(&candidates, &nodes, &parents))
        .or_else(|| initial_focus_target(&candidates, &nodes, &parents));

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
    input: Res<UiInputState>,
    focused: Query<(Entity, &UiFocusNav, &UiElementKind), With<FocusedUiElement>>,
    candidates: Query<(
        &UiFocusNav,
        &UiElementKind,
        Has<DisabledUiElement>,
        Option<&Node>,
        Option<&UiScrollArea>,
    )>,
    child_query: Query<&Children>,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
) {
    let Some(request) = requested_target(&input, &focused) else {
        return;
    };

    let target_entity = if request.target == Entity::PLACEHOLDER {
        resolve_placeholder_target(
            request.focused_entity,
            request.direction,
            request.kind,
            &candidates,
            &child_query,
            &nodes,
            &parents,
        )
    } else {
        resolve_target(
            request.target,
            request.direction,
            &candidates,
            &child_query,
            &nodes,
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
    capture: Res<UiInputCapture>,
    focusable: Query<&UiElementKind, (With<UiFocusNav>, Without<DisabledUiElement>)>,
    hovered: Query<Entity, With<HoveredUiElement>>,
    focused: Query<Entity, With<FocusedUiElement>>,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
) {
    if capture.active {
        for _ in clicked.read() {}
        return;
    }

    let clicked = clicked.read().map(|click| click.entity).collect::<Vec<_>>();

    if mouse_buttons.just_pressed(MouseButton::Left)
        && !clicked.iter().any(|entity| focusable.get(*entity).is_ok())
    {
        if hovered.iter().next().is_some() {
            return;
        }

        for entity in &focused {
            commands.entity(entity).remove::<FocusedUiElement>();
        }
        return;
    }

    for entity in clicked {
        let Ok(kind) = focusable.get(entity) else {
            continue;
        };
        if !entity_visible(entity, &nodes, &parents) {
            continue;
        }
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
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    candidates
        .get(entity)
        .is_ok_and(|(entity, kind, _, _, disabled, _, scroll_area)| {
            focus_available(entity, *kind, disabled, nodes, parents, scroll_area)
        })
        .then_some(entity)
}

fn containing_recoverable_list(
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
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let mut current = entity;
    while let Ok(parent) = parents.get(current) {
        let parent_entity = parent.0;
        if candidates
            .get(parent_entity)
            .is_ok_and(|(_, kind, _, _, disabled, _, scroll_area)| {
                *kind == UiElementKind::List
                    && focus_available(parent_entity, *kind, disabled, nodes, parents, scroll_area)
            })
        {
            return Some(parent_entity);
        }
        current = parent_entity;
    }

    None
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
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    candidates
        .iter()
        .find_map(|(entity, kind, _, default, disabled, _, scroll_area)| {
            (default && focus_available(entity, *kind, disabled, nodes, parents, scroll_area))
                .then_some(entity)
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
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    candidates
        .iter()
        .find_map(|(entity, kind, initial, _, disabled, _, scroll_area)| {
            initial
                .is_some_and(|initial| initial.enabled)
                .then_some(entity)
                .filter(|entity| {
                    focus_available(*entity, *kind, disabled, nodes, parents, scroll_area)
                })
        })
}

fn requested_target(
    input: &UiInputState,
    focused: &Query<(Entity, &UiFocusNav, &UiElementKind), With<FocusedUiElement>>,
) -> Option<FocusRequest> {
    let (entity, nav, kind) = focused.iter().next()?;

    let direction = match input.direction()? {
        UiInputDirection::Up => FocusDirection::Up,
        UiInputDirection::Right if *kind != UiElementKind::TextInput => FocusDirection::Right,
        UiInputDirection::Down => FocusDirection::Down,
        UiInputDirection::Left if *kind != UiElementKind::TextInput => FocusDirection::Left,
        _ => return None,
    };
    let target = direction.target(nav);

    Some(FocusRequest {
        focused_entity: entity,
        kind: *kind,
        direction,
        target,
    })
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
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let mut next = target;
    let mut hops_remaining = candidates.iter().len();

    while next != Entity::PLACEHOLDER {
        if hops_remaining == 0 {
            return None;
        }
        hops_remaining -= 1;

        let Ok((nav, kind, disabled, _, scroll_area)) = candidates.get(next) else {
            return None;
        };

        if focus_available(next, *kind, disabled, nodes, parents, scroll_area) {
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
                nodes,
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
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    match (kind, direction) {
        (UiElementKind::ScrollBar, FocusDirection::Left) => {
            let children = child_query.get(focused_entity).ok()?;
            first_focusable_descendant(children, candidates, child_query, nodes, parents)
        }
        (UiElementKind::MultiSelectOption, FocusDirection::Up | FocusDirection::Down) => {
            adjacent_multi_select_option(
                focused_entity,
                direction,
                candidates,
                child_query,
                nodes,
                parents,
            )
        }
        (UiElementKind::ListItem, FocusDirection::Up | FocusDirection::Down) => adjacent_list_item(
            focused_entity,
            direction,
            candidates,
            child_query,
            nodes,
            parents,
        ),
        (UiElementKind::ListItem, FocusDirection::Left | FocusDirection::Right) => {
            containing_list(focused_entity, candidates, nodes, parents)
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
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    for child in children {
        if candidates
            .get(*child)
            .is_ok_and(|(_, kind, disabled, _, scroll_area)| {
                focus_available(*child, *kind, disabled, nodes, parents, scroll_area)
            })
        {
            return Some(*child);
        }

        if let Ok(grandchildren) = child_query.get(*child) {
            if let Some(entity) =
                first_focusable_descendant(grandchildren, candidates, child_query, nodes, parents)
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
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let parent = parents.get(focused_entity).ok()?.0;
    let siblings = child_query.get(parent).ok()?;
    let options = siblings
        .iter()
        .filter_map(|sibling| {
            candidates
                .get(sibling)
                .is_ok_and(|(_, kind, disabled, _, scroll_area)| {
                    *kind == UiElementKind::MultiSelectOption
                        && focus_available(sibling, *kind, disabled, nodes, parents, scroll_area)
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
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let parent = parents.get(focused_entity).ok()?.0;
    let siblings = child_query.get(parent).ok()?;
    let items = siblings
        .iter()
        .filter_map(|sibling| {
            candidates
                .get(sibling)
                .is_ok_and(|(_, kind, disabled, _, scroll_area)| {
                    *kind == UiElementKind::ListItem
                        && focus_available(sibling, *kind, disabled, nodes, parents, scroll_area)
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
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let mut current = focused_entity;
    loop {
        let parent = parents.get(current).ok()?.0;
        if candidates
            .get(parent)
            .is_ok_and(|(_, kind, disabled, _, scroll_area)| {
                *kind == UiElementKind::List
                    && focus_available(parent, *kind, disabled, nodes, parents, scroll_area)
            })
        {
            return Some(parent);
        }
        current = parent;
    }
}

fn focus_available(
    entity: Entity,
    kind: UiElementKind,
    disabled: bool,
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
    scroll_area: Option<&UiScrollArea>,
) -> bool {
    if disabled {
        return false;
    }

    if !entity_visible(entity, nodes, parents) {
        return false;
    }

    if kind != UiElementKind::ScrollBar {
        return true;
    }

    scroll_area.is_none_or(|area| area.max_offset > 0.0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newly_added_initial_focus_replaces_existing_focus() {
        let mut world = World::new();
        let previous = world.spawn(FocusedUiElement).id();
        let target = world
            .spawn((InitialFocus { enabled: true }, UiFocusNav::default()))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(ensure_initial_focus);
        schedule.run(&mut world);

        assert!(world.get::<FocusedUiElement>(previous).is_none());
        assert!(world.get::<FocusedUiElement>(target).is_some());
    }
}
