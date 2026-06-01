use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::ui_elements::list_view::{VirtualListContent, VirtualListRow};

use super::focus::FocusedUiElement;
use super::multi_select::UiMultiSelectOption;
use super::picking::{DraggableUiElement, HoveredUiElement};
use super::tree::contains_entity;
use super::visual_state::UiElementKind;

const WHEEL_SCROLL_PIXELS: f32 = 48.0;
const KEY_SCROLL_PIXELS: f32 = 48.0;
const KEY_SCROLL_REPEAT_DELAY_SECONDS: f32 = 0.32;
const KEY_SCROLL_REPEAT_SECONDS: f32 = 0.045;
const SCROLLBAR_VERTICAL_PADDING: f32 = 12.0;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct AutoScrollFocusedChild;

#[derive(Clone, Copy, Component, Debug, Default)]
pub struct SuppressFocusAutoScroll;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiScrollArea {
    pub offset: f32,
    pub max_offset: f32,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiPopupScrollArea {
    pub option_count: usize,
    pub max_visible_options: usize,
    pub option_height: f32,
    pub option_gap: f32,
}

impl UiPopupScrollArea {
    pub fn has_overflow(self) -> bool {
        self.option_count > self.max_visible_options
    }

    pub fn max_offset(self) -> f32 {
        (self.content_height() - self.visible_height()).max(0.0)
    }

    pub fn visible_height(self) -> f32 {
        let visible_options = self.option_count.min(self.max_visible_options).max(1);
        visible_options as f32 * self.option_height
            + visible_options.saturating_sub(1) as f32 * self.option_gap
    }

    fn content_height(self) -> f32 {
        self.option_count as f32 * self.option_height
            + self.option_count.saturating_sub(1) as f32 * self.option_gap
    }

