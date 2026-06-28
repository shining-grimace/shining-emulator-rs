use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

use crate::circuit_board::utils::active_rect;
use crate::dimensions::{
    GAME_BOY_FRAME_BUFFER_BYTES, GAME_BOY_RGB_CHANNELS, GAME_BOY_RGBA_CHANNELS,
    GAME_BOY_SCREEN_HEIGHT, GAME_BOY_SCREEN_HEIGHT_U32, GAME_BOY_SCREEN_WIDTH,
    GAME_BOY_SCREEN_WIDTH_U32, GAME_BOY_TEXTURE_BYTES, GAMEPLAY_MENU_BUTTON_CLEARANCE,
    GAMEPLAY_TOUCH_OVERLAY_MARGIN, SGB_GAME_BOY_X_OFFSET, SGB_GAME_BOY_Y_OFFSET, SGB_SCREEN_HEIGHT,
    SGB_SCREEN_HEIGHT_U32, SGB_SCREEN_WIDTH, SGB_SCREEN_WIDTH_U32, SGB_TEXTURE_BYTES,
    TOUCH_OVERLAY_MARGIN, UI_ELEMENT_HEIGHT,
};
use crate::game_boy::emulator::SgbState;
use crate::game_boy::frame_buffer::{GameBoyFrameRing, GameBoyFrameSequence};
use crate::game_boy::xbr_upscaler;
use crate::game_boy::{GameBoyCore, GameBoyEmulator};
use crate::input::controller::ConnectedControllers;
use crate::input::selection::{PrimaryInputDevice, selected_mapping_has_available_device};
use crate::input::touch_overlay::{
    should_show_touch_controller_overlay, touch_controller_overlay_top_offset,
};
use crate::storage::LocalStorage;

const GAME_BOY_FRAME_Z: f32 = -40.0;
const GAME_BOY_TEXTURE_CLEAR_PIXEL: [u8; GAME_BOY_RGBA_CHANNELS] = [0, 0, 0, u8::MAX];

#[derive(Component)]
pub(super) struct GameBoyFrameDisplay;

#[derive(Clone, Copy, Debug, PartialEq)]
struct GameBoyFrameLayout {
    size: Vec2,
    translation: Vec2,
}

#[derive(Debug, Default, Resource)]
pub(super) struct GameBoyFrameTexture {
    handle: Option<Handle<Image>>,
    scale: Option<usize>,
    extent: Option<FrameExtent>,
    uploaded_sequence: Option<GameBoyFrameSequence>,
    uploaded_border_sequence: Option<u64>,
    source_pixels: Vec<u32>,
    upscaled_pixels: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrameExtent {
    GameBoy,
    Sgb,
}

impl FrameExtent {
    fn width(self) -> usize {
        match self {
            Self::GameBoy => GAME_BOY_SCREEN_WIDTH,
            Self::Sgb => SGB_SCREEN_WIDTH,
        }
    }

