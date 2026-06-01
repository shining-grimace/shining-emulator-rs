use bevy::prelude::*;

#[derive(Component)]
pub(super) struct BackgroundImageLayer;

#[derive(Component)]
pub(super) struct BackgroundParticleLayer;

#[derive(Component)]
pub(super) struct BackgroundParticle {
    pub direction: Vec2,
    pub max_speed: f32,
    pub drift_seconds: f32,
    pub drift_phase: f32,
    pub base_alpha: f32,
    pub pulse_offset: f32,
    pub pulse_speed: f32,
}

pub(super) struct RandomParticle {
    pub position: Vec2,
    pub behaviour: BackgroundParticle,
}
