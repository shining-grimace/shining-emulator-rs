use bevy::prelude::*;

use crate::app_state::AppState;
use crate::circuit_board::components::{RoundedRectCorner, RoundedRectStroke};
use crate::circuit_board::constants::{
    CIRCUIT_ELEMENT_SIZE, CIRCUIT_LAYOUT_HEIGHT, CIRCUIT_LAYOUT_WIDTH, CIRCUIT_MIN_WINDOW_MARGIN,
    CIRCUIT_NODE_HEIGHT, CIRCUIT_NODE_MIN_SCALE, CIRCUIT_NODE_THICKNESS, CIRCUIT_NODE_WIDTH,
    CIRCUIT_ROUNDING_SEGMENTS, CIRCUIT_TRACE_THICKNESS, CIRCUIT_WINDOW_MARGIN,
};

const MUX_SOURCE_BOTTOM_FRACTION: f32 = 0.75;

#[derive(Clone, Copy)]
pub(super) struct LineSegment {
    pub from: Vec2,
    pub to: Vec2,
    pub role: SegmentRole,
}

#[derive(Clone, Copy)]
pub(super) enum SegmentRole {
    Trace,
    Element,
}

pub(super) const CIRCUIT_SCREEN_NODES: [AppState; 7] = [
    AppState::InterfaceDemo,
    AppState::Settings,
    AppState::InputMapping,
    AppState::RomProvider,
    AppState::RomData,
    AppState::AudioSettings,
    AppState::Gameplay,
];

pub(super) fn screen_has_circuit_board(screen: AppState) -> bool {
    matches!(
        screen,
        AppState::InterfaceDemo
            | AppState::Home
            | AppState::Settings
            | AppState::InputMapping
            | AppState::RomProvider
            | AppState::RomData
            | AppState::AudioSettings
    )
}

pub(super) fn display_node_for_screen(screen: AppState) -> AppState {
    match screen {
        AppState::Home => AppState::InterfaceDemo,
        screen => screen,
    }
}

pub(super) fn move_toward(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else if current < target {
        current + max_delta
    } else {
        current - max_delta
    }
}

pub(super) fn move_rect_toward(current: Rect, target: Rect, max_delta: f32) -> Rect {
    Rect::new(
        move_toward(current.min.x, target.min.x, max_delta),
        move_toward(current.min.y, target.min.y, max_delta),
        move_toward(current.max.x, target.max.x, max_delta),
        move_toward(current.max.y, target.max.y, max_delta),
    )
}

pub(super) fn base_rect(screen: AppState, window_size: Vec2) -> Rect {
    let center = schematic_point(base_position(display_node_for_screen(screen)), window_size);
    let scale = node_scale(window_size);
    Rect::from_center_size(
        center,
        Vec2::new(CIRCUIT_NODE_WIDTH, CIRCUIT_NODE_HEIGHT) * scale,
    )
}

pub(super) fn active_rect(window_size: Vec2) -> Rect {
    let margin = CIRCUIT_WINDOW_MARGIN
        .min(window_size.x * 0.16)
        .min(window_size.y * 0.16)
        .max(CIRCUIT_MIN_WINDOW_MARGIN);
    let half_size = (window_size * 0.5 - Vec2::splat(margin)).max(Vec2::splat(12.0));
    Rect::from_center_half_size(Vec2::ZERO, half_size)
}

pub(super) fn transformed_base_rect(
    screen: AppState,
    active_screen: Option<AppState>,
    active_rect: Option<Rect>,
    window_size: Vec2,
) -> Rect {
    let screen = display_node_for_screen(screen);
    let base = base_rect(screen, window_size);
    let Some(active_screen) = active_screen.map(display_node_for_screen) else {
        return base;
    };
    let Some(active_rect) = active_rect else {
        return base;
    };

    if screen == active_screen {
        return active_rect;
    }

    let active_base = base_rect(active_screen, window_size);
    Rect::from_center_size(
        transform_schematic_point(base.center(), active_base, active_rect),
        base.size(),
    )
}