    fn height(self) -> usize {
        match self {
            Self::GameBoy => GAME_BOY_SCREEN_HEIGHT,
            Self::Sgb => SGB_SCREEN_HEIGHT,
        }
    }
}

pub(super) fn spawn_game_boy_frame_display(
    mut commands: Commands,
    texture: ResMut<GameBoyFrameTexture>,
    mut images: ResMut<Assets<Image>>,
    storage: Res<LocalStorage>,
    primary_input: Res<PrimaryInputDevice>,
    controllers: Res<ConnectedControllers>,
    windows: Query<&Window, With<PrimaryWindow>>,
    emulators: Query<&GameBoyCore, With<GameBoyEmulator>>,
    displays: Query<(), With<GameBoyFrameDisplay>>,
) {
    if !displays.is_empty() {
        return;
    }

    let scale = upscaling_scale(storage.data.settings.upscaling_mode);
    let extent = active_frame_extent(&storage, &emulators);
    let texture = texture.into_inner();
    ensure_game_boy_screen_image(texture, &mut images, scale, extent);
    let Some(handle) = texture.handle.clone() else {
        return;
    };
    let layout = gameplay_frame_layout(
        &windows,
        extent,
        touch_overlay_reserve_action_hints(&storage, &primary_input, &controllers),
        should_show_touch_controller_overlay(&storage),
    );

    commands.spawn((
        GameBoyFrameDisplay,
        DespawnOnExit::<crate::app_state::AppState>(crate::app_state::AppState::Gameplay),
        Sprite {
            image: handle,
            custom_size: layout.map(|layout| layout.size),
            ..default()
        },
        Transform::from_translation(
            layout
                .map(|layout| layout.translation.extend(GAME_BOY_FRAME_Z))
                .unwrap_or(Vec3::new(0.0, 0.0, GAME_BOY_FRAME_Z)),
        ),
    ));
}

pub(super) fn reset_game_boy_frame_output(
    mut frames: ResMut<GameBoyFrameRing>,
    texture: ResMut<GameBoyFrameTexture>,
    mut images: ResMut<Assets<Image>>,
) {
    frames.clear_to_black();
    clear_game_boy_screen_image(texture.into_inner(), &mut images);
}

pub(super) fn update_game_boy_frame_texture(
    frames: Res<GameBoyFrameRing>,
    storage: Res<LocalStorage>,
    texture: ResMut<GameBoyFrameTexture>,
    mut images: ResMut<Assets<Image>>,
    emulators: Query<&GameBoyCore, With<GameBoyEmulator>>,
) {
    let Some(frame) = frames.latest_written_frame() else {
        return;
    };

    let scale = upscaling_scale(storage.data.settings.upscaling_mode);
    let sgb_border = active_sgb_border(&storage, &emulators);
    let extent = if sgb_border.is_some() {
        FrameExtent::Sgb
    } else {
        FrameExtent::GameBoy
    };
    let border_sequence = sgb_border.map(|emulator| emulator.sgb.border_sequence);
    let texture = texture.into_inner();
    ensure_game_boy_screen_image(texture, &mut images, scale, extent);

    if texture.uploaded_sequence == Some(frame.sequence())
        && texture.uploaded_border_sequence == border_sequence
    {
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

    let uploaded = if let Some(emulator) = sgb_border {
        upload_sgb_frame(frame.pixels(), &emulator.sgb, data, scale, texture)
    } else if scale == 1 {
        copy_rgb_frame_to_rgba_texture(frame.pixels(), data)
    } else {
        upload_upscaled_rgb_frame(frame.pixels(), data, scale, texture)
    };

    if !uploaded {
        warn!("Game Boy frame texture dimensions do not match the frame buffer");
        return;
    }

    texture.uploaded_sequence = Some(frame.sequence());
    texture.uploaded_border_sequence = border_sequence;
}

pub(super) fn resize_game_boy_frame_display(
    windows: Query<&Window, With<PrimaryWindow>>,
    storage: Res<LocalStorage>,
    primary_input: Res<PrimaryInputDevice>,
    controllers: Res<ConnectedControllers>,
    emulators: Query<&GameBoyCore, With<GameBoyEmulator>>,
    mut displays: Query<(&mut Sprite, &mut Transform), With<GameBoyFrameDisplay>>,
) {
    let layout = gameplay_frame_layout(
        &windows,
        active_frame_extent(&storage, &emulators),
        touch_overlay_reserve_action_hints(&storage, &primary_input, &controllers),
        should_show_touch_controller_overlay(&storage),
    );
    for (mut sprite, mut transform) in &mut displays {
        sprite.custom_size = layout.map(|layout| layout.size);
        if let Some(layout) = layout {
            transform.translation.x = layout.translation.x;
            transform.translation.y = layout.translation.y;
        }
    }
}

fn game_boy_screen_image(scale: usize, extent: FrameExtent) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: extent_u32_width(extent) * scale as u32,
            height: extent_u32_height(extent) * scale as u32,
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
    extent: FrameExtent,
) {
    if texture.scale == Some(scale) && texture.extent == Some(extent) {
        return;
    }

    let image = game_boy_screen_image(scale, extent);
    if let Some(handle) = texture.handle.as_ref()
        && let Some(mut existing_image) = images.get_mut(handle)
    {
        *existing_image = image;
    } else {
        texture.handle = Some(images.add(image));
    }

    texture.scale = Some(scale);
    texture.extent = Some(extent);
    texture.uploaded_sequence = None;
    texture.uploaded_border_sequence = None;
}

fn clear_game_boy_screen_image(texture: &mut GameBoyFrameTexture, images: &mut Assets<Image>) {
    texture.uploaded_sequence = None;
    texture.uploaded_border_sequence = None;

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

    for pixel in data.chunks_exact_mut(GAME_BOY_RGBA_CHANNELS) {
        pixel.copy_from_slice(&GAME_BOY_TEXTURE_CLEAR_PIXEL);
    }
}

fn gameplay_frame_layout(
    windows: &Query<&Window, With<PrimaryWindow>>,
    extent: FrameExtent,
    reserve_action_hints: bool,
    touch_overlay_visible: bool,
) -> Option<GameBoyFrameLayout> {
    let Ok(window) = windows.single() else {
        return None;
    };

    Some(gameplay_frame_layout_for_window_size(
        Vec2::new(window.width(), window.height()),
        extent,
        reserve_action_hints,
        touch_overlay_visible,
    ))
}

fn gameplay_frame_layout_for_window_size(
    window_size: Vec2,
    extent: FrameExtent,
    reserve_action_hints: bool,
    touch_overlay_visible: bool,
) -> GameBoyFrameLayout {
    let rect =
        gameplay_frame_available_rect(window_size, reserve_action_hints, touch_overlay_visible);
    let size = aspect_fit_size(rect.size(), extent).unwrap_or(Vec2::ZERO);
    let translation = if touch_overlay_visible && window_size.x < window_size.y {
        Vec2::new(0.0, rect.min.y + size.y * 0.5)
    } else {
        rect.center()
    };

    GameBoyFrameLayout { size, translation }
}

fn gameplay_frame_available_rect(
    window_size: Vec2,
    reserve_action_hints: bool,
    touch_overlay_visible: bool,
) -> Rect {
    let rect = active_rect(window_size);
    if !touch_overlay_visible || window_size.x >= window_size.y {
        return rect;
    }

    let controls_top =
        -window_size.y * 0.5 + touch_controller_overlay_top_offset(reserve_action_hints);
    let top = gameplay_menu_clearance_y(window_size).min(rect.max.y);
    let bottom = (controls_top + GAMEPLAY_TOUCH_OVERLAY_MARGIN)
        .max(rect.min.y)
        .min(top);
    Rect::new(rect.min.x, bottom, rect.max.x, top)
}

fn gameplay_menu_clearance_y(window_size: Vec2) -> f32 {
    window_size.y * 0.5 - TOUCH_OVERLAY_MARGIN - UI_ELEMENT_HEIGHT - GAMEPLAY_MENU_BUTTON_CLEARANCE
}

fn aspect_fit_size(window_size: Vec2, extent: FrameExtent) -> Option<Vec2> {
    if window_size.x <= 0.0 || window_size.y <= 0.0 {
        return None;
    }

    let screen_size = Vec2::new(extent.width() as f32, extent.height() as f32);
    let scale = (window_size.x / screen_size.x).min(window_size.y / screen_size.y);
    Some(screen_size * scale)
}

fn active_frame_extent(
    storage: &LocalStorage,
    emulators: &Query<&GameBoyCore, With<GameBoyEmulator>>,
) -> FrameExtent {
    if active_sgb_border(storage, emulators).is_some() {
        FrameExtent::Sgb
    } else {
        FrameExtent::GameBoy
    }
}

fn active_sgb_border<'a>(
    storage: &LocalStorage,
    emulators: &'a Query<&GameBoyCore, With<GameBoyEmulator>>,
) -> Option<&'a GameBoyCore> {
    if storage.data.settings.sgb_overlay_enable == 0 {
        return None;
    }

    emulators
        .iter()
        .find(|emulator| emulator.rom.properties.sgb_flag)
}

