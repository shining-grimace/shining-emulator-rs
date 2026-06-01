use bevy::prelude::*;

use crate::background::components::{BackgroundParticle, RandomParticle};
use crate::background::constants::{
    BACKGROUND_GAMEBOY_HEIGHT, BACKGROUND_GAMEBOY_WIDTH, BACKGROUND_WINDOW_MARGIN,
    BLINK_MAX_DELAY_SECONDS, BLINK_MAX_DURATION_SECONDS, BLINK_MIN_DELAY_SECONDS,
    BLINK_MIN_DURATION_SECONDS, PARTICLE_BASE_ALPHA_MAX, PARTICLE_BASE_ALPHA_MIN,
    PARTICLE_MAX_DRIFT_SECONDS, PARTICLE_MAX_MAX_SPEED, PARTICLE_MIN_DRIFT_SECONDS,
    PARTICLE_MIN_MAX_SPEED, PARTICLE_PULSE_SPEED_MAX, PARTICLE_PULSE_SPEED_MIN,
};

pub(super) fn random_particle(window_size: Vec2) -> RandomParticle {
    RandomParticle {
        position: Vec2::new(
            random_range(-window_size.x * 0.5, window_size.x * 0.5),
            random_range(-window_size.y * 0.5, window_size.y * 0.5),
        ),
        behaviour: BackgroundParticle {
            direction: random_direction(),
            max_speed: random_range(PARTICLE_MIN_MAX_SPEED, PARTICLE_MAX_MAX_SPEED),
            drift_seconds: random_range(PARTICLE_MIN_DRIFT_SECONDS, PARTICLE_MAX_DRIFT_SECONDS),
            drift_phase: fastrand::f32(),
            base_alpha: random_range(PARTICLE_BASE_ALPHA_MIN, PARTICLE_BASE_ALPHA_MAX),
            pulse_offset: random_range(0.0, std::f32::consts::TAU),
            pulse_speed: random_range(PARTICLE_PULSE_SPEED_MIN, PARTICLE_PULSE_SPEED_MAX),
        },
    }
}

pub(super) fn game_boy_aspect_fit_size(window_size: Vec2) -> Vec2 {
    let available_size = Vec2::new(
        (window_size.x - BACKGROUND_WINDOW_MARGIN * 2.0).max(BACKGROUND_GAMEBOY_WIDTH),
        (window_size.y - BACKGROUND_WINDOW_MARGIN * 2.0).max(BACKGROUND_GAMEBOY_HEIGHT),
    );
    let game_boy_size = Vec2::new(BACKGROUND_GAMEBOY_WIDTH, BACKGROUND_GAMEBOY_HEIGHT);
    let scale = (available_size.x / game_boy_size.x).min(available_size.y / game_boy_size.y);
    game_boy_size * scale
}

pub(super) fn move_toward(current: f32, target: f32, max_delta: f32) -> f32 {
    let difference = target - current;
    if difference.abs() <= max_delta {
        target
    } else {
        current + max_delta.copysign(difference)
    }
}

pub(super) fn random_blink_delay() -> f32 {
    random_range(BLINK_MIN_DELAY_SECONDS, BLINK_MAX_DELAY_SECONDS)
}

pub(super) fn random_blink_duration() -> f32 {
    random_range(BLINK_MIN_DURATION_SECONDS, BLINK_MAX_DURATION_SECONDS)
}

pub(super) fn random_direction() -> Vec2 {
    let angle = random_range(0.0, std::f32::consts::TAU);
    Vec2::new(angle.cos(), angle.sin())
}

pub(super) fn quadratic_drift_speed_multiplier(phase: f32) -> f32 {
    let phase = phase.clamp(0.0, 1.0);
    4.0 * phase * (1.0 - phase)
}

pub(super) fn random_range(min: f32, max: f32) -> f32 {
    min + (max - min) * fastrand::f32()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_boy_aspect_fit_leaves_margin_and_preserves_ratio() {
        let size = game_boy_aspect_fit_size(Vec2::new(416.0, 336.0));
        assert!((size.x - 266.6667).abs() < 0.001);
        assert_eq!(size.y, 240.0);
    }

    #[test]
    fn move_toward_stops_at_target() {
        assert_eq!(move_toward(0.0, 1.0, 2.0), 1.0);
        assert_eq!(move_toward(1.0, 0.0, 2.0), 0.0);
    }

    #[test]
    fn quadratic_drift_speed_is_zero_at_ends_and_max_in_middle() {
        assert_eq!(quadratic_drift_speed_multiplier(0.0), 0.0);
        assert_eq!(quadratic_drift_speed_multiplier(0.5), 1.0);
        assert_eq!(quadratic_drift_speed_multiplier(1.0), 0.0);
    }
}