pub(super) fn rounded_rect_strokes() -> Vec<RoundedRectStroke> {
    let mut strokes = vec![
        RoundedRectStroke::Top,
        RoundedRectStroke::Right,
        RoundedRectStroke::Bottom,
        RoundedRectStroke::Left,
    ];

    for corner in [
        RoundedRectCorner::TopRight,
        RoundedRectCorner::TopLeft,
        RoundedRectCorner::BottomLeft,
        RoundedRectCorner::BottomRight,
    ] {
        for segment in 0..CIRCUIT_ROUNDING_SEGMENTS {
            strokes.push(RoundedRectStroke::Corner { corner, segment });
        }
    }

    strokes
}

pub(super) fn rounded_rect_corner_radius(rect: Rect) -> f32 {
    rect.width().min(rect.height()) * 0.14
}

pub(super) fn rounded_rect_stroke_segment(
    rect: Rect,
    part: RoundedRectStroke,
    corner_radius: f32,
) -> Option<LineSegment> {
    let radius = corner_radius
        .min(rect.width() * 0.5)
        .min(rect.height() * 0.5);
    let role = SegmentRole::Element;
    match part {
        RoundedRectStroke::Top => Some(LineSegment {
            from: Vec2::new(rect.min.x + radius, rect.max.y),
            to: Vec2::new(rect.max.x - radius, rect.max.y),
            role,
        }),
        RoundedRectStroke::Right => Some(LineSegment {
            from: Vec2::new(rect.max.x, rect.max.y - radius),
            to: Vec2::new(rect.max.x, rect.min.y + radius),
            role,
        }),
        RoundedRectStroke::Bottom => Some(LineSegment {
            from: Vec2::new(rect.max.x - radius, rect.min.y),
            to: Vec2::new(rect.min.x + radius, rect.min.y),
            role,
        }),
        RoundedRectStroke::Left => Some(LineSegment {
            from: Vec2::new(rect.min.x, rect.min.y + radius),
            to: Vec2::new(rect.min.x, rect.max.y - radius),
            role,
        }),
        RoundedRectStroke::Corner { corner, segment } => {
            let center = match corner {
                RoundedRectCorner::TopRight => Vec2::new(rect.max.x - radius, rect.max.y - radius),
                RoundedRectCorner::TopLeft => Vec2::new(rect.min.x + radius, rect.max.y - radius),
                RoundedRectCorner::BottomLeft => {
                    Vec2::new(rect.min.x + radius, rect.min.y + radius)
                }
                RoundedRectCorner::BottomRight => {
                    Vec2::new(rect.max.x - radius, rect.min.y + radius)
                }
            };
            let start_angle = match corner {
                RoundedRectCorner::TopRight => 0.0,
                RoundedRectCorner::TopLeft => std::f32::consts::FRAC_PI_2,
                RoundedRectCorner::BottomLeft => std::f32::consts::PI,
                RoundedRectCorner::BottomRight => std::f32::consts::PI * 1.5,
            };
            let angle_step = std::f32::consts::FRAC_PI_2 / CIRCUIT_ROUNDING_SEGMENTS as f32;
            let from_angle = start_angle + angle_step * segment as f32;
            let to_angle = from_angle + angle_step;
            Some(LineSegment {
                from: center + Vec2::new(from_angle.cos(), from_angle.sin()) * radius,
                to: center + Vec2::new(to_angle.cos(), to_angle.sin()) * radius,
                role,
            })
        }
    }
}

pub(super) fn base_node_rects(window_size: Vec2) -> Vec<(AppState, Rect)> {
    CIRCUIT_SCREEN_NODES
        .into_iter()
        .map(|screen| (screen, base_rect(screen, window_size)))
        .collect()
}

