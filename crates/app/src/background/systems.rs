use bevy::color::Alpha;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::app_assets::AppAssets;
use crate::app_theme::{ActiveTheme, ActiveThemeChanged};
use crate::background::components::{
    BackgroundImageLayer, BackgroundParticle, BackgroundParticleLayer,
};
use crate::background::constants::{
    BACKGROUND_FADE_SECONDS, BACKGROUND_MAX_OPACITY, BACKGROUND_Z, OFFSCREEN_MARGIN,
    PARTICLE_COUNT, PARTICLE_MAX_SIZE, PARTICLE_MIN_SIZE, PARTICLE_Z,
};
use crate::background::effects::BackgroundDisplay;
use crate::background::utils::{
    game_boy_aspect_fit_size, move_toward, quadratic_drift_speed_multiplier, random_direction,
    random_particle,
};

pub(super) fn fade_background_in(mut display: ResMut<BackgroundDisplay>) {
    display.fade_in();
}

pub(super) fn fade_background_out(mut display: ResMut<BackgroundDisplay>) {
    display.fade_out();
}

pub(super) fn spawn_background_entities(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    display: Res<BackgroundDisplay>,
    windows: Query<&Window, With<PrimaryWindow>>,
    background_query: Query<(), With<BackgroundImageLayer>>,
    particle_query: Query<(), With<BackgroundParticleLayer>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let window_size = Vec2::new(window.width(), window.height());

    if background_query.is_empty() {
        if let Some(background_image) = &assets.theme_background {
            commands.spawn((
                BackgroundImageLayer,
                Sprite {
                    image: background_image.clone(),
                    color: Color::WHITE.with_alpha(display.rendered_opacity()),
                    custom_size: Some(game_boy_aspect_fit_size(window_size)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, BACKGROUND_Z),
            ));
        }
    }

    if particle_query.is_empty() && theme.background_asset_path.is_some() {
        for _ in 0..PARTICLE_COUNT {
            let particle = random_particle(window_size);
            commands.spawn((
                BackgroundParticleLayer,
                Sprite::from_color(
                    theme.tertiary.with_alpha(0.0),
                    Vec2::splat(random_range(PARTICLE_MIN_SIZE, PARTICLE_MAX_SIZE)),
                ),
                Transform::from_xyz(particle.position.x, particle.position.y, PARTICLE_Z),
                particle.behaviour,
            ));
        }
    }
}

pub(super) fn update_background_theme(
    _theme_changed: On<ActiveThemeChanged>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    theme: Res<ActiveTheme>,
    mut assets: ResMut<AppAssets>,
    mut background_sprites: Query<(Entity, &mut Sprite), With<BackgroundImageLayer>>,
    particles: Query<Entity, With<BackgroundParticleLayer>>,
) {
    assets.theme_background = theme
        .background_asset_path
        .map(|path| asset_server.load(path));

    if let Some(background_image) = &assets.theme_background {
        for (_, mut sprite) in &mut background_sprites {
            sprite.image = background_image.clone();
        }
    } else {
        for (entity, _) in &mut background_sprites {
            commands.entity(entity).despawn();
        }
    }

    if theme.background_asset_path.is_none() {
        for entity in &particles {
            commands.entity(entity).despawn();
        }
    }
}

pub(super) fn update_background_opacity(
    time: Res<Time>,
    mut display: ResMut<BackgroundDisplay>,
    mut background_sprites: Query<&mut Sprite, With<BackgroundImageLayer>>,
) {
    let delta_seconds = time.delta_secs();
    let fade_speed = BACKGROUND_MAX_OPACITY / BACKGROUND_FADE_SECONDS;
    display.opacity = move_toward(
        display.opacity,
        display.target_opacity,
        fade_speed * delta_seconds,
    );
    let background_visible = display.opacity > 0.0;
    display.update_blink(delta_seconds, background_visible);

    let rendered_opacity = display.rendered_opacity();
    for mut sprite in &mut background_sprites {
        sprite.color.set_alpha(rendered_opacity);
    }
}

pub(super) fn configure_background_image_sampler(
    assets: Res<AppAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    if let Some(background_image) = &assets.theme_background {
        if let Some(mut image) = images.get_mut(background_image) {
            image.sampler = ImageSampler::nearest();
        }
    }
}

pub(super) fn resize_background_image(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut background_sprites: Query<&mut Sprite, With<BackgroundImageLayer>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let window_size = Vec2::new(window.width(), window.height());
    if window_size.x <= 0.0 || window_size.y <= 0.0 {
        return;
    }

    for mut sprite in &mut background_sprites {
        sprite.custom_size = Some(game_boy_aspect_fit_size(window_size));
    }
}

pub(super) fn animate_particles(
    time: Res<Time>,
    theme: Res<ActiveTheme>,
    display: Res<BackgroundDisplay>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut particles: Query<(&mut Transform, &mut Sprite, &mut BackgroundParticle)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let delta_seconds = time.delta_secs();
    let window_size = Vec2::new(window.width(), window.height());
    let half_width = window_size.x * 0.5 + OFFSCREEN_MARGIN;
    let half_height = window_size.y * 0.5 + OFFSCREEN_MARGIN;
    let visibility_scale = if BACKGROUND_MAX_OPACITY > 0.0 {
        display.rendered_opacity() / BACKGROUND_MAX_OPACITY
    } else {
        0.0
    };

    for (mut transform, mut sprite, mut particle) in &mut particles {
        particle.drift_phase += delta_seconds / particle.drift_seconds;
        if particle.drift_phase >= 1.0 {
            particle.drift_phase %= 1.0;
            particle.direction = random_direction();
        }

        let speed = particle.max_speed * quadratic_drift_speed_multiplier(particle.drift_phase);
        transform.translation.x += particle.direction.x * speed * delta_seconds;
        transform.translation.y += particle.direction.y * speed * delta_seconds;

        if transform.translation.x > half_width {
            transform.translation.x = -half_width;
        } else if transform.translation.x < -half_width {
            transform.translation.x = half_width;
        }

        if transform.translation.y > half_height {
            transform.translation.y = -half_height;
        } else if transform.translation.y < -half_height {
            transform.translation.y = half_height;
        }

        let pulse = ((time.elapsed_secs() * particle.pulse_speed + particle.pulse_offset).sin()
            + 1.0)
            * 0.5;
        let alpha = particle.base_alpha * (0.45 + pulse * 0.55) * visibility_scale;
        sprite.color = theme.tertiary.with_alpha(alpha);
    }
}

fn random_range(min: f32, max: f32) -> f32 {
    min + (max - min) * fastrand::f32()
}