fn touch_overlay_reserve_action_hints(
    storage: &LocalStorage,
    primary_input: &PrimaryInputDevice,
    controllers: &ConnectedControllers,
) -> bool {
    selected_mapping_has_available_device(primary_input, storage, controllers)
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

fn upload_sgb_frame(
    game_rgb: &[u8],
    sgb: &SgbState,
    rgba: &mut [u8],
    scale: usize,
    texture: &mut GameBoyFrameTexture,
) -> bool {
    if game_rgb.len() != GAME_BOY_FRAME_BUFFER_BYTES {
        return false;
    }

    render_sgb_frame_to_xrgb(game_rgb, sgb, &mut texture.source_pixels);
    if scale == 1 {
        return rgba.len() == SGB_TEXTURE_BYTES
            && copy_xrgb_to_rgba_texture(&texture.source_pixels, rgba);
    }

    let texture_bytes = SGB_TEXTURE_BYTES * scale * scale;
    if rgba.len() != texture_bytes {
        return false;
    }

    let upscaled_len = SGB_SCREEN_WIDTH * SGB_SCREEN_HEIGHT * scale * scale;
    texture.upscaled_pixels.resize(upscaled_len, 0);
    if !xbr_upscaler::upscale(
        &texture.source_pixels,
        SGB_SCREEN_WIDTH,
        SGB_SCREEN_HEIGHT,
        scale,
        &mut texture.upscaled_pixels,
    ) {
        return false;
    }

    copy_xrgb_to_rgba_texture(&texture.upscaled_pixels, rgba)
}

fn render_sgb_frame_to_xrgb(game_rgb: &[u8], sgb: &SgbState, pixels: &mut Vec<u32>) {
    pixels.clear();
    pixels.resize(SGB_SCREEN_WIDTH * SGB_SCREEN_HEIGHT, 0);
    copy_game_boy_frame_into_sgb_frame(game_rgb, pixels);
    draw_sgb_border_over_frame(sgb, pixels);
}

fn copy_game_boy_frame_into_sgb_frame(game_rgb: &[u8], pixels: &mut [u32]) {
    for y in 0..GAME_BOY_SCREEN_HEIGHT {
        for x in 0..GAME_BOY_SCREEN_WIDTH {
            let source = (y * GAME_BOY_SCREEN_WIDTH + x) * GAME_BOY_RGB_CHANNELS;
            let destination =
                (y + SGB_GAME_BOY_Y_OFFSET) * SGB_SCREEN_WIDTH + x + SGB_GAME_BOY_X_OFFSET;
            let Some(pixel) = pixels.get_mut(destination) else {
                continue;
            };
            *pixel = (u32::from(game_rgb[source]) << 16)
                | (u32::from(game_rgb[source + 1]) << 8)
                | u32::from(game_rgb[source + 2]);
        }
    }
}

fn draw_sgb_border_over_frame(sgb: &SgbState, pixels: &mut [u32]) {
    for y in 0..SGB_SCREEN_HEIGHT {
        for x in 0..SGB_SCREEN_WIDTH {
            let map_index = (y / 8) * 32 + (x / 8);
            let map_entry = sgb.border_tile_map.get(map_index).copied().unwrap_or(0);
            let colour_index = sgb_border_colour_index(sgb, map_entry, x % 8, y % 8);
            let inside_game_boy_screen =
                (SGB_GAME_BOY_X_OFFSET..SGB_GAME_BOY_X_OFFSET + GAME_BOY_SCREEN_WIDTH).contains(&x)
                    && (SGB_GAME_BOY_Y_OFFSET..SGB_GAME_BOY_Y_OFFSET + GAME_BOY_SCREEN_HEIGHT)
                        .contains(&y);
            if inside_game_boy_screen && colour_index == 0 {
                continue;
            }

            let palette = sgb_border_palette(map_entry);
            let palette_index = palette * 16 + usize::from(colour_index);
            let colour = sgb.border_palettes.get(palette_index).copied().unwrap_or(0) & 0x00ff_ffff;
            let Some(pixel) = pixels.get_mut(y * SGB_SCREEN_WIDTH + x) else {
                continue;
            };
            *pixel = colour;
        }
    }
}

fn sgb_border_colour_index(sgb: &SgbState, map_entry: u16, tile_x: usize, tile_y: usize) -> u8 {
    let x = if map_entry & 0x4000 != 0 {
        7 - tile_x
    } else {
        tile_x
    };
    let y = if map_entry & 0x8000 != 0 {
        7 - tile_y
    } else {
        tile_y
    };
    let tile = usize::from(map_entry & 0x00ff);
    let tile_start = tile * 32;
    let row = y * 2;
    let plane_01 = tile_start + row;
    let plane_23 = tile_start + 16 + row;
    let shift = 7 - x;
    let lo0 = sgb.border_tiles.get(plane_01).copied().unwrap_or(0);
    let lo1 = sgb.border_tiles.get(plane_01 + 1).copied().unwrap_or(0);
    let hi0 = sgb.border_tiles.get(plane_23).copied().unwrap_or(0);
    let hi1 = sgb.border_tiles.get(plane_23 + 1).copied().unwrap_or(0);

    ((lo0 >> shift) & 0x01)
        | (((lo1 >> shift) & 0x01) << 1)
        | (((hi0 >> shift) & 0x01) << 2)
        | (((hi1 >> shift) & 0x01) << 3)
}

fn sgb_border_palette(map_entry: u16) -> usize {
    usize::from(((map_entry >> 10) & 0x07).saturating_sub(4)).min(2)
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

fn extent_u32_width(extent: FrameExtent) -> u32 {
    match extent {
        FrameExtent::GameBoy => GAME_BOY_SCREEN_WIDTH_U32,
        FrameExtent::Sgb => SGB_SCREEN_WIDTH_U32,
    }
}

fn extent_u32_height(extent: FrameExtent) -> u32 {
    match extent {
        FrameExtent::GameBoy => GAME_BOY_SCREEN_HEIGHT_U32,
        FrameExtent::Sgb => SGB_SCREEN_HEIGHT_U32,
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
    fn clear_game_boy_screen_image_resets_texture_to_black() {
        let mut images = Assets::<Image>::default();
        let handle = images.add(game_boy_screen_image(1, FrameExtent::GameBoy));
        {
            let mut image = images.get_mut(&handle).unwrap();
            image.data.as_mut().unwrap().fill(0x80);
        }
        let mut texture = GameBoyFrameTexture {
            handle: Some(handle.clone()),
            scale: Some(1),
            extent: Some(FrameExtent::GameBoy),
            uploaded_sequence: Some(GameBoyFrameSequence::default()),
            uploaded_border_sequence: Some(3),
            source_pixels: Vec::new(),
            upscaled_pixels: Vec::new(),
        };

        clear_game_boy_screen_image(&mut texture, &mut images);

        let image = images.get(&handle).unwrap();
        assert_eq!(texture.uploaded_sequence, None);
        assert_eq!(texture.uploaded_border_sequence, None);
        assert!(
            image
                .data
                .as_ref()
                .unwrap()
                .chunks_exact(GAME_BOY_RGBA_CHANNELS)
                .all(|pixel| pixel == GAME_BOY_TEXTURE_CLEAR_PIXEL)
        );
    }

    #[test]
    fn frame_size_preserves_game_boy_aspect_ratio() {
        let size = aspect_fit_size(
            active_rect(Vec2::new(1280.0, 720.0)).size(),
            FrameExtent::GameBoy,
        )
        .unwrap();

        assert!((size.x - 746.6667).abs() < 0.001);
        assert_eq!(size.y, 672.0);
    }

    #[test]
    fn frame_size_preserves_sgb_aspect_ratio_when_border_is_active() {
        let size = aspect_fit_size(
            active_rect(Vec2::new(1280.0, 720.0)).size(),
            FrameExtent::Sgb,
        )
        .unwrap();

        assert!((size.x - 768.0).abs() < 0.001);
        assert_eq!(size.y, 672.0);
    }

    #[test]
    fn portrait_frame_sits_above_touch_overlay_controls() {
        let window_size = Vec2::new(500.0, 1000.0);
        let layout =
            gameplay_frame_layout_for_window_size(window_size, FrameExtent::GameBoy, true, true);
        let controls_top = -window_size.y * 0.5 + touch_controller_overlay_top_offset(true);
        let frame_bottom = layout.translation.y - layout.size.y * 0.5;
        let frame_top = layout.translation.y + layout.size.y * 0.5;

        assert!((frame_bottom - (controls_top + GAMEPLAY_TOUCH_OVERLAY_MARGIN)).abs() < 0.001);
        assert!(frame_top <= gameplay_menu_clearance_y(window_size) + 0.001);
    }

    #[test]
    fn tall_portrait_frame_shrinks_to_fit_below_gameplay_menu() {
        let window_size = Vec2::new(700.0, 800.0);
        let layout =
            gameplay_frame_layout_for_window_size(window_size, FrameExtent::GameBoy, true, true);
        let frame_top = layout.translation.y + layout.size.y * 0.5;

        assert!(frame_top <= gameplay_menu_clearance_y(window_size) + 0.001);
    }

    #[test]
    fn landscape_frame_stays_centred_with_touch_overlay_visible() {
        let layout = gameplay_frame_layout_for_window_size(
            Vec2::new(1280.0, 720.0),
            FrameExtent::GameBoy,
            true,
            true,
        );

        assert_eq!(layout.translation, Vec2::ZERO);
        assert!((layout.size.x - 746.6667).abs() < 0.001);
        assert_eq!(layout.size.y, 672.0);
    }

    #[test]
    fn portrait_without_touch_overlay_uses_centred_active_rect() {
        let layout = gameplay_frame_layout_for_window_size(
            Vec2::new(700.0, 800.0),
            FrameExtent::GameBoy,
            true,
            false,
        );

        assert_eq!(layout.translation, Vec2::ZERO);
    }

    #[test]
    fn cramped_portrait_frame_does_not_extend_past_active_top() {
        let window_size = Vec2::new(320.0, 360.0);
        let layout =
            gameplay_frame_layout_for_window_size(window_size, FrameExtent::GameBoy, true, true);
        let frame_top = layout.translation.y + layout.size.y * 0.5;

        assert!(frame_top <= active_rect(window_size).max.y);
        assert!(frame_top <= gameplay_menu_clearance_y(window_size));
    }

    #[test]
    fn sgb_frame_centres_game_boy_pixels_without_border_data() {
        let mut game_rgb = vec![0; GAME_BOY_FRAME_BUFFER_BYTES];
        game_rgb[0] = 0x12;
        game_rgb[1] = 0x34;
        game_rgb[2] = 0x56;
        let sgb = SgbState::default();
        let mut pixels = Vec::new();

        render_sgb_frame_to_xrgb(&game_rgb, &sgb, &mut pixels);

        let game_origin = SGB_GAME_BOY_Y_OFFSET * SGB_SCREEN_WIDTH + SGB_GAME_BOY_X_OFFSET;
        assert_eq!(pixels.len(), SGB_SCREEN_WIDTH * SGB_SCREEN_HEIGHT);
        assert_eq!(pixels[game_origin], 0x0012_3456);
        assert_eq!(pixels[0], 0);
    }

    #[test]
    fn sgb_border_non_zero_pixels_can_cover_game_boy_window() {
        let mut game_rgb = vec![0; GAME_BOY_FRAME_BUFFER_BYTES];
        game_rgb[0] = 0x12;
        game_rgb[1] = 0x34;
        game_rgb[2] = 0x56;
        let mut sgb = SgbState::default();
        let map_index = (SGB_GAME_BOY_Y_OFFSET / 8) * 32 + (SGB_GAME_BOY_X_OFFSET / 8);
        sgb.border_tile_map[map_index] = 4 << 10;
        sgb.border_palettes[1] = 0xffaa_5500;
        sgb.border_tiles[0] = 0x80;
        let mut pixels = Vec::new();

        render_sgb_frame_to_xrgb(&game_rgb, &sgb, &mut pixels);

        let game_origin = SGB_GAME_BOY_Y_OFFSET * SGB_SCREEN_WIDTH + SGB_GAME_BOY_X_OFFSET;
        assert_eq!(pixels[game_origin], 0x00aa_5500);
    }
}