pub(super) fn schematic_segments(
    node_rects: &[(AppState, Rect)],
    window_size: Vec2,
) -> Vec<LineSegment> {
    let input_rect = node_rect(node_rects, AppState::InterfaceDemo, window_size);
    let settings_rect = node_rect(node_rects, AppState::Settings, window_size);
    let mapping_rect = node_rect(node_rects, AppState::InputMapping, window_size);
    let provider_rect = node_rect(node_rects, AppState::RomProvider, window_size);
    let data_rect = node_rect(node_rects, AppState::RomData, window_size);
    let audio_rect = node_rect(node_rects, AppState::AudioSettings, window_size);
    let gameplay_rect = node_rect(node_rects, AppState::Gameplay, window_size);
    let scale = node_scale(window_size);
    let element = CIRCUIT_ELEMENT_SIZE * scale;

    let mut segments = Vec::new();
    let trace = SegmentRole::Trace;

    let left_bus_x = input_rect.min.x - 92.0 * scale;
    for offset in [-18.0, 0.0, 18.0] {
        segments.push(line(
            Vec2::new(left_bus_x, input_rect.center().y + offset * scale),
            Vec2::new(input_rect.min.x, input_rect.center().y + offset * scale),
            trace,
        ));
    }

    let left_diode_center = Vec2::new(input_rect.max.x + 68.0 * scale, input_rect.center().y);
    segments.push(line(
        input_rect.right_center(),
        left_diode_center + Vec2::new(-element * 0.5, 0.0),
        trace,
    ));
    push_diode(&mut segments, left_diode_center, element, false);
    segments.push(line(
        left_diode_center + Vec2::new(element * 0.5, 0.0),
        settings_rect.left_center(),
        trace,
    ));

    let top_gate_center = Vec2::new(
        settings_rect.center().x + 102.0 * scale,
        mapping_rect.center().y,
    );
    let mid_gate_center = Vec2::new(settings_rect.max.x + 70.0 * scale, settings_rect.center().y);
    let mux_center = Vec2::new(settings_rect.max.x + 82.0 * scale, data_rect.center().y);
    let right_diode_center = Vec2::new(
        settings_rect.max.x + 252.0 * scale,
        settings_rect.center().y,
    );

    push_orthogonal(
        &mut segments,
        settings_rect.top_center(),
        Vec2::new(settings_rect.center().x, mapping_rect.center().y),
        top_gate_center + Vec2::new(-element * 0.64, 0.0),
    );
    push_and_gate(&mut segments, top_gate_center, element * 0.82);
    segments.push(line(
        top_gate_center + Vec2::new(element * 0.5, 0.0),
        mapping_rect.left_center(),
        trace,
    ));

    push_orthogonal(
        &mut segments,
        settings_rect.right_center(),
        mid_gate_center + Vec2::new(-element * 0.64, 0.0),
        mid_gate_center + Vec2::new(-element * 0.64, 0.0),
    );
    push_and_gate(&mut segments, mid_gate_center, element * 0.76);
    segments.push(line(
        mid_gate_center + Vec2::new(element * 0.5, 0.0),
        right_diode_center + Vec2::new(-element * 0.56, 0.0),
        trace,
    ));

    push_diode(&mut segments, right_diode_center, element * 0.98, false);
    segments.push(line(
        right_diode_center + Vec2::new(element * 0.5, 0.0),
        gameplay_rect.left_center(),
        trace,
    ));

    push_orthogonal(
        &mut segments,
        settings_rect.right_center(),
        Vec2::new(settings_rect.max.x + 40.0 * scale, provider_rect.center().y),
        provider_rect.left_center(),
    );
    push_orthogonal(
        &mut segments,
        settings_rect.bottom_center(),
        Vec2::new(settings_rect.center().x, audio_rect.center().y),
        audio_rect.left_center(),
    );

    let mux_source = settings_rect.bottom_at_fraction(MUX_SOURCE_BOTTOM_FRACTION);
    let mux_input = mux_center + Vec2::new(-element * 0.45, 0.0);
    let mux_corner = Vec2::new(mux_source.x, mux_input.y);
    segments.push(line(mux_source, mux_corner, trace));
    segments.push(line(mux_corner, mux_input, trace));
    push_multiplexer(&mut segments, mux_center, element * 0.58, element * 1.05);
    for offset in [-18.0, 0.0, 18.0] {
        segments.push(line(
            mux_center + Vec2::new(element * 0.30, offset * scale),
            data_rect.left_center() + Vec2::new(0.0, offset * scale),
            trace,
        ));
    }

    segments
}

