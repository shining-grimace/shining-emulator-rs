use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

use crate::circuit_board::utils::active_rect;
use crate::dimensions::{
    GAME_BOY_FRAME_BUFFER_BYTES, GAME_BOY_RGB_CHANNELS, GAME_BOY_RGBA_CHANNELS,
    GAME_BOY_SCREEN_HEIGHT, GAME_BOY_SCREEN_HEIGHT_F32, GAME_BOY_SCREEN_HEIGHT_U32,
    GAME_BOY_SCREEN_WIDTH, GAME_BOY_SCREEN_WIDTH_F32, GAME_BOY_SCREEN_WIDTH_U32,
    GAME_BOY_TEXTURE_BYTES,
};
use crate::game_boy::frame_buffer::{GameBoyFrameRing, GameBoyFrameSequence};
use crate::game_boy::xbr_upscaler;
use crate::storage::LocalStorage;

const GAME_BOY_FRAME_Z: f32 = -40.0;
const GAME_BOY_TEXTURE_CLEAR_PIXEL: [u8; GAME_BOY_RGBA_CHANNELS] = [0, 0, 0, u8::MAX];

#[derive(Component)]
pub(super) struct GameBoyFrameDisplay;

#[derive(Debug, Default, Resource)]
pub(super) struct GameBoyFrameTexture {
    handle: Option<Handle<Image>>,
    scale: Option<usize>,
    uploaded_sequence: Option<GameBoyFrameSequence>,
    source_pixels: Vec<u32>,
    upscaled_pixels: Vec<u32>,
}

pub(super) fn spawn_game_boy_frame_display(
    mut commands: Commands,
    texture: ResMut<GameBoyFrameTexture>,
    mut images: ResMut<Assets<Image>>,
    storage: Res<LocalStorage>,
    windows: Query<&Window, With<PrimaryWindow>>,
    displays: Query<(), With<GameBoyFrameDisplay>>,
) {
    if !displays.is_empty() {
        return;
    }

    let scale = upscaling_scale(storage.data.settings.upscaling_mode);
    let texture = texture.into_inner();
    ensure_game_boy_screen_image(texture, &mut images, scale);
    let Some(handle) = texture.handle.clone() else {
        return;
    };

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
    storage: Res<LocalStorage>,
    texture: ResMut<GameBoyFrameTexture>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(frame) = frames.latest_written_frame() else {
        return;
    };

    let scale = upscaling_scale(storage.data.settings.upscaling_mode);
    let texture = texture.into_inner();
    ensure_game_boy_screen_image(texture, &mut images, scale);

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

    let uploaded = if scale == 1 {
        copy_rgb_frame_to_rgba_texture(frame.pixels(), data)
    } else {
        upload_upscaled_rgb_frame(frame.pixels(), data, scale, texture)
    };

    if !uploaded {
        warn!("Game Boy frame texture dimensions do not match the frame buffer");
        return;
    }

    texture.uploaded_sequence = Some(frame.sequence());
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

fn game_boy_screen_image(scale: usize) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: GAME_BOY_SCREEN_WIDTH_U32 * scale as u32,
            height: GAME_BOY_SCREEN_HEIGHT_U32 * scale as u32,
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

fn ensure_game_boy_screen_image(
    texture: &mut GameBoyFrameTexture,
    images: &mut Assets<Image>,
    scale: usize,
) {
    if texture.scale == Some(scale) {
        return;
    }

    let image = game_boy_screen_image(scale);
    if let Some(handle) = texture.handle.as_ref()
        && let Some(mut existing_image) = images.get_mut(handle)
    {
        *existing_image = image;
    } else {
        texture.handle = Some(images.add(image));
    }

    texture.scale = Some(scale);
    texture.uploaded_sequence = None;
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

fn upload_upscaled_rgb_frame(
    rgb: &[u8],
    rgba: &mut [u8],
    scale: usize,
    texture: &mut GameBoyFrameTexture,
) -> bool {
    let texture_bytes = GAME_BOY_TEXTURE_BYTES * scale * scale;
    if rgb.len() != GAME_BOY_FRAME_BUFFER_BYTES || rgba.len() != texture_bytes {
        return false;
    }

    pack_rgb_frame(rgb, &mut texture.source_pixels);
    let upscaled_len = GAME_BOY_SCREEN_WIDTH * GAME_BOY_SCREEN_HEIGHT * scale * scale;
    texture.upscaled_pixels.resize(upscaled_len, 0);
    if !xbr_upscaler::upscale(
        &texture.source_pixels,
        GAME_BOY_SCREEN_WIDTH,
        GAME_BOY_SCREEN_HEIGHT,
        scale,
        &mut texture.upscaled_pixels,
    ) {
        return false;
    }

    copy_xrgb_to_rgba_texture(&texture.upscaled_pixels, rgba)
}

fn pack_rgb_frame(rgb: &[u8], packed: &mut Vec<u32>) {
    packed.clear();
    packed.reserve_exact(GAME_BOY_SCREEN_WIDTH * GAME_BOY_SCREEN_HEIGHT);
    for source in rgb.chunks_exact(GAME_BOY_RGB_CHANNELS) {
        packed.push(((source[0] as u32) << 16) | ((source[1] as u32) << 8) | source[2] as u32);
    }
}

fn copy_xrgb_to_rgba_texture(xrgb: &[u32], rgba: &mut [u8]) -> bool {
    if rgba.len() != xrgb.len() * GAME_BOY_RGBA_CHANNELS {
        return false;
    }

    for (source, destination) in xrgb
        .iter()
        .zip(rgba.chunks_exact_mut(GAME_BOY_RGBA_CHANNELS))
    {
        destination[0] = ((source >> 16) & 0xff) as u8;
        destination[1] = ((source >> 8) & 0xff) as u8;
        destination[2] = (source & 0xff) as u8;
        destination[3] = u8::MAX;
    }

    true
}

fn upscaling_scale(setting: u8) -> usize {
    match setting {
        1 => 2,
        2 => 3,
        3 => 4,
        _ => 1,
    }
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
    fn xrgb_frame_is_uploaded_as_opaque_rgba_texture_data() {
        let xrgb = vec![0x0011_2233, 0x0044_5566];
        let mut rgba = vec![0; xrgb.len() * GAME_BOY_RGBA_CHANNELS];

        assert!(copy_xrgb_to_rgba_texture(&xrgb, &mut rgba));

        assert_eq!(&rgba[0..4], &[0x11, 0x22, 0x33, u8::MAX]);
        assert_eq!(&rgba[4..8], &[0x44, 0x55, 0x66, u8::MAX]);
    }

    #[test]
    fn upscaling_setting_maps_to_supported_scales() {
        assert_eq!(upscaling_scale(0), 1);
        assert_eq!(upscaling_scale(1), 2);
        assert_eq!(upscaling_scale(2), 3);
        assert_eq!(upscaling_scale(3), 4);
        assert_eq!(upscaling_scale(u8::MAX), 1);
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
