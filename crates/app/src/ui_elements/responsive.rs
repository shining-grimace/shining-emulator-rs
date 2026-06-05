use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::ui_elements::interactions::{
    DisabledUiElement, FocusedUiElement, InitialFocus, UiFocusId, UiFocusNav, UiFocusNavIds,
};

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct ResponsiveScreenPadding {
    pub landscape: f32,
    pub portrait: f32,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct ResponsiveColumns {
    pub gap: f32,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct ResponsiveFieldRow {
    pub gap: f32,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct ResponsiveButtonRow {
    pub gap: f32,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct ResponsivePercentWidth {
    pub landscape: f32,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct ResponsivePxWidth {
    pub landscape: f32,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct ResponsiveFlexWidth {
    pub landscape: f32,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct ResponsiveLandscapeOnly;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct ResponsivePortraitOnly;

pub struct ResponsiveUiPlugin;

impl Plugin for ResponsiveUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, apply_responsive_layout)
            .add_systems(
                PreUpdate,
                apply_responsive_display.after(apply_responsive_layout),
            )
            .add_systems(
                PreUpdate,
                rebind_visible_focus_nav.after(apply_responsive_display),
            );
    }
}

fn apply_responsive_layout(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: ParamSet<(
        Query<(&ResponsiveScreenPadding, &mut Node)>,
        Query<(&ResponsiveColumns, &mut Node), Without<ResponsiveFieldRow>>,
        Query<(&ResponsiveFieldRow, &mut Node), Without<ResponsiveColumns>>,
        Query<(&ResponsiveButtonRow, &mut Node), Without<ResponsiveColumns>>,
        Query<(&ResponsivePercentWidth, &mut Node), Without<ResponsivePxWidth>>,
        Query<(&ResponsivePxWidth, &mut Node), Without<ResponsivePercentWidth>>,
        Query<(&ResponsiveFlexWidth, &mut Node)>,
    )>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let portrait = window.width() < window.height();

    for (config, mut node) in &mut nodes.p0() {
        let padding = if portrait {
            config.portrait
        } else {
            config.landscape
        };
        node.padding = UiRect::all(px(padding));
    }

    for (config, mut node) in &mut nodes.p1() {
        if portrait {
            node.flex_direction = FlexDirection::Column;
            node.column_gap = px(0.0);
            node.row_gap = px(config.gap);
            node.align_items = AlignItems::Stretch;
            node.justify_content = JustifyContent::FlexStart;
        } else {
            node.flex_direction = FlexDirection::Row;
            node.column_gap = px(config.gap);
            node.row_gap = px(0.0);
            node.align_items = AlignItems::Stretch;
            node.justify_content = JustifyContent::FlexStart;
        }
    }

    for (config, mut node) in &mut nodes.p2() {
        if portrait {
            node.flex_direction = FlexDirection::Column;
            node.column_gap = px(0.0);
            node.row_gap = px(config.gap * 0.5);
            node.align_items = AlignItems::Stretch;
            node.justify_content = JustifyContent::FlexStart;
        } else {
            node.flex_direction = FlexDirection::Row;
            node.column_gap = px(config.gap);
            node.row_gap = px(0.0);
            node.align_items = AlignItems::Center;
            node.justify_content = JustifyContent::SpaceBetween;
        }
    }

    for (config, mut node) in &mut nodes.p3() {
        if portrait {
            node.flex_direction = FlexDirection::Column;
            node.column_gap = px(0.0);
            node.row_gap = px(config.gap * 0.5);
            node.align_items = AlignItems::FlexEnd;
            node.justify_content = JustifyContent::FlexStart;
        } else {
            node.flex_direction = FlexDirection::Row;
            node.column_gap = px(config.gap);
            node.row_gap = px(0.0);
            node.align_items = AlignItems::Center;
            node.justify_content = JustifyContent::FlexEnd;
        }
    }

    for (config, mut node) in &mut nodes.p4() {
        if portrait {
            fill_portrait_width(&mut node);
        } else {
            node.width = percent(config.landscape);
            node.flex_grow = 0.0;
            node.flex_shrink = 1.0;
            node.min_width = px(0.0);
        }
    }

    for (config, mut node) in &mut nodes.p5() {
        if portrait {
            node.width = percent(100);
            node.min_width = px(0.0);
        } else {
            node.width = px(config.landscape);
            node.flex_grow = 0.0;
            node.flex_shrink = 0.0;
        }
    }

    for (config, mut node) in &mut nodes.p6() {
        if portrait {
            fill_portrait_width(&mut node);
        } else {
            node.width = px(0.0);
            node.flex_grow = config.landscape;
            node.flex_shrink = 1.0;
            node.min_width = px(0.0);
        }
    }
}

fn apply_responsive_display(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut landscape_nodes: Query<
        &mut Node,
        (
            With<ResponsiveLandscapeOnly>,
            Without<ResponsivePortraitOnly>,
        ),
    >,
    mut portrait_nodes: Query<
        &mut Node,
        (
            With<ResponsivePortraitOnly>,
            Without<ResponsiveLandscapeOnly>,
        ),
    >,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let portrait = window.width() < window.height();

    for mut node in &mut landscape_nodes {
        node.display = if portrait {
            Display::None
        } else {
            Display::Flex
        };
    }

    for mut node in &mut portrait_nodes {
        node.display = if portrait {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn fill_portrait_width(node: &mut Node) {
    node.width = percent(100);
    node.flex_grow = 0.0;
    node.flex_shrink = 0.0;
    node.min_width = px(0.0);
}

fn rebind_visible_focus_nav(
    ids: Query<(Entity, &UiFocusId)>,
    mut navs: Query<(&UiFocusNavIds, &mut UiFocusNav)>,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
    focused: Query<Entity, With<FocusedUiElement>>,
    initial_focus: Query<(Entity, &InitialFocus), Without<DisabledUiElement>>,
    mut commands: Commands,
) {
    let target_entities = ids
        .iter()
        .filter(|(entity, _)| entity_visible(*entity, &nodes, &parents))
        .map(|(entity, target)| (target.id, entity))
        .collect::<Vec<_>>();
    let target = |id| {
        if id == crate::ui_elements::interactions::UI_FOCUS_NONE {
            return Entity::PLACEHOLDER;
        }
        target_entities
            .iter()
            .find_map(|(target_id, entity)| (*target_id == id).then_some(*entity))
            .unwrap_or(Entity::PLACEHOLDER)
    };

    for (nav_ids, mut nav) in &mut navs {
        *nav = UiFocusNav {
            up: target(nav_ids.up),
            right: target(nav_ids.right),
            down: target(nav_ids.down),
            left: target(nav_ids.left),
        };
    }

    let mut visible_focus_exists = false;
    for entity in &focused {
        if !entity_visible(entity, &nodes, &parents) {
            commands.entity(entity).remove::<FocusedUiElement>();
        } else {
            visible_focus_exists = true;
        }
    }

    if !visible_focus_exists {
        if let Some((entity, _)) = initial_focus
            .iter()
            .find(|(entity, initial)| initial.enabled && entity_visible(*entity, &nodes, &parents))
        {
            commands.entity(entity).insert(FocusedUiElement);
        }
    }
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
