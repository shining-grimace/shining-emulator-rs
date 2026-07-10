use bevy::color::Alpha;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::circuit_board::components::{
    CircuitBoardLayer, CircuitNode, CircuitNodeStroke, CircuitSchematicStroke,
};
use crate::circuit_board::constants::{
    CIRCUIT_ELEMENT_Z, CIRCUIT_FADE_SECONDS, CIRCUIT_MAX_OPACITY, CIRCUIT_NODE_Z, CIRCUIT_Z,
};
use crate::circuit_board::effects::CircuitBoardDisplay;
use crate::circuit_board::utils::{
    CIRCUIT_SCREEN_NODES, SegmentRole, active_rect, base_node_rects, base_rect,
    display_node_for_screen, line_geometry, move_rect_toward, move_toward,
    rounded_rect_corner_radius, rounded_rect_stroke_segment, rounded_rect_strokes,
    schematic_segments, screen_has_circuit_board, transformed_base_rect,
};
use crate::settings_transition::SettingsTransition;
use crate::visual_effects::ACTIVE_SCREEN_RECT_ANIMATION_SECONDS;

pub(super) fn update_circuit_board_target(
    state: Res<State<AppState>>,
    mut display: ResMut<CircuitBoardDisplay>,
    transition: Res<SettingsTransition>,
) {
    let screen = transition.circuit_screen().unwrap_or(*state.get());
    if screen_has_circuit_board(screen) {
        display.fade_in_for(screen);
    } else {
        display.fade_out();
    }
}

