use bevy::prelude::*;

use crate::app_state::AppState;

#[derive(Component)]
pub(super) struct CircuitBoardLayer;

#[derive(Component)]
pub(super) struct CircuitNode {
    pub screen: AppState,
    pub current_rect: Rect,
    pub corner_radius: f32,
}

#[derive(Component)]
pub(super) struct CircuitNodeStroke {
    pub screen: AppState,
    pub part: RoundedRectStroke,
}

#[derive(Component)]
pub(super) struct CircuitSchematicStroke {
    pub index: usize,
}

#[derive(Clone, Copy)]
pub(super) enum RoundedRectStroke {
    Top,
    Right,
    Bottom,
    Left,
    Corner {
        corner: RoundedRectCorner,
        segment: usize,
    },
}

#[derive(Clone, Copy)]
pub(super) enum RoundedRectCorner {
    TopRight,
    TopLeft,
    BottomLeft,
    BottomRight,
}
