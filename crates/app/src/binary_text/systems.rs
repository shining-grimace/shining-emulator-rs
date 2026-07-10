use bevy::color::Alpha;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;
use std::collections::HashSet;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::binary_text::components::{BinaryTextDigit, BinaryTextLayer};
use crate::binary_text::constants::{
    BINARY_TEXT_CELL_HEIGHT, BINARY_TEXT_CELL_WIDTH, BINARY_TEXT_DIGIT_POOL_SIZE,
    BINARY_TEXT_FADE_SECONDS, BINARY_TEXT_FONT_SIZE, BINARY_TEXT_INSET, BINARY_TEXT_MAX_OPACITY,
    BINARY_TEXT_Z,
};
use crate::binary_text::effects::BinaryTextEffects;
use crate::circuit_board::utils::{active_rect, screen_has_circuit_board};
use crate::settings_transition::SettingsTransition;

pub(super) fn update_binary_text_grid(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut effects: ResMut<BinaryTextEffects>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let window_size = Vec2::new(window.width(), window.height());
    let (columns, rows) = grid_size(window_size);
    if effects.columns != columns || effects.rows != rows {
        effects.reset_grid(columns, rows);
    }
}

pub(super) fn spawn_binary_text_pool(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    assets: Res<AppAssets>,
    existing_digits: Query<(), With<BinaryTextLayer>>,
) {
    if !existing_digits.is_empty() || !asset_server.is_loaded(&assets.ubuntu_mono_font) {
        return;
    }

    for _ in 0..BINARY_TEXT_DIGIT_POOL_SIZE {
        commands.spawn((
            BinaryTextLayer,
            BinaryTextDigit {
                group_id: None,
                digit_index: 0,
            },
            Text2d::new(random_digit()),
            binary_text_font(&assets),
            TextColor(Color::NONE),
            TextLayout::justify(Justify::Center),
            Anchor::CENTER,
            Visibility::Hidden,
            Transform::from_xyz(0.0, 0.0, BINARY_TEXT_Z),
        ));
    }
}

pub(super) fn animate_binary_text(
    time: Res<Time>,
    state: Res<State<AppState>>,
    theme: Res<ActiveTheme>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut effects: ResMut<BinaryTextEffects>,
    transition: Res<SettingsTransition>,
    mut digits: Query<(
        &mut BinaryTextDigit,
        &mut TextColor,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let delta_seconds = time.delta_secs();
    let screen = *state.get();
    let foreground_opaque = transition.foreground_is_opaque();
    let active_screen = (screen_has_circuit_board(screen) && foreground_opaque).then_some(screen);
    effects.update(delta_seconds, active_screen);

    let rect = digit_rect(Vec2::new(window.width(), window.height()));
    let top_left = Vec2::new(rect.min.x, rect.max.y);
    let fade_multiplier = fade_multiplier(delta_seconds);
    let enabled = effects.is_settled() && foreground_opaque;
    if !enabled {
        for (mut digit, mut colour, _, mut visibility) in &mut digits {
            digit.group_id = None;
            colour.0 = Color::NONE;
            *visibility = Visibility::Hidden;
        }
        return;
    }

    let mut existing_digit_keys = HashSet::new();
    for (mut digit, mut colour, _, mut visibility) in &mut digits {
        let Some(group_id) = digit.group_id else {
            continue;
        };
        if effects.group(group_id).is_some() {
            existing_digit_keys.insert((group_id, digit.digit_index));
        } else {
            digit.group_id = None;
            colour.0 = Color::NONE;
            *visibility = Visibility::Hidden;
        }
    }

    let mut missing_digits = Vec::new();
    for group in &effects.groups {
        for digit_index in 0..group.digit_count {
            if existing_digit_keys.contains(&(group.id, digit_index)) {
                continue;
            }
            missing_digits.push((group.id, digit_index));
        }
    }

    for (mut digit, mut colour, _, mut visibility) in &mut digits {
        if digit.group_id.is_some() {
            continue;
        }

        let Some((group_id, digit_index)) = missing_digits.pop() else {
            break;
        };
        digit.group_id = Some(group_id);
        digit.digit_index = digit_index;
        colour.0 = Color::NONE;
        *visibility = Visibility::Visible;
    }

    for (digit, mut colour, mut transform, mut visibility) in &mut digits {
        let Some(group_id) = digit.group_id else {
            continue;
        };
        let Some(group) = effects.group(group_id) else {
            continue;
        };

        let column = group.start_column + digit.digit_index;
        transform.translation.x = top_left.x + column as f32 * BINARY_TEXT_CELL_WIDTH;
        transform.translation.y = top_left.y - group.row as f32 * BINARY_TEXT_CELL_HEIGHT;
        *visibility = Visibility::Visible;

        let target_alpha =
            group.digit_opacity_multiplier(digit.digit_index) * BINARY_TEXT_MAX_OPACITY;
        let alpha = move_toward(colour.0.alpha(), target_alpha, fade_multiplier);
        if alpha <= 0.0 {
            colour.0 = theme.tertiary.with_alpha(0.0);
        } else {
            colour.0 = theme.tertiary.with_alpha(alpha);
        }
    }
}

fn grid_size(window_size: Vec2) -> (usize, usize) {
    let rect = digit_rect(window_size);
    let columns = (rect.width() / BINARY_TEXT_CELL_WIDTH).floor().max(0.0) as usize;
    let rows = (rect.height() / BINARY_TEXT_CELL_HEIGHT).floor().max(0.0) as usize;
    (columns, rows)
}

fn digit_rect(window_size: Vec2) -> Rect {
    let rect = active_rect(window_size);
    Rect::new(
        rect.min.x + BINARY_TEXT_INSET,
        rect.min.y + BINARY_TEXT_INSET,
        rect.max.x - BINARY_TEXT_INSET,
        rect.max.y - BINARY_TEXT_INSET,
    )
}

fn fade_multiplier(delta_seconds: f32) -> f32 {
    if BINARY_TEXT_FADE_SECONDS <= 0.0 {
        return BINARY_TEXT_MAX_OPACITY;
    }

    BINARY_TEXT_MAX_OPACITY / BINARY_TEXT_FADE_SECONDS * delta_seconds
}

fn move_toward(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else if current < target {
        current + max_delta
    } else {
        current - max_delta
    }
}

fn random_digit() -> &'static str {
    if fastrand::bool() { "1" } else { "0" }
}

fn binary_text_font(assets: &AppAssets) -> TextFont {
    TextFont {
        font: assets.ubuntu_mono_font.clone().into(),
        font_size: BINARY_TEXT_FONT_SIZE.into(),
        ..default()
    }
}