pub(super) fn spawn_circuit_board(
    mut commands: Commands,
    theme: Res<ActiveTheme>,
    windows: Query<&Window, With<PrimaryWindow>>,
    board_query: Query<(), With<CircuitBoardLayer>>,
) {
    if !board_query.is_empty() {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let window_size = Vec2::new(window.width(), window.height());
    let hidden = theme.primary.with_alpha(0.0);

    for screen in CIRCUIT_SCREEN_NODES {
        let base_rect = base_rect(screen, window_size);
        commands.spawn((
            CircuitBoardLayer,
            CircuitNode {
                screen,
                current_rect: base_rect,
                corner_radius: rounded_rect_corner_radius(base_rect),
                transition_start_rect: None,
            },
        ));

        for part in rounded_rect_strokes() {
            commands.spawn((
                CircuitBoardLayer,
                CircuitNodeStroke { screen, part },
                Sprite::from_color(hidden, Vec2::ONE),
                Transform::from_xyz(0.0, 0.0, CIRCUIT_NODE_Z),
            ));
        }
    }

    let base_rects = base_node_rects(window_size);
    for index in 0..schematic_segments(&base_rects, window_size).len() {
        commands.spawn((
            CircuitBoardLayer,
            CircuitSchematicStroke { index },
            Sprite::from_color(hidden, Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, CIRCUIT_Z),
        ));
    }
}

pub(super) fn animate_circuit_board(
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut display: ResMut<CircuitBoardDisplay>,
    mut nodes: Query<&mut CircuitNode>,
    transition: Res<SettingsTransition>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let delta_seconds = time.delta_secs();
    let opacity_step = CIRCUIT_MAX_OPACITY / CIRCUIT_FADE_SECONDS * delta_seconds;
    display.opacity = move_toward(display.opacity, display.target_opacity, opacity_step);

    let window_size = Vec2::new(window.width(), window.height());
    if window_size.x <= 0.0 || window_size.y <= 0.0 {
        return;
    }

    let active_screen = display.active_screen.map(display_node_for_screen);
    if let Some(progress) = transition.circuit_progress() {
        let final_active_rect = active_rect(window_size);
        for mut node in &mut nodes {
            let current_rect = node.current_rect;
            let start = *node.transition_start_rect.get_or_insert(current_rect);
            let target = transformed_base_rect(
                node.screen,
                active_screen,
                Some(final_active_rect),
                window_size,
            );
            node.current_rect = lerp_rect(start, target, progress);
            node.corner_radius = rounded_rect_corner_radius(base_rect(node.screen, window_size));
        }
        return;
    }

    for mut node in &mut nodes {
        node.transition_start_rect = None;
    }

    let max_rect_delta =
        window_size.max_element() / ACTIVE_SCREEN_RECT_ANIMATION_SECONDS * delta_seconds;
    let active_rect = active_screen.and_then(|screen| {
        nodes.iter().find(|node| node.screen == screen).map(|node| {
            move_rect_toward(node.current_rect, active_rect(window_size), max_rect_delta)
        })
    });

    for mut node in &mut nodes {
        let base_rect = base_rect(node.screen, window_size);
        let target = transformed_base_rect(node.screen, active_screen, active_rect, window_size);
        node.current_rect = move_rect_toward(node.current_rect, target, max_rect_delta);
        node.corner_radius = rounded_rect_corner_radius(base_rect);
    }
}

fn lerp_rect(start: Rect, end: Rect, progress: f32) -> Rect {
    Rect::from_corners(
        start.min.lerp(end.min, progress),
        start.max.lerp(end.max, progress),
    )
}

pub(super) fn update_circuit_nodes(
    theme: Res<ActiveTheme>,
    display: Res<CircuitBoardDisplay>,
    nodes: Query<&CircuitNode>,
    mut strokes: Query<(&CircuitNodeStroke, &mut Sprite, &mut Transform)>,
) {
    let color = theme.primary.with_alpha(display.opacity);
    let rects: Vec<(AppState, Rect, f32)> = nodes
        .iter()
        .map(|node| (node.screen, node.current_rect, node.corner_radius))
        .collect();

    for (stroke, mut sprite, mut transform) in &mut strokes {
        let Some((_, rect, corner_radius)) =
            rects.iter().find(|(screen, _, _)| *screen == stroke.screen)
        else {
            continue;
        };
        let Some(segment) = rounded_rect_stroke_segment(*rect, stroke.part, *corner_radius) else {
            continue;
        };
        let (center, size, angle, _) = line_geometry(segment);
        sprite.color = color;
        sprite.custom_size = Some(size);
        transform.translation.x = center.x;
        transform.translation.y = center.y;
        transform.rotation = Quat::from_rotation_z(angle);
    }
}

pub(super) fn update_circuit_schematic(
    theme: Res<ActiveTheme>,
    display: Res<CircuitBoardDisplay>,
    windows: Query<&Window, With<PrimaryWindow>>,
    nodes: Query<&CircuitNode>,
    mut strokes: Query<(&CircuitSchematicStroke, &mut Sprite, &mut Transform)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let window_size = Vec2::new(window.width(), window.height());
    if window_size.x <= 0.0 || window_size.y <= 0.0 {
        return;
    }

    let rects: Vec<(AppState, Rect)> = nodes
        .iter()
        .map(|node| (node.screen, node.current_rect))
        .collect();
    let segments = schematic_segments(&rects, window_size);
    for (stroke, mut sprite, mut transform) in &mut strokes {
        let Some(segment) = segments.get(stroke.index).copied() else {
            sprite.color.set_alpha(0.0);
            continue;
        };
        let (center, size, angle, role) = line_geometry(segment);
        let color = match role {
            SegmentRole::Trace => theme.primary.with_alpha(display.opacity * 0.88),
            SegmentRole::Element => theme.secondary.with_alpha(display.opacity),
        };
        sprite.color = color;
        sprite.custom_size = Some(size);
        transform.translation.x = center.x;
        transform.translation.y = center.y;
        transform.translation.z = match role {
            SegmentRole::Trace => CIRCUIT_Z,
            SegmentRole::Element => CIRCUIT_ELEMENT_Z,
        };
        transform.rotation = Quat::from_rotation_z(angle);
    }
}
