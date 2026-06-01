use bevy::prelude::*;

use crate::background::constants::{
    BACKGROUND_MAX_OPACITY, BLINK_MAX_OPACITY_MULTIPLIER, BLINK_MIN_OPACITY_MULTIPLIER,
};
use crate::background::utils::{random_blink_delay, random_blink_duration, random_range};

#[derive(Resource, Debug)]
pub struct BackgroundDisplay {
    pub(super) opacity: f32,
    pub(super) target_opacity: f32,
    blink: BlinkState,
}

impl BackgroundDisplay {
    pub fn fade_in(&mut self) {
        self.target_opacity = BACKGROUND_MAX_OPACITY;
    }

    pub fn fade_out(&mut self) {
        self.target_opacity = 0.0;
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn target_opacity(&self) -> f32 {
        self.target_opacity
    }

    pub(super) fn rendered_opacity(&self) -> f32 {
        self.opacity * self.blink.opacity_multiplier()
    }

    pub(super) fn update_blink(&mut self, delta_seconds: f32, background_visible: bool) {
        self.blink.update(delta_seconds, background_visible);
    }
}

impl Default for BackgroundDisplay {
    fn default() -> Self {
        Self {
            opacity: 0.0,
            target_opacity: 0.0,
            blink: BlinkState::new(),
        }
    }
}

#[derive(Debug)]
struct BlinkState {
    delay_seconds: f32,
    remaining_seconds: f32,
    opacity_multiplier: f32,
}

impl BlinkState {
    fn new() -> Self {
        Self {
            delay_seconds: random_blink_delay(),
            remaining_seconds: 0.0,
            opacity_multiplier: 1.0,
        }
    }

    fn update(&mut self, delta_seconds: f32, background_visible: bool) {
        if !background_visible {
            return;
        }

        if self.remaining_seconds > 0.0 {
            self.remaining_seconds -= delta_seconds;
            if self.remaining_seconds <= 0.0 {
                self.delay_seconds = random_blink_delay();
            }
        } else {
            self.delay_seconds -= delta_seconds;
            if self.delay_seconds <= 0.0 {
                self.remaining_seconds = random_blink_duration();
                self.opacity_multiplier =
                    random_range(BLINK_MIN_OPACITY_MULTIPLIER, BLINK_MAX_OPACITY_MULTIPLIER);
            }
        }
    }

    fn opacity_multiplier(&self) -> f32 {
        if self.remaining_seconds > 0.0 {
            self.opacity_multiplier
        } else {
            1.0
        }
    }
}
