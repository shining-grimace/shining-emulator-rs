use bevy::prelude::*;

use crate::background::constants::BACKGROUND_MAX_OPACITY;
use crate::background::utils::{random_blink_delay, random_blink_duration};

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
    elapsed_seconds: f32,
    duration_seconds: f32,
}

impl BlinkState {
    fn new() -> Self {
        Self {
            delay_seconds: random_blink_delay(),
            elapsed_seconds: 0.0,
            duration_seconds: 0.0,
        }
    }

    fn update(&mut self, delta_seconds: f32, background_visible: bool) {
        if !background_visible {
            return;
        }

        if self.duration_seconds > 0.0 {
            self.elapsed_seconds += delta_seconds;
            if self.elapsed_seconds >= self.duration_seconds {
                self.delay_seconds = random_blink_delay();
                self.elapsed_seconds = 0.0;
                self.duration_seconds = 0.0;
            }
        } else {
            self.delay_seconds -= delta_seconds;
            if self.delay_seconds <= 0.0 {
                self.duration_seconds = random_blink_duration();
                self.elapsed_seconds = 0.0;
            }
        }
    }

    fn opacity_multiplier(&self) -> f32 {
        if self.duration_seconds <= 0.0 {
            return 1.0;
        }
        let phase = (self.elapsed_seconds / self.duration_seconds).clamp(0.0, 1.0);
        (2.0 * phase - 1.0).abs()
    }
}