pub(super) fn line_geometry(segment: LineSegment) -> (Vec2, Vec2, f32, SegmentRole) {
    let delta = segment.to - segment.from;
    let length = delta.length().max(1.0);
    let angle = delta.y.atan2(delta.x);
    let thickness = match segment.role {
        SegmentRole::Trace => CIRCUIT_TRACE_THICKNESS,
        SegmentRole::Element => CIRCUIT_NODE_THICKNESS,
    };
    let length = match segment.role {
        SegmentRole::Trace => length + thickness,
        SegmentRole::Element => length,
    };
    (
        (segment.from + segment.to) * 0.5,
        Vec2::new(length, thickness),
        angle,
        segment.role,
    )
}

fn line(from: Vec2, to: Vec2, role: SegmentRole) -> LineSegment {
    LineSegment { from, to, role }
}

fn push_orthogonal(segments: &mut Vec<LineSegment>, from: Vec2, corner: Vec2, to: Vec2) {
    segments.push(line(from, Vec2::new(corner.x, from.y), SegmentRole::Trace));
    segments.push(line(
        Vec2::new(corner.x, from.y),
        Vec2::new(corner.x, to.y),
        SegmentRole::Trace,
    ));
    segments.push(line(Vec2::new(corner.x, to.y), to, SegmentRole::Trace));
}

fn push_diode(segments: &mut Vec<LineSegment>, center: Vec2, size: f32, vertical: bool) {
    let half_width = size * 0.5;
    let half_height = size * 0.32;
    if vertical {
        return;
    }

    let left = center + Vec2::new(-half_width, -half_height);
    let point = center + Vec2::new(half_width * 0.46, 0.0);
    let top_left = center + Vec2::new(-half_width, half_height);
    let right_bar_top = center + Vec2::new(half_width * 0.58, half_height);
    let right_bar_bottom = center + Vec2::new(half_width * 0.58, -half_height);
    segments.push(line(left, point, SegmentRole::Element));
    segments.push(line(point, top_left, SegmentRole::Element));
    segments.push(line(top_left, left, SegmentRole::Element));
    segments.push(line(right_bar_bottom, right_bar_top, SegmentRole::Element));
}

fn push_and_gate(segments: &mut Vec<LineSegment>, center: Vec2, size: f32) {
    let half_height = size * 0.38;
    let left_x = center.x - size * 0.48;
    let mid_x = center.x - size * 0.04;
    let arc_center = Vec2::new(mid_x, center.y);
    let radius = half_height;

    segments.push(line(
        Vec2::new(left_x, center.y - half_height),
        Vec2::new(left_x, center.y + half_height),
        SegmentRole::Element,
    ));
    segments.push(line(
        Vec2::new(left_x, center.y + half_height),
        Vec2::new(mid_x, center.y + half_height),
        SegmentRole::Element,
    ));
    segments.push(line(
        Vec2::new(left_x, center.y - half_height),
        Vec2::new(mid_x, center.y - half_height),
        SegmentRole::Element,
    ));

    for segment in 0..CIRCUIT_ROUNDING_SEGMENTS {
        let angle_step = std::f32::consts::PI / CIRCUIT_ROUNDING_SEGMENTS as f32;
        let from_angle = -std::f32::consts::FRAC_PI_2 + angle_step * segment as f32;
        let to_angle = from_angle + angle_step;
        segments.push(line(
            arc_center + Vec2::new(from_angle.cos(), from_angle.sin()) * radius,
            arc_center + Vec2::new(to_angle.cos(), to_angle.sin()) * radius,
            SegmentRole::Element,
        ));
    }
}

fn push_multiplexer(segments: &mut Vec<LineSegment>, center: Vec2, width: f32, height: f32) {
    let rect = Rect::from_center_size(center, Vec2::new(width, height));
    for part in rounded_rect_strokes() {
        if let Some(segment) =
            rounded_rect_stroke_segment(rect, part, rounded_rect_corner_radius(rect))
        {
            segments.push(segment);
        }
    }
}

fn base_position(screen: AppState) -> Vec2 {
    match screen {
        AppState::InterfaceDemo | AppState::Home => Vec2::new(-410.0, 0.0),
        AppState::Settings => Vec2::new(-150.0, 0.0),
        AppState::InputMapping => Vec2::new(90.0, 176.0),
        AppState::RomProvider => Vec2::new(90.0, 72.0),
        AppState::RomData => Vec2::new(90.0, -72.0),
        AppState::AudioSettings => Vec2::new(90.0, -176.0),
        AppState::Gameplay => Vec2::new(410.0, 0.0),
        AppState::Loading | AppState::Splash => Vec2::ZERO,
    }
}

