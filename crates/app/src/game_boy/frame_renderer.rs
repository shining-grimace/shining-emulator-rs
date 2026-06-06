use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

use crate::circuit_board::utils::active_rect;
use crate::dimensions::{
    GAME_BOY_FRAME_BUFFER_BYTES, GAME_BOY_RGB_CHANNELS, GAME_BOY_RGBA_CHANNELS,
    GAME_BOY_SCREEN_HEIGHT_F32, GAME_BOY_SCREEN_HEIGHT_U32, GAME_BOY_SCREEN_WIDTH_F32,
    GAME_BOY_SCREEN_WIDTH_U32, GAME_BOY_TEXTURE_BYTES,
};
use crate::game_boy::frame_buffer::{GameBoyFrameRing, GameBoyFrameSequence};

const GAME_BOY_FRAME_Z: f32 = -40.0;
const GAME_BOY_TEXTURE_CLEAR_PIXEL: [u8; GAME_BOY_RGBA_CHANNELS] = [0, 0, 0, u8::MAX];

#[derive(Component)]
pub(super) struct GameBoyFrameDisplay;

#[derive(Debug, Default, Resource)]
pub(super) struct GameBoyFrameTexture {
    handle: Option<Handle<Image>>,
    uploaded_sequence: Option<GameBoyFrameSequence>,
}

pub(super) fn spawn_game_boy_frame_display(
    mut commands: Commands,
    mut texture: ResMut<GameBoyFrameTexture>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    displays: Query<(), With<GameBoyFrameDisplay>>,
) {
    if !displays.is_empty() {
        return;
    }

    let handle = texture
        .handle
        .get_or_insert_with(|| images.add(game_boy_screen_image()))
        .clone();

    commands.spawn((
        GameBoyFrameDisplay,
        DespawnOnExit::<crate::app_state::AppState>(crate::app_state::AppState::Gameplay),
        Sprite {
            image: handle,
            custom_size: gameplay_frame_size(&windows),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, GAME_BOY_FRAME_Z),
    ));
}

pub(super) fn update_game_boy_frame_texture(
    frames: Res<GameBoyFrameRing>,
    texture: ResMut<GameBoyFrameTexture>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(frame) = frames.latest_written_frame() else {
        return;
    };

    if texture.uploaded_sequence == Some(frame.sequence()) {
        return;
    }

    let Some(handle) = texture.handle.as_ref() else {
        return;
    };

    let Some(mut image) = images.get_mut(handle) else {
        return;
    };

    let Some(data) = image.data.as_mut() else {
        warn!("Game Boy frame texture has no writable image data");
        return;
    };

    if !copy_rgb_frame_to_rgba_texture(frame.pixels(), data) {
        warn!("Game Boy frame texture dimensions do not match the frame buffer");
        return;
    }

    texture.into_inner().uploaded_sequence = Some(frame.sequence());
}

pub(super) fn resize_game_boy_frame_display(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut displays: Query<&mut Sprite, With<GameBoyFrameDisplay>>,
) {
    let size = gameplay_frame_size(&windows);
    for mut sprite in &mut displays {
        sprite.custom_size = size;
    }
}

fn game_boy_screen_image() -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: GAME_BOY_SCREEN_WIDTH_U32,
            height: GAME_BOY_SCREEN_HEIGHT_U32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &GAME_BOY_TEXTURE_CLEAR_PIXEL,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn gameplay_frame_size(windows: &Query<&Window, With<PrimaryWindow>>) -> Option<Vec2> {
    let Ok(window) = windows.single() else {
        return None;
    };

    aspect_fit_size(active_rect(Vec2::new(window.width(), window.height())).size())
}

fn aspect_fit_size(window_size: Vec2) -> Option<Vec2> {
    if window_size.x <= 0.0 || window_size.y <= 0.0 {
        return None;
    }

    let screen_size = Vec2::new(GAME_BOY_SCREEN_WIDTH_F32, GAME_BOY_SCREEN_HEIGHT_F32);
    let scale = (window_size.x / screen_size.x).min(window_size.y / screen_size.y);
    Some(screen_size * scale)
}

fn copy_rgb_frame_to_rgba_texture(rgb: &[u8], rgba: &mut [u8]) -> bool {
    if rgb.len() != GAME_BOY_FRAME_BUFFER_BYTES || rgba.len() != GAME_BOY_TEXTURE_BYTES {
        return false;
    }

    for (source, destination) in rgb
        .chunks_exact(GAME_BOY_RGB_CHANNELS)
        .zip(rgba.chunks_exact_mut(GAME_BOY_RGBA_CHANNELS))
    {
        destination[0] = source[0];
        destination[1] = source[1];
        destination[2] = source[2];
        destination[3] = u8::MAX;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_frame_is_uploaded_as_opaque_rgba_texture_data() {
        let mut rgb = vec![0; GAME_BOY_FRAME_BUFFER_BYTES];
        rgb[0] = 0x11;
        rgb[1] = 0x22;
        rgb[2] = 0x33;
        rgb[GAME_BOY_RGB_CHANNELS] = 0x44;
        rgb[GAME_BOY_RGB_CHANNELS + 1] = 0x55;
        rgb[GAME_BOY_RGB_CHANNELS + 2] = 0x66;
        let mut rgba = vec![0; GAME_BOY_TEXTURE_BYTES];

        assert!(copy_rgb_frame_to_rgba_texture(&rgb, &mut rgba));

        assert_eq!(&rgba[0..4], &[0x11, 0x22, 0x33, u8::MAX]);
        assert_eq!(&rgba[4..8], &[0x44, 0x55, 0x66, u8::MAX]);
    }

    #[test]
    fn mismatched_buffers_are_not_uploaded() {
        let mut rgba = vec![0; GAME_BOY_TEXTURE_BYTES];

        assert!(!copy_rgb_frame_to_rgba_texture(&[], &mut rgba));
    }

    #[test]
    fn frame_size_preserves_game_boy_aspect_ratio() {
        let size = aspect_fit_size(active_rect(Vec2::new(1280.0, 720.0)).size()).unwrap();

        assert!((size.x - 746.6667).abs() < 0.001);
        assert_eq!(size.y, 672.0);
    }
}
