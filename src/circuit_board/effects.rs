use bevy::prelude::*;

use crate::app_state::AppState;
use crate::circuit_board::constants::CIRCUIT_MAX_OPACITY;

#[derive(Resource, Debug)]
pub struct CircuitBoardDisplay {
    pub(super) opacity: f32,
    pub(super) target_opacity: f32,
    pub(super) active_screen: Option<AppState>,
}

impl CircuitBoardDisplay {
    pub fn fade_in_for(&mut self, screen: AppState) {
        self.active_screen = Some(screen);
        self.target_opacity = CIRCUIT_MAX_OPACITY;
    }

    pub fn fade_out(&mut self) {
        self.active_screen = None;
        self.target_opacity = 0.0;
    }
}

impl Default for CircuitBoardDisplay {
    fn default() -> Self {
        Self {
            opacity: 0.0,
            target_opacity: 0.0,
            active_screen: None,
        }
    }
}