fn schematic_point(point: Vec2, window_size: Vec2) -> Vec2 {
    point * layout_scale(window_size)
}

fn layout_scale(window_size: Vec2) -> f32 {
    (window_size.x / CIRCUIT_LAYOUT_WIDTH)
        .min(window_size.y / CIRCUIT_LAYOUT_HEIGHT)
        .clamp(0.62, 1.18)
}

fn node_scale(window_size: Vec2) -> f32 {
    layout_scale(window_size).clamp(CIRCUIT_NODE_MIN_SCALE, 1.0)
}

fn node_rect(node_rects: &[(AppState, Rect)], screen: AppState, window_size: Vec2) -> Rect {
    node_rects
        .iter()
        .find(|(node_screen, _)| *node_screen == display_node_for_screen(screen))
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| base_rect(screen, window_size))
}

fn transform_schematic_point(point: Vec2, base_anchor: Rect, animated_anchor: Rect) -> Vec2 {
    Vec2::new(
        transform_axis(
            point.x,
            base_anchor.min.x,
            base_anchor.max.x,
            animated_anchor.min.x,
            animated_anchor.max.x,
        ),
        transform_axis(
            point.y,
            base_anchor.min.y,
            base_anchor.max.y,
            animated_anchor.min.y,
            animated_anchor.max.y,
        ),
    )
}

fn transform_axis(
    value: f32,
    base_min: f32,
    base_max: f32,
    target_min: f32,
    target_max: f32,
) -> f32 {
    if value < base_min {
        target_min - (base_min - value)
    } else if value > base_max {
        target_max + (value - base_max)
    } else {
        let base_width = (base_max - base_min).max(f32::EPSILON);
        target_min + (target_max - target_min) * ((value - base_min) / base_width)
    }
}

trait RectAnchors {
    fn left_center(&self) -> Vec2;
    fn right_center(&self) -> Vec2;
    fn top_center(&self) -> Vec2;
    fn bottom_center(&self) -> Vec2;
    fn bottom_at_fraction(&self, fraction: f32) -> Vec2;
}

impl RectAnchors for Rect {
    fn left_center(&self) -> Vec2 {
        Vec2::new(self.min.x, self.center().y)
    }

    fn right_center(&self) -> Vec2 {
        Vec2::new(self.max.x, self.center().y)
    }

    fn top_center(&self) -> Vec2 {
        Vec2::new(self.center().x, self.max.y)
    }

    fn bottom_center(&self) -> Vec2 {
        Vec2::new(self.center().x, self.min.y)
    }

    fn bottom_at_fraction(&self, fraction: f32) -> Vec2 {
        Vec2::new(
            self.min.x + self.width() * fraction.clamp(0.0, 1.0),
            self.min.y,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gameplay_is_rightmost_node() {
        let window_size = Vec2::new(1280.0, 720.0);
        let gameplay_center = base_rect(AppState::Gameplay, window_size).center().x;
        for screen in CIRCUIT_SCREEN_NODES {
            if screen != AppState::Gameplay {
                assert!(gameplay_center > base_rect(screen, window_size).center().x);
            }
        }
    }

    #[test]
    fn home_and_interface_demo_share_the_left_screen_node() {
        let window_size = Vec2::new(1280.0, 720.0);
        assert_eq!(
            base_rect(AppState::Home, window_size),
            base_rect(AppState::InterfaceDemo, window_size)
        );
    }

    #[test]
    fn rounded_rect_corner_radius_comes_from_base_node_size() {
        let window_size = Vec2::new(1280.0, 720.0);
        let base = base_rect(AppState::Settings, window_size);
        let active = active_rect(window_size);
        assert_ne!(
            rounded_rect_corner_radius(base),
            rounded_rect_corner_radius(active)
        );
    }

    #[test]
    fn active_rect_keeps_margin_from_window_edges() {
        let rect = active_rect(Vec2::new(1280.0, 720.0));
        assert_eq!(rect.min, Vec2::new(-616.0, -336.0));
        assert_eq!(rect.max, Vec2::new(616.0, 336.0));
    }
}