    fn option_bounds(self, option_index: usize) -> (f32, f32) {
        let top = option_index as f32 * (self.option_height + self.option_gap);
        (top, top + self.option_height)
    }
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiScrollContent;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiScrollbar;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiScrollThumb {
    pub height: f32,
    pub travel: f32,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiScrollThumbColors {
    pub primary: Color,
    pub secondary: Color,
}

#[derive(Default)]
pub(super) struct KeyScrollRepeatState {
    direction: Option<KeyScrollDirection>,
    held_for_seconds: f32,
    repeat_for_seconds: f32,
}

#[derive(Default, Resource)]
pub(super) struct ScrollThumbDragState {
    active: bool,
    released_this_frame: bool,
}

pub(super) fn update_dynamic_scroll_metrics(
    mut areas: Query<(
        Entity,
        &ComputedNode,
        &mut UiScrollArea,
        Option<&UiPopupScrollArea>,
        &Children,
    )>,
    scroll_contents: Query<(), With<UiScrollContent>>,
    mut scroll_nodes: ParamSet<(
        Query<(&ComputedNode, &mut Node, Has<VirtualListContent>), With<UiScrollContent>>,
        Query<(&mut UiScrollThumb, &mut Node, &ComputedNode)>,
        Query<&mut Node, With<UiScrollbar>>,
        Query<&ComputedNode, With<UiScrollbar>>,
    )>,
    child_query: Query<&Children>,
) {
    for (_, area_node, mut area, popup_scroll, children) in &mut areas {
        let content_height = popup_scroll
            .copied()
            .map(UiPopupScrollArea::content_height)
            .or_else(|| scroll_content_height(children, &scroll_nodes.p0(), &child_query));
        let Some(content_height) = content_height else {
            continue;
        };

        let viewport_height = popup_scroll
            .copied()
            .map(UiPopupScrollArea::visible_height)
            .unwrap_or_else(|| logical_height(area_node));
        let max_offset = (content_height - viewport_height).max(0.0);
        if area.max_offset != max_offset {
            area.max_offset = max_offset;
        }
        let offset = area.offset.clamp(0.0, area.max_offset);
        if area.offset != offset {
            area.offset = offset;
        }

        let scrollbar_heights =
            collect_scrollbar_heights(children, &scroll_contents, &scroll_nodes.p3(), &child_query);

        update_scroll_content_offset(children, area.offset, &mut scroll_nodes.p0(), &child_query);
        update_scrollbar_visibility(
            children,
            popup_scroll
                .copied()
                .map(UiPopupScrollArea::has_overflow)
                .unwrap_or(area.max_offset > 0.0),
            &scroll_contents,
            &mut scroll_nodes.p2(),
            &child_query,
        );
        update_dynamic_scroll_thumb(
            children,
            viewport_height,
            area.offset,
            area.max_offset,
            &mut scroll_nodes.p1(),
            &scrollbar_heights,
            &scroll_contents,
            &child_query,
        );
    }
}

pub(super) fn update_scroll_thumb_colours(
    focused: Query<&UiElementKind, With<FocusedUiElement>>,
    parents: Query<&ChildOf>,
    mut thumbs: Query<(Entity, &UiScrollThumbColors, &mut BackgroundColor), With<UiScrollThumb>>,
) {
    for (entity, colours, mut background) in &mut thumbs {
        background.0 = if has_focused_scrollbar_ancestor(entity, &parents, &focused) {
            colours.secondary
        } else {
            colours.primary
        };
    }
}

fn update_scrollbar_visibility(
    children: &Children,
    visible: bool,
    scroll_contents: &Query<(), With<UiScrollContent>>,
    scrollbar_nodes: &mut Query<&mut Node, With<UiScrollbar>>,
    child_query: &Query<&Children>,
) {
    for child in children {
        if scroll_contents.contains(*child) {
            continue;
        }

        if let Ok(mut node) = scrollbar_nodes.get_mut(*child) {
            let display = if visible {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            update_scrollbar_visibility(
                grandchildren,
                visible,
                scroll_contents,
                scrollbar_nodes,
                child_query,
            );
        }
    }
}

pub(super) fn setup_scroll_drag_tracking(
    mut commands: Commands,
    scroll_areas: Query<Entity, Added<UiScrollArea>>,
    thumbs: Query<Entity, (Added<UiScrollThumb>, With<DraggableUiElement>)>,
) {
    for entity in &scroll_areas {
        commands.entity(entity).observe(drag_scroll_area);
    }

    for entity in &thumbs {
        commands
            .entity(entity)
            .observe(start_scroll_thumb_drag)
            .observe(drag_scroll_thumb)
            .observe(end_scroll_thumb_drag);
    }
}

pub(super) fn scroll_focused_scrollbar_by_keys(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    focused: Query<(Entity, &UiElementKind), With<FocusedUiElement>>,
    mut areas: Query<(Entity, &mut UiScrollArea, &Children)>,
    scroll_contents: Query<(), With<UiScrollContent>>,
    mut scroll_nodes: ParamSet<(
        Query<(&mut Node, Has<VirtualListContent>), With<UiScrollContent>>,
        Query<(&UiScrollThumb, &mut Node)>,
    )>,
    virtual_rows: Query<(), With<VirtualListRow>>,
    child_query: Query<&Children>,
    mut repeat: Local<KeyScrollRepeatState>,
) {
    let Some((focused_entity, _)) = focused
        .iter()
        .find(|(_, kind)| **kind == UiElementKind::ScrollBar)
    else {
        *repeat = KeyScrollRepeatState::default();
        return;
    };

    let Some(delta) = key_scroll_delta(&time, &keys, &mut repeat) else {
        return;
    };

    let target_area = {
        let mut target_area = None;
        for (area_entity, _, children) in &areas {
            if area_entity == focused_entity
                || contains_owned_scroll_entity(
                    children,
                    focused_entity,
                    &scroll_contents,
                    &child_query,
                )
            {
                target_area = Some(area_entity);
                break;
            }
        }
        target_area
    };

    let Some(target_area) = target_area else {
        return;
    };
    let Ok((_, mut area, children)) = areas.get_mut(target_area) else {
        return;
    };

    area.offset = (area.offset + delta).clamp(0.0, area.max_offset);
    {
        let mut content_nodes = scroll_nodes.p0();
        apply_scroll_offset(
            children,
            area.offset,
            &mut content_nodes,
            &virtual_rows,
            &child_query,
        );
    }
    {
        let mut thumb_nodes = scroll_nodes.p1();
        apply_scroll_thumb_offset(
            children,
            area.offset,
            area.max_offset,
            &scroll_contents,
            &mut thumb_nodes,
            &child_query,
        );
    }
}

pub(super) fn scroll_areas(
    mut wheel_events: MessageReader<MouseWheel>,
    mut areas: Query<(
        Entity,
        &mut UiScrollArea,
        Has<HoveredUiElement>,
        Has<UiPopupScrollArea>,
        &Children,
    )>,
    scroll_contents: Query<(), With<UiScrollContent>>,
    mut scroll_nodes: ParamSet<(
        Query<(&mut Node, Has<VirtualListContent>), With<UiScrollContent>>,
        Query<(&UiScrollThumb, &mut Node)>,
    )>,
    pointer_states: Query<(), With<HoveredUiElement>>,
    virtual_rows: Query<(), With<VirtualListRow>>,
    child_query: Query<&Children>,
    parents: Query<&ChildOf>,
) {
    let wheel_delta = wheel_events.read().map(|event| event.y).sum::<f32>();
    if wheel_delta == 0.0 {
        return;
    }

    let target = {
        let mut popup_hover = None;
        let mut regular_hover = None;
        for (entity, area, hovered, popup_scroll, children) in &mut areas {
            if !hovered && !has_hovered_descendant(children, &pointer_states, &child_query) {
                continue;
            }
            if !can_scroll_wheel(&area, wheel_delta) {
                continue;
            }

            let candidate = (entity, hierarchy_depth(entity, &parents));
            if popup_scroll {
                popup_hover = deeper_scroll_candidate(popup_hover, candidate);
            } else {
                regular_hover = deeper_scroll_candidate(regular_hover, candidate);
            }
        }
        popup_hover.or(regular_hover).map(|(entity, _)| entity)
    };

    let Some(target) = target else {
        return;
    };
    let Ok((_, mut area, _, _, children)) = areas.get_mut(target) else {
        return;
    };

    area.offset = (area.offset - wheel_delta * WHEEL_SCROLL_PIXELS).clamp(0.0, area.max_offset);
    {
        let mut content_nodes = scroll_nodes.p0();
        apply_scroll_offset(
            children,
            area.offset,
            &mut content_nodes,
            &virtual_rows,
            &child_query,
        );
    }
    {
        let mut thumb_nodes = scroll_nodes.p1();
        apply_scroll_thumb_offset(
            children,
            area.offset,
            area.max_offset,
            &scroll_contents,
            &mut thumb_nodes,
            &child_query,
        );
    }
}

fn drag_scroll_area(
    mut event: On<Pointer<Drag>>,
    thumb_targets: Query<(), With<UiScrollThumb>>,
    mut areas: Query<(Entity, &mut UiScrollArea, &Children)>,
    scroll_contents: Query<(), With<UiScrollContent>>,
    mut scroll_nodes: ParamSet<(
        Query<(&mut Node, Has<VirtualListContent>), With<UiScrollContent>>,
        Query<(&UiScrollThumb, &mut Node)>,
    )>,
    virtual_rows: Query<(), With<VirtualListRow>>,
    child_query: Query<&Children>,
) {
    if thumb_targets.contains(event.original_event_target()) {
        return;
    }

    event.propagate(false);
    scroll_area_by_delta(
        event.entity,
        -event.delta.y,
        &mut areas,
        &scroll_contents,
        &mut scroll_nodes,
        &virtual_rows,
        &child_query,
    );
}

fn can_scroll_wheel(area: &UiScrollArea, wheel_delta: f32) -> bool {
    if area.max_offset <= 0.0 {
        return false;
    }

    let offset_delta = -wheel_delta * WHEEL_SCROLL_PIXELS;
    (offset_delta < 0.0 && area.offset > 0.0)
        || (offset_delta > 0.0 && area.offset < area.max_offset)
}

fn deeper_scroll_candidate(
    current: Option<(Entity, usize)>,
    candidate: (Entity, usize),
) -> Option<(Entity, usize)> {
    match current {
        Some(current) if current.1 >= candidate.1 => Some(current),
        _ => Some(candidate),
    }
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

fn start_scroll_thumb_drag(
    mut event: On<Pointer<DragStart>>,
    mut state: ResMut<ScrollThumbDragState>,
) {
    event.propagate(false);
    state.active = true;
    state.released_this_frame = false;
}

fn drag_scroll_thumb(
    mut event: On<Pointer<Drag>>,
    mut state: ResMut<ScrollThumbDragState>,
    mut areas: Query<(Entity, &mut UiScrollArea, &Children)>,
    scroll_contents: Query<(), With<UiScrollContent>>,
    mut scroll_nodes: ParamSet<(
        Query<(&mut Node, Has<VirtualListContent>), With<UiScrollContent>>,
        Query<(&UiScrollThumb, &mut Node)>,
    )>,
    thumbs: Query<&UiScrollThumb, With<DraggableUiElement>>,
    virtual_rows: Query<(), With<VirtualListRow>>,
    child_query: Query<&Children>,
) {
    event.propagate(false);
    state.active = true;
    state.released_this_frame = false;

    let Ok(thumb) = thumbs.get(event.entity) else {
        return;
    };

    for (area_entity, area, children) in &mut areas {
        if !contains_owned_scroll_entity(children, event.entity, &scroll_contents, &child_query) {
            continue;
        }
        if thumb.travel <= 0.0 || area.max_offset <= 0.0 {
            return;
        }

        let delta = event.delta.y / thumb.travel * area.max_offset;
        scroll_area_by_delta(
            area_entity,
            delta,
            &mut areas,
            &scroll_contents,
            &mut scroll_nodes,
            &virtual_rows,
            &child_query,
        );
        return;
    }
}

fn end_scroll_thumb_drag(mut event: On<Pointer<DragEnd>>, mut state: ResMut<ScrollThumbDragState>) {
    event.propagate(false);
    state.active = false;
    state.released_this_frame = true;
}

pub(super) fn keep_focused_list_item_visible(
    mut commands: Commands,
    drag_state: Res<ScrollThumbDragState>,
    focused_items: Query<(Entity, &ComputedNode, &UiGlobalTransform), With<FocusedUiElement>>,
    added_focus: Query<(), Added<FocusedUiElement>>,
    suppressed_focus: Query<(), With<SuppressFocusAutoScroll>>,
    focused_options: Query<&UiMultiSelectOption, With<FocusedUiElement>>,
    mut areas: Query<
        (
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &mut UiScrollArea,
            Option<&UiPopupScrollArea>,
            &Children,
        ),
        With<AutoScrollFocusedChild>,
    >,
    scroll_contents: Query<(), With<UiScrollContent>>,
    mut scroll_nodes: ParamSet<(
        Query<(&mut Node, Has<VirtualListContent>), With<UiScrollContent>>,
        Query<(&UiScrollThumb, &mut Node)>,
    )>,
    virtual_rows: Query<(), With<VirtualListRow>>,
    child_query: Query<&Children>,
) {
    if drag_state.active || drag_state.released_this_frame {
        return;
    }

    let Some((focused_entity, focused_node, focused_transform)) = focused_items.iter().next()
    else {
        return;
    };
    if added_focus.get(focused_entity).is_err() {
        return;
    }
    if suppressed_focus.get(focused_entity).is_ok() {
        commands
            .entity(focused_entity)
            .remove::<SuppressFocusAutoScroll>();
        return;
    }
    let focused_option = focused_options.get(focused_entity).ok();

    for (_, area_node, area_transform, mut area, popup_scroll, children) in &mut areas {
        if !contains_entity(children, focused_entity, &child_query) {
            continue;
        }
        if focused_option.is_some() && popup_scroll.is_none() {
            continue;
        }

        let next_offset = popup_scroll
            .and_then(|popup_scroll| {
                focused_option
                    .map(|option| popup_option_scroll_offset(*popup_scroll, option, area.offset))
            })
            .unwrap_or_else(|| {
                focused_child_scroll_offset(
                    focused_node,
                    focused_transform,
                    area_node,
                    area_transform,
                    area.offset,
                )
            })
            .clamp(0.0, area.max_offset);

        if (next_offset - area.offset).abs() <= f32::EPSILON {
            return;
        }

        area.offset = next_offset;
        {
            let mut content_nodes = scroll_nodes.p0();
            apply_scroll_offset(
                children,
                area.offset,
                &mut content_nodes,
                &virtual_rows,
                &child_query,
            );
        }
        {
            let mut thumb_nodes = scroll_nodes.p1();
            apply_scroll_thumb_offset(
                children,
                area.offset,
                area.max_offset,
                &scroll_contents,
                &mut thumb_nodes,
                &child_query,
            );
        }
        return;
    }
}

pub(super) fn clear_focus_auto_scroll_suppression(
    mut commands: Commands,
    suppressed: Query<Entity, With<SuppressFocusAutoScroll>>,
) {
    for entity in &suppressed {
        commands.entity(entity).remove::<SuppressFocusAutoScroll>();
    }
}

pub(super) fn clear_scroll_thumb_drag_release(mut state: ResMut<ScrollThumbDragState>) {
    state.released_this_frame = false;
}

fn scroll_area_by_delta(
    target: Entity,
    delta: f32,
    areas: &mut Query<(Entity, &mut UiScrollArea, &Children)>,
    scroll_contents: &Query<(), With<UiScrollContent>>,
    scroll_nodes: &mut ParamSet<(
        Query<(&mut Node, Has<VirtualListContent>), With<UiScrollContent>>,
        Query<(&UiScrollThumb, &mut Node)>,
    )>,
    virtual_rows: &Query<(), With<VirtualListRow>>,
    child_query: &Query<&Children>,
) {
    if delta == 0.0 {
        return;
    }

    let Ok((_, mut area, children)) = areas.get_mut(target) else {
        return;
    };
    let next_offset = (area.offset + delta).clamp(0.0, area.max_offset);
    if (next_offset - area.offset).abs() <= f32::EPSILON {
        return;
    }

    area.offset = next_offset;
    {
        let mut content_nodes = scroll_nodes.p0();
        apply_scroll_offset(
            children,
            area.offset,
            &mut content_nodes,
            virtual_rows,
            child_query,
        );
    }
    {
        let mut thumb_nodes = scroll_nodes.p1();
        apply_scroll_thumb_offset(
            children,
            area.offset,
            area.max_offset,
            scroll_contents,
            &mut thumb_nodes,
            child_query,
        );
    }
}

fn popup_option_scroll_offset(
    popup_scroll: UiPopupScrollArea,
    option: &UiMultiSelectOption,
    current_offset: f32,
) -> f32 {
    let (option_top, option_bottom) = popup_scroll.option_bounds(option.option_index);
    let viewport_top = current_offset;
    let viewport_bottom = current_offset + popup_scroll.visible_height();

    if option_top < viewport_top {
        option_top
    } else if option_bottom > viewport_bottom {
        option_bottom - popup_scroll.visible_height()
    } else {
        current_offset
    }
}

fn focused_child_scroll_offset(
    focused_node: &ComputedNode,
    focused_transform: &UiGlobalTransform,
    area_node: &ComputedNode,
    area_transform: &UiGlobalTransform,
    current_offset: f32,
) -> f32 {
    let (_, _, area_center) = area_transform.to_scale_angle_translation();
    let (_, _, focused_center) = focused_transform.to_scale_angle_translation();
    let area_top = area_center.y - area_node.size().y / 2.0;
    let area_bottom = area_center.y + area_node.size().y / 2.0;
    let focused_top = focused_center.y - focused_node.size().y / 2.0;
    let focused_bottom = focused_center.y + focused_node.size().y / 2.0;

    if focused_top < area_top {
        current_offset - (area_top - focused_top)
    } else if focused_bottom > area_bottom {
        current_offset + (focused_bottom - area_bottom)
    } else {
        current_offset
    }
}

fn scroll_content_height(
    children: &Children,
    content_nodes: &Query<
        (&ComputedNode, &mut Node, Has<VirtualListContent>),
        With<UiScrollContent>,
    >,
    child_query: &Query<&Children>,
) -> Option<f32> {
    for child in children {
        if let Ok((computed, _, _)) = content_nodes.get(*child) {
            return Some(logical_height(computed));
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            if let Some(height) = scroll_content_height(grandchildren, content_nodes, child_query) {
                return Some(height);
            }
        }
    }
    None
}

fn logical_height(node: &ComputedNode) -> f32 {
    node.size().y * node.inverse_scale_factor()
}

fn update_scroll_content_offset(
    children: &Children,
    offset: f32,
    content_nodes: &mut Query<
        (&ComputedNode, &mut Node, Has<VirtualListContent>),
        With<UiScrollContent>,
    >,
    child_query: &Query<&Children>,
) {
    for child in children {
        if let Ok((_, mut node, virtual_content)) = content_nodes.get_mut(*child) {
            if virtual_content {
                continue;
            }

            let top = px(-offset);
            if node.top != top {
                node.top = top;
            }
            continue;
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            update_scroll_content_offset(grandchildren, offset, content_nodes, child_query);
        }
    }
}

fn update_dynamic_scroll_thumb(
    children: &Children,
    viewport_height: f32,
    offset: f32,
    max_offset: f32,
    thumb_nodes: &mut Query<(&mut UiScrollThumb, &mut Node, &ComputedNode)>,
    scrollbar_heights: &[(Entity, f32)],
    scroll_contents: &Query<(), With<UiScrollContent>>,
    child_query: &Query<&Children>,
) {
    let ratio = if max_offset <= 0.0 {
        0.0
    } else {
        offset / max_offset
    };

    for child in children {
        if scroll_contents.contains(*child) {
            continue;
        }

        let viewport_height = scrollbar_heights
            .iter()
            .find_map(|(entity, height)| (*entity == *child).then_some(*height))
            .unwrap_or(viewport_height);

        if let Ok((mut thumb, mut node, _)) = thumb_nodes.get_mut(*child) {
            let max_thumb_height = (viewport_height - SCROLLBAR_VERTICAL_PADDING).max(0.0);
            let thumb_height = thumb.height.min(max_thumb_height);
            let travel = (viewport_height - thumb_height - SCROLLBAR_VERTICAL_PADDING).max(0.0);
            let height = px(thumb_height);
            let top = px(6.0 + travel * ratio);
            if node.height != height {
                node.height = height;
            }
            if thumb.travel != travel {
                thumb.travel = travel;
            }
            if node.top != top {
                node.top = top;
            }
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            update_dynamic_scroll_thumb(
                grandchildren,
                viewport_height,
                offset,
                max_offset,
                thumb_nodes,
                scrollbar_heights,
                scroll_contents,
                child_query,
            );
        }
    }
}

fn collect_scrollbar_heights(
    children: &Children,
    scroll_contents: &Query<(), With<UiScrollContent>>,
    scrollbar_nodes: &Query<&ComputedNode, With<UiScrollbar>>,
    child_query: &Query<&Children>,
) -> Vec<(Entity, f32)> {
    let mut heights = Vec::new();
    collect_scrollbar_heights_recursive(
        children,
        scroll_contents,
        scrollbar_nodes,
        child_query,
        &mut heights,
    );
    heights
}

fn collect_scrollbar_heights_recursive(
    children: &Children,
    scroll_contents: &Query<(), With<UiScrollContent>>,
    scrollbar_nodes: &Query<&ComputedNode, With<UiScrollbar>>,
    child_query: &Query<&Children>,
    heights: &mut Vec<(Entity, f32)>,
) {
    for child in children {
        if scroll_contents.contains(*child) {
            continue;
        }

        if let Ok(node) = scrollbar_nodes.get(*child) {
            heights.push((*child, logical_height(node)));
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            collect_scrollbar_heights_recursive(
                grandchildren,
                scroll_contents,
                scrollbar_nodes,
                child_query,
                heights,
            );
        }
    }
}

fn key_scroll_delta(
    time: &Time,
    keys: &ButtonInput<KeyCode>,
    repeat: &mut KeyScrollRepeatState,
) -> Option<f32> {
    if keys.just_pressed(KeyCode::ArrowUp) {
        repeat.direction = Some(KeyScrollDirection::Up);
        repeat.held_for_seconds = 0.0;
        repeat.repeat_for_seconds = 0.0;
        return Some(-KEY_SCROLL_PIXELS);
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        repeat.direction = Some(KeyScrollDirection::Down);
        repeat.held_for_seconds = 0.0;
        repeat.repeat_for_seconds = 0.0;
        return Some(KEY_SCROLL_PIXELS);
    }

    let Some(direction) = repeat.direction else {
        *repeat = KeyScrollRepeatState::default();
        return None;
    };
    let key = direction.key_code();
    if !keys.pressed(key) {
        *repeat = KeyScrollRepeatState::default();
        return None;
    }

    repeat.held_for_seconds += time.delta_secs();
    if repeat.held_for_seconds < KEY_SCROLL_REPEAT_DELAY_SECONDS {
        return None;
    }

    repeat.repeat_for_seconds += time.delta_secs();
    let mut steps = 0;
    while repeat.repeat_for_seconds >= KEY_SCROLL_REPEAT_SECONDS {
        repeat.repeat_for_seconds -= KEY_SCROLL_REPEAT_SECONDS;
        steps += 1;
    }

    (steps > 0).then_some(direction.delta() * steps as f32)
}

fn has_focused_scrollbar_ancestor(
    entity: Entity,
    parents: &Query<&ChildOf>,
    focused: &Query<&UiElementKind, With<FocusedUiElement>>,
) -> bool {
    let mut current = entity;
    loop {
        if focused
            .get(current)
            .is_ok_and(|kind| *kind == UiElementKind::ScrollBar)
        {
            return true;
        }

        let Ok(parent) = parents.get(current) else {
            return false;
        };
        current = parent.0;
    }
}

fn contains_owned_scroll_entity(
    children: &Children,
    target: Entity,
    scroll_contents: &Query<(), With<UiScrollContent>>,
    child_query: &Query<&Children>,
) -> bool {
    for child in children {
        if scroll_contents.contains(*child) {
            continue;
        }
        if *child == target {
            return true;
        }
        if child_query.get(*child).is_ok_and(|grandchildren| {
            contains_owned_scroll_entity(grandchildren, target, scroll_contents, child_query)
        }) {
            return true;
        }
    }
    false
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum KeyScrollDirection {
    Up,
    Down,
}

impl KeyScrollDirection {
    fn key_code(self) -> KeyCode {
        match self {
            Self::Up => KeyCode::ArrowUp,
            Self::Down => KeyCode::ArrowDown,
        }
    }

    fn delta(self) -> f32 {
        match self {
            Self::Up => -KEY_SCROLL_PIXELS,
            Self::Down => KEY_SCROLL_PIXELS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn popup_scroll() -> UiPopupScrollArea {
        UiPopupScrollArea {
            option_count: 17,
            max_visible_options: 5,
            option_height: 48.0,
            option_gap: 8.0,
        }
    }

    #[test]
    fn popup_option_scroll_offset_scrolls_down_to_reveal_lower_option() {
        let option = UiMultiSelectOption {
            option_index: 5,
            label: "Option".to_string(),
        };

        assert_eq!(
            popup_option_scroll_offset(popup_scroll(), &option, 0.0),
            56.0
        );
    }

    #[test]
    fn popup_option_scroll_offset_scrolls_up_to_reveal_higher_option() {
        let option = UiMultiSelectOption {
            option_index: 1,
            label: "Option".to_string(),
        };

        assert_eq!(
            popup_option_scroll_offset(popup_scroll(), &option, 280.0),
            56.0
        );
    }

    #[test]
    fn popup_option_scroll_offset_keeps_visible_option_stable() {
        let option = UiMultiSelectOption {
            option_index: 4,
            label: "Option".to_string(),
        };

        assert_eq!(
            popup_option_scroll_offset(popup_scroll(), &option, 56.0),
            56.0
        );
    }
}

fn has_hovered_descendant(
    children: &Children,
    pointer_states: &Query<(), With<HoveredUiElement>>,
    child_query: &Query<&Children>,
) -> bool {
    for child in children {
        if pointer_states.get(*child).is_ok() {
            return true;
        }
        if child_query.get(*child).is_ok_and(|grandchildren| {
            has_hovered_descendant(grandchildren, pointer_states, child_query)
        }) {
            return true;
        }
    }
    false
}

fn apply_scroll_offset(
    children: &Children,
    offset: f32,
    content_nodes: &mut Query<(&mut Node, Has<VirtualListContent>), With<UiScrollContent>>,
    virtual_rows: &Query<(), With<VirtualListRow>>,
    child_query: &Query<&Children>,
) {
    for child in children {
        if let Ok((mut node, virtual_content)) = content_nodes.get_mut(*child) {
            if virtual_content
                && child_query.get(*child).is_ok_and(|children| {
                    has_virtual_row_descendant(children, virtual_rows, child_query)
                })
            {
                continue;
            }

            let top = px(-offset);
            if node.top != top {
                node.top = top;
            }
            continue;
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            apply_scroll_offset(
                grandchildren,
                offset,
                content_nodes,
                virtual_rows,
                child_query,
            );
        }
    }
}

fn has_virtual_row_descendant(
    children: &Children,
    virtual_rows: &Query<(), With<VirtualListRow>>,
    child_query: &Query<&Children>,
) -> bool {
    for child in children {
        if virtual_rows.get(*child).is_ok() {
            return true;
        }
        if child_query
            .get(*child)
            .is_ok_and(|children| has_virtual_row_descendant(children, virtual_rows, child_query))
        {
            return true;
        }
    }
    false
}

fn apply_scroll_thumb_offset(
    children: &Children,
    offset: f32,
    max_offset: f32,
    scroll_contents: &Query<(), With<UiScrollContent>>,
    thumb_nodes: &mut Query<(&UiScrollThumb, &mut Node)>,
    child_query: &Query<&Children>,
) {
    let ratio = if max_offset <= 0.0 {
        0.0
    } else {
        offset / max_offset
    };

    for child in children {
        if scroll_contents.contains(*child) {
            continue;
        }

        if let Ok((thumb, mut node)) = thumb_nodes.get_mut(*child) {
            let top = px(6.0 + thumb.travel * ratio);
            if node.top != top {
                node.top = top;
            }
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            apply_scroll_thumb_offset(
                grandchildren,
                offset,
                max_offset,
                scroll_contents,
                thumb_nodes,
                child_query,
            );
        }
    }
}
