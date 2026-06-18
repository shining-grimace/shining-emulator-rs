use bevy::prelude::warn;

use crate::dimensions::{
    GAME_BOY_FRAME_BUFFER_BYTES, GAME_BOY_RGB_CHANNELS, GAME_BOY_SCREEN_HEIGHT,
    GAME_BOY_SCREEN_WIDTH,
};
use crate::game_boy::emulator::memory::GameBoyMemory;
use crate::game_boy::emulator::palettes::PaletteState;
use crate::game_boy::emulator::sgb::SgbState;
use crate::game_boy::frame_buffer::GameBoyFrameRing;

const BLACK_RGB: [u8; GAME_BOY_RGB_CHANNELS] = [0, 0, 0];

const LCDC_BG_ENABLE: u8 = 0x01;
const LCDC_OBJ_ENABLE: u8 = 0x02;
const LCDC_OBJ_SIZE: u8 = 0x04;
const LCDC_BG_TILE_MAP: u8 = 0x08;
const LCDC_TILE_DATA: u8 = 0x10;
const LCDC_WINDOW_ENABLE: u8 = 0x20;
const LCDC_WINDOW_TILE_MAP: u8 = 0x40;
const LCDC_DISPLAY_ENABLE: u8 = 0x80;

const OBJECT_COUNT: usize = 40;
const MAX_OBJECTS_PER_LINE: usize = 10;
const OBJECT_WIDTH: usize = 8;
const OBJECT_X_OFFSET: usize = 8;
const OBJECT_Y_OFFSET: usize = 16;

#[derive(Debug)]
pub(crate) struct VideoFrameAssembler {
    pixels: Box<[u8]>,
    frame_in_progress: bool,
}

impl Default for VideoFrameAssembler {
    fn default() -> Self {
        Self {
            pixels: vec![0; GAME_BOY_FRAME_BUFFER_BYTES].into_boxed_slice(),
            frame_in_progress: true,
        }
    }
}

impl VideoFrameAssembler {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.begin_frame();
    }

    pub(crate) fn begin_frame(&mut self) {
        self.frame_in_progress = true;
    }

    pub(crate) fn write_gb_line(
        &mut self,
        line: u8,
        memory: &GameBoyMemory,
        palettes: &PaletteState,
    ) {
        if !self.frame_in_progress {
            return;
        }
        let line_no = usize::from(line);
        if line_no >= GAME_BOY_SCREEN_HEIGHT {
            return;
        }

        let lcd_control = io(memory, 0x40);
        if lcd_control & LCDC_DISPLAY_ENABLE == 0 {
            self.fill_line(line_no, BLACK_RGB);
            return;
        }

        let mut bg_colour_numbers = [0_u32; GAME_BOY_SCREEN_WIDTH];
        let bg_window_enabled = lcd_control & LCDC_BG_ENABLE != 0;
        if bg_window_enabled {
            self.draw_gb_background_line(
                line_no,
                lcd_control,
                memory,
                palettes,
                &mut bg_colour_numbers,
            );
        } else {
            self.fill_line(line_no, BLACK_RGB);
        }

        if bg_window_enabled && lcd_control & LCDC_WINDOW_ENABLE != 0 {
            self.draw_gb_window_line(
                line_no,
                lcd_control,
                memory,
                palettes,
                &mut bg_colour_numbers,
            );
        }

        if lcd_control & LCDC_OBJ_ENABLE != 0 {
            self.draw_gb_sprite_line(line_no, lcd_control, memory, palettes, &bg_colour_numbers);
        }
    }

    pub(crate) fn write_cgb_line(
        &mut self,
        line: u8,
        memory: &GameBoyMemory,
        palettes: &PaletteState,
    ) {
        if !self.frame_in_progress {
            return;
        }
        let line_no = usize::from(line);
        if line_no >= GAME_BOY_SCREEN_HEIGHT {
            return;
        }

        let lcd_control = io(memory, 0x40);
        if lcd_control & LCDC_DISPLAY_ENABLE == 0 {
            self.fill_line(line_no, BLACK_RGB);
            return;
        }

        let mut bg_colour_numbers = [0_u32; GAME_BOY_SCREEN_WIDTH];
        let mut bg_display_priorities = [false; GAME_BOY_SCREEN_WIDTH];
        self.draw_cgb_background_line(
            line_no,
            lcd_control,
            memory,
            palettes,
            &mut bg_colour_numbers,
            &mut bg_display_priorities,
        );

        if lcd_control & LCDC_WINDOW_ENABLE != 0 {
            self.draw_cgb_window_line(
                line_no,
                lcd_control,
                memory,
                palettes,
                &mut bg_colour_numbers,
                &mut bg_display_priorities,
            );
        }

        if lcd_control & LCDC_OBJ_ENABLE != 0 {
            self.draw_cgb_sprite_line(
                line_no,
                lcd_control,
                memory,
                palettes,
                &bg_colour_numbers,
                &bg_display_priorities,
            );
        }
    }

    pub(crate) fn write_sgb_line(
        &mut self,
        line: u8,
        memory: &GameBoyMemory,
        palettes: &PaletteState,
        sgb: &mut SgbState,
    ) {
        let line_no = usize::from(line);
        if line_no >= GAME_BOY_SCREEN_HEIGHT {
            return;
        }

        let lcd_control = io(memory, 0x40);
        if lcd_control & LCDC_DISPLAY_ENABLE == 0 {
            fill_sgb_mono_line(sgb, line_no, 0);
            return;
        }

        let bg_window_enabled = lcd_control & LCDC_BG_ENABLE != 0;
        if bg_window_enabled {
            draw_sgb_background_line(line_no, lcd_control, memory, palettes, sgb);
        } else {
            fill_sgb_mono_line(sgb, line_no, 0);
        }

        if bg_window_enabled && lcd_control & LCDC_WINDOW_ENABLE != 0 {
            draw_sgb_window_line(line_no, lcd_control, memory, palettes, sgb);
        }

        if lcd_control & LCDC_OBJ_ENABLE != 0 {
            draw_sgb_sprite_line(line_no, lcd_control, memory, palettes, sgb);
        }
    }

    pub(crate) fn colourise_sgb_frame(&mut self, sgb: &SgbState) {
        for tile_y in 0..18 {
            for tile_x in 0..20 {
                let palette_number = sgb
                    .character_palettes
                    .get(tile_y * 20 + tile_x)
                    .copied()
                    .unwrap_or(0) as usize;
                let palette_offset = (palette_number & 0x03) * 4;
                for y in tile_y * 8..tile_y * 8 + 8 {
                    for x in tile_x * 8..tile_x * 8 + 8 {
                        let colour_index = sgb
                            .mono_data
                            .get(y * GAME_BOY_SCREEN_WIDTH + x)
                            .copied()
                            .unwrap_or(0) as usize;
                        let colour = sgb
                            .palettes
                            .get(palette_offset + (colour_index & 0x03))
                            .copied()
                            .unwrap_or(0);
                        self.write_pixel(y, x, colour);
                    }
                }
            }
        }
    }

    pub(crate) fn clear_black(&mut self) {
        for pixel in self.pixels.chunks_exact_mut(GAME_BOY_RGB_CHANNELS) {
            pixel.copy_from_slice(&BLACK_RGB);
        }
    }

    pub(crate) fn publish_frame(&mut self, frames: &mut GameBoyFrameRing) {
        if !self.frame_in_progress {
            return;
        }

        let Some(mut frame) = frames.borrow_next_write_frame() else {
            warn!("Game Boy frame ring is unavailable");
            return;
        };

        let destination = frame.pixels_mut();
        if destination.len() != self.pixels.len() {
            warn!("Game Boy frame ring buffer does not match assembled video frame size");
            return;
        }

        destination.copy_from_slice(&self.pixels);
        frame.publish();
        self.frame_in_progress = false;
    }

    fn draw_gb_background_line(
        &mut self,
        line_no: usize,
        lcd_control: u8,
        memory: &GameBoyMemory,
        palettes: &PaletteState,
        bg_colour_numbers: &mut [u32; GAME_BOY_SCREEN_WIDTH],
    ) {
        let scroll_y = usize::from(io(memory, 0x42));
        let scroll_x = usize::from(io(memory, 0x43));
        let tile_map_base = if lcd_control & LCDC_BG_TILE_MAP != 0 {
            0x1c00
        } else {
            0x1800
        };
        let source_y = (line_no + scroll_y) & 0xff;

        for screen_x in 0..GAME_BOY_SCREEN_WIDTH {
            let source_x = (screen_x + scroll_x) & 0xff;
            let colour_index =
                gb_bg_colour_index(memory, lcd_control, tile_map_base, source_x, source_y);
            bg_colour_numbers[screen_x] = colour_index;
            self.write_pixel(
                line_no,
                screen_x,
                palettes.translated_bg[colour_index as usize],
            );
        }
    }

    fn draw_gb_window_line(
        &mut self,
        line_no: usize,
        lcd_control: u8,
        memory: &GameBoyMemory,
        palettes: &PaletteState,
        bg_colour_numbers: &mut [u32; GAME_BOY_SCREEN_WIDTH],
    ) {
        let window_y = usize::from(io(memory, 0x4a));
        let window_x = usize::from(io(memory, 0x4b));
        if window_x >= 167 || window_y > line_no {
            return;
        }

        let screen_start_x = window_x.saturating_sub(7).min(GAME_BOY_SCREEN_WIDTH);
        let tile_map_base = if lcd_control & LCDC_WINDOW_TILE_MAP != 0 {
            0x1c00
        } else {
            0x1800
        };
        let source_y = line_no - window_y;

        for screen_x in screen_start_x..GAME_BOY_SCREEN_WIDTH {
            let source_x = screen_x - screen_start_x;
            let colour_index =
                gb_bg_colour_index(memory, lcd_control, tile_map_base, source_x, source_y);
            bg_colour_numbers[screen_x] = colour_index;
            self.write_pixel(
                line_no,
                screen_x,
                palettes.translated_bg[colour_index as usize],
            );
        }
    }

    fn draw_gb_sprite_line(
        &mut self,
        line_no: usize,
        lcd_control: u8,
        memory: &GameBoyMemory,
        palettes: &PaletteState,
        bg_colour_numbers: &[u32; GAME_BOY_SCREEN_WIDTH],
    ) {
        let objects = select_objects_for_line(memory, lcd_control, line_no);
        let priority_objects = dmg_priority_objects(&objects);

        for screen_x in 0..GAME_BOY_SCREEN_WIDTH {
            let Some(pixel) = priority_objects.iter().find_map(|&object| {
                object_pixel(memory, lcd_control, object, line_no, screen_x, false)
            }) else {
                continue;
            };
            if pixel.flags & 0x80 != 0 && bg_colour_numbers[screen_x] != 0 {
                continue;
            }

            let palette_offset = if pixel.flags & 0x10 != 0 { 4 } else { 0 };
            self.write_pixel(
                line_no,
                screen_x,
                palettes.translated_obj[palette_offset + pixel.colour_index as usize],
            );
        }
    }

    fn draw_cgb_background_line(
        &mut self,
        line_no: usize,
        lcd_control: u8,
        memory: &GameBoyMemory,
        palettes: &PaletteState,
        bg_colour_numbers: &mut [u32; GAME_BOY_SCREEN_WIDTH],
        bg_display_priorities: &mut [bool; GAME_BOY_SCREEN_WIDTH],
    ) {
        let scroll_y = usize::from(io(memory, 0x42));
        let scroll_x = usize::from(io(memory, 0x43));
        let tile_map_base = if lcd_control & LCDC_BG_TILE_MAP != 0 {
            0x1c00
        } else {
            0x1800
        };
        let source_y = (line_no + scroll_y) & 0xff;

        for screen_x in 0..GAME_BOY_SCREEN_WIDTH {
            let source_x = (screen_x + scroll_x) & 0xff;
            let pixel = cgb_bg_pixel(memory, lcd_control, tile_map_base, source_x, source_y);
            bg_colour_numbers[screen_x] = pixel.colour_index;
            bg_display_priorities[screen_x] = pixel.bg_priority;
            self.write_pixel(
                line_no,
                screen_x,
                palettes.cgb_bg[pixel.palette_offset + pixel.colour_index as usize],
            );
        }
    }

    fn draw_cgb_window_line(
        &mut self,
        line_no: usize,
        lcd_control: u8,
        memory: &GameBoyMemory,
        palettes: &PaletteState,
        bg_colour_numbers: &mut [u32; GAME_BOY_SCREEN_WIDTH],
        bg_display_priorities: &mut [bool; GAME_BOY_SCREEN_WIDTH],
    ) {
        let window_y = usize::from(io(memory, 0x4a));
        let window_x = usize::from(io(memory, 0x4b));
        if window_x >= 167 || window_y > line_no {
            return;
        }

        let screen_start_x = window_x.saturating_sub(7).min(GAME_BOY_SCREEN_WIDTH);
        let tile_map_base = if lcd_control & LCDC_WINDOW_TILE_MAP != 0 {
            0x1c00
        } else {
            0x1800
        };
        let source_y = line_no - window_y;

        for screen_x in screen_start_x..GAME_BOY_SCREEN_WIDTH {
            let source_x = screen_x - screen_start_x;
            let pixel = cgb_bg_pixel(memory, lcd_control, tile_map_base, source_x, source_y);
            bg_colour_numbers[screen_x] = pixel.colour_index;
            bg_display_priorities[screen_x] = pixel.bg_priority;
            self.write_pixel(
                line_no,
                screen_x,
                palettes.cgb_bg[pixel.palette_offset + pixel.colour_index as usize],
            );
        }
    }

    fn draw_cgb_sprite_line(
        &mut self,
        line_no: usize,
        lcd_control: u8,
        memory: &GameBoyMemory,
        palettes: &PaletteState,
        bg_colour_numbers: &[u32; GAME_BOY_SCREEN_WIDTH],
        bg_display_priorities: &[bool; GAME_BOY_SCREEN_WIDTH],
    ) {
        let objects = select_objects_for_line(memory, lcd_control, line_no);
        let priority_objects = cgb_priority_objects(&objects);

        for screen_x in 0..GAME_BOY_SCREEN_WIDTH {
            let Some(pixel) = priority_objects.iter().find_map(|&object| {
                object_pixel(memory, lcd_control, object, line_no, screen_x, true)
            }) else {
                continue;
            };
            if cgb_bg_has_priority(
                lcd_control,
                bg_colour_numbers[screen_x],
                bg_display_priorities[screen_x],
                pixel.flags,
            ) {
                continue;
            }

            let palette_offset = usize::from(pixel.flags & 0x07) * 4;
            self.write_pixel(
                line_no,
                screen_x,
                palettes.cgb_obj[palette_offset + pixel.colour_index as usize],
            );
        }
    }

    fn fill_line(&mut self, line_no: usize, rgb: [u8; GAME_BOY_RGB_CHANNELS]) {
        let line_start = line_no * GAME_BOY_SCREEN_WIDTH * GAME_BOY_RGB_CHANNELS;
        let line_end = line_start + GAME_BOY_SCREEN_WIDTH * GAME_BOY_RGB_CHANNELS;
        let Some(row) = self.pixels.get_mut(line_start..line_end) else {
            return;
        };
        for pixel in row.chunks_exact_mut(GAME_BOY_RGB_CHANNELS) {
            pixel.copy_from_slice(&rgb);
        }
    }

    fn write_pixel(&mut self, line_no: usize, x: usize, colour: u32) {
        let offset = (line_no * GAME_BOY_SCREEN_WIDTH + x) * GAME_BOY_RGB_CHANNELS;
        let Some(pixel) = self.pixels.get_mut(offset..offset + GAME_BOY_RGB_CHANNELS) else {
            return;
        };
        pixel[0] = ((colour >> 16) & 0xff) as u8;
        pixel[1] = ((colour >> 8) & 0xff) as u8;
        pixel[2] = (colour & 0xff) as u8;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SelectedObject {
    pub(crate) index: usize,
    pub(crate) screen_x: i32,
    pub(crate) oam_x: u8,
    oam_y: u8,
    tile_no: u8,
    flags: u8,
}

impl SelectedObject {
    fn is_horizontally_visible(self) -> bool {
        self.oam_x != 0 && self.oam_x < 168
    }
}

pub(crate) fn select_objects_for_line(
    memory: &GameBoyMemory,
    lcd_control: u8,
    line_no: usize,
) -> Vec<SelectedObject> {
    let sprite_height = sprite_height(lcd_control);
    let line = i32::try_from(line_no).unwrap_or_default();
    let mut objects = Vec::with_capacity(MAX_OBJECTS_PER_LINE);

    for index in 0..OBJECT_COUNT {
        let offset = index * 4;
        let oam_y = memory.oam[offset];
        let sprite_top = i32::from(oam_y) - OBJECT_Y_OFFSET as i32;
        if line < sprite_top || line >= sprite_top + sprite_height as i32 {
            continue;
        }

        let oam_x = memory.oam[offset + 1];
        objects.push(SelectedObject {
            index,
            screen_x: i32::from(oam_x) - OBJECT_X_OFFSET as i32,
            oam_x,
            oam_y,
            tile_no: memory.oam[offset + 2],
            flags: memory.oam[offset + 3],
        });
        if objects.len() >= MAX_OBJECTS_PER_LINE {
            break;
        }
    }

    objects
}

fn sprite_height(lcd_control: u8) -> usize {
    if lcd_control & LCDC_OBJ_SIZE != 0 {
        16
    } else {
        8
    }
}

fn dmg_priority_objects(objects: &[SelectedObject]) -> Vec<SelectedObject> {
    let mut priority_objects: Vec<_> = objects
        .iter()
        .copied()
        .filter(|object| object.is_horizontally_visible())
        .collect();
    priority_objects.sort_by_key(|object| (object.oam_x, object.index));
    priority_objects
}

fn cgb_priority_objects(objects: &[SelectedObject]) -> Vec<SelectedObject> {
    objects
        .iter()
        .copied()
        .filter(|object| object.is_horizontally_visible())
        .collect()
}

fn cgb_bg_has_priority(
    lcd_control: u8,
    bg_colour_number: u32,
    bg_priority: bool,
    object_flags: u8,
) -> bool {
    bg_colour_number != 0
        && lcd_control & LCDC_BG_ENABLE != 0
        && (bg_priority || object_flags & 0x80 != 0)
}

fn object_pixel(
    memory: &GameBoyMemory,
    lcd_control: u8,
    object: SelectedObject,
    line_no: usize,
    screen_x: usize,
    cgb_mode: bool,
) -> Option<ObjectPixel> {
    let source_x = i32::try_from(screen_x)
        .ok()?
        .saturating_sub(object.screen_x);
    if !(0..OBJECT_WIDTH as i32).contains(&source_x) {
        return None;
    }

    let mut tile_no = usize::from(object.tile_no);
    let line_in_object = line_no + OBJECT_Y_OFFSET - usize::from(object.oam_y);
    if lcd_control & LCDC_OBJ_SIZE != 0 {
        tile_no &= 0xfe;
        if line_in_object >= 8 {
            tile_no |= 0x01;
        }
    }

    let mut pixel_y = line_in_object % 8;
    if object.flags & 0x40 != 0 {
        pixel_y = 7 - pixel_y;
        if lcd_control & LCDC_OBJ_SIZE != 0 {
            tile_no ^= 0x01;
        }
    }

    if cgb_mode && object.flags & 0x08 != 0 {
        tile_no += 384;
    }

    let source_x = usize::try_from(source_x).ok()?;
    let pixel_x = if object.flags & 0x20 != 0 {
        7 - source_x
    } else {
        source_x
    };
    let colour_index = tile_pixel(memory, tile_no, pixel_x, pixel_y);
    if colour_index == 0 {
        return None;
    }

    Some(ObjectPixel {
        colour_index,
        flags: object.flags,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ObjectPixel {
    colour_index: u32,
    flags: u8,
}

fn gb_bg_colour_index(
    memory: &GameBoyMemory,
    lcd_control: u8,
    tile_map_base: usize,
    source_x: usize,
    source_y: usize,
) -> u32 {
    let tile_set_index_offset = if lcd_control & LCDC_TILE_DATA != 0 {
        0
    } else {
        0x80
    };
    let tile_set_index_inverter = if lcd_control & LCDC_TILE_DATA != 0 {
        0
    } else {
        0x80
    };
    let tile_x = (source_x / 8) & 31;
    let tile_y = (source_y / 8) & 31;
    let tile_map_index = tile_map_base + 32 * tile_y + tile_x;
    let tile_number = memory
        .vram
        .get(tile_map_index)
        .map(|tile| usize::from(*tile ^ tile_set_index_inverter) + tile_set_index_offset)
        .unwrap_or(0);
    tile_pixel(memory, tile_number, source_x % 8, source_y % 8)
}

#[derive(Clone, Copy, Debug)]
struct CgbBackgroundPixel {
    colour_index: u32,
    palette_offset: usize,
    bg_priority: bool,
}

fn cgb_bg_pixel(
    memory: &GameBoyMemory,
    lcd_control: u8,
    tile_map_base: usize,
    source_x: usize,
    source_y: usize,
) -> CgbBackgroundPixel {
    let tile_set_index_offset = if lcd_control & LCDC_TILE_DATA != 0 {
        0
    } else {
        0x80
    };
    let tile_set_index_inverter = if lcd_control & LCDC_TILE_DATA != 0 {
        0
    } else {
        0x80
    };
    let tile_x = (source_x / 8) & 31;
    let tile_y = (source_y / 8) & 31;
    let tile_map_index = tile_map_base + 32 * tile_y + tile_x;
    let tile_attributes = memory
        .vram
        .get(0x2000 + tile_map_index)
        .copied()
        .unwrap_or(0);
    let mut tile_number = memory
        .vram
        .get(tile_map_index)
        .map(|tile| usize::from(*tile ^ tile_set_index_inverter) + tile_set_index_offset)
        .unwrap_or(0);
    if tile_attributes & 0x08 != 0 {
        tile_number += 384;
    }

    let pixel_x = if tile_attributes & 0x20 != 0 {
        7 - (source_x % 8)
    } else {
        source_x % 8
    };
    let pixel_y = if tile_attributes & 0x40 != 0 {
        7 - (source_y % 8)
    } else {
        source_y % 8
    };

    CgbBackgroundPixel {
        colour_index: tile_pixel(memory, tile_number, pixel_x, pixel_y),
        palette_offset: usize::from(tile_attributes & 0x07) * 4,
        bg_priority: tile_attributes & 0x80 != 0,
    }
}

fn draw_sgb_background_line(
    line_no: usize,
    lcd_control: u8,
    memory: &GameBoyMemory,
    palettes: &PaletteState,
    sgb: &mut SgbState,
) {
    let scroll_y = usize::from(io(memory, 0x42));
    let scroll_x = usize::from(io(memory, 0x43));
    let tile_map_base = if lcd_control & LCDC_BG_TILE_MAP != 0 {
        0x1c00
    } else {
        0x1800
    };
    let source_y = (line_no + scroll_y) & 0xff;

    for screen_x in 0..GAME_BOY_SCREEN_WIDTH {
        let source_x = (screen_x + scroll_x) & 0xff;
        let colour_index =
            gb_bg_colour_index(memory, lcd_control, tile_map_base, source_x, source_y);
        write_sgb_mono_pixel(
            sgb,
            line_no,
            screen_x,
            palettes.sgb_translation_bg[colour_index as usize],
        );
    }
}

fn draw_sgb_window_line(
    line_no: usize,
    lcd_control: u8,
    memory: &GameBoyMemory,
    palettes: &PaletteState,
    sgb: &mut SgbState,
) {
    let window_y = usize::from(io(memory, 0x4a));
    let window_x = usize::from(io(memory, 0x4b));
    if window_x >= 167 || window_y > line_no {
        return;
    }

    let screen_start_x = window_x.saturating_sub(7).min(GAME_BOY_SCREEN_WIDTH);
    let tile_map_base = if lcd_control & LCDC_WINDOW_TILE_MAP != 0 {
        0x1c00
    } else {
        0x1800
    };
    let source_y = line_no - window_y;

    for screen_x in screen_start_x..GAME_BOY_SCREEN_WIDTH {
        let source_x = screen_x - screen_start_x;
        let colour_index =
            gb_bg_colour_index(memory, lcd_control, tile_map_base, source_x, source_y);
        write_sgb_mono_pixel(
            sgb,
            line_no,
            screen_x,
            palettes.sgb_translation_bg[colour_index as usize],
        );
    }
}

fn draw_sgb_sprite_line(
    line_no: usize,
    lcd_control: u8,
    memory: &GameBoyMemory,
    palettes: &PaletteState,
    sgb: &mut SgbState,
) {
    let objects = select_objects_for_line(memory, lcd_control, line_no);
    let priority_objects = dmg_priority_objects(&objects);

    for screen_x in 0..GAME_BOY_SCREEN_WIDTH {
        let Some(pixel) = priority_objects.iter().find_map(|&object| {
            object_pixel(memory, lcd_control, object, line_no, screen_x, false)
        }) else {
            continue;
        };
        let mono_index = sgb
            .mono_data
            .get(line_no * GAME_BOY_SCREEN_WIDTH + screen_x)
            .copied()
            .unwrap_or(0);
        if pixel.flags & 0x80 != 0 && mono_index != palettes.sgb_translation_bg[0] {
            continue;
        }
        let palette_offset = if pixel.flags & 0x10 != 0 { 4 } else { 0 };
        write_sgb_mono_pixel(
            sgb,
            line_no,
            screen_x,
            palettes.sgb_translation_obj[palette_offset + pixel.colour_index as usize],
        );
    }
}

fn fill_sgb_mono_line(sgb: &mut SgbState, line_no: usize, value: u32) {
    let start = line_no * GAME_BOY_SCREEN_WIDTH;
    let end = start + GAME_BOY_SCREEN_WIDTH;
    if let Some(row) = sgb.mono_data.get_mut(start..end) {
        row.fill(value);
    }
}

fn write_sgb_mono_pixel(sgb: &mut SgbState, line_no: usize, x: usize, value: u32) {
    if let Some(pixel) = sgb.mono_data.get_mut(line_no * GAME_BOY_SCREEN_WIDTH + x) {
        *pixel = value & 0x03;
    }
}

fn tile_pixel(memory: &GameBoyMemory, tile_number: usize, pixel_x: usize, pixel_y: usize) -> u32 {
    let index = tile_number
        .saturating_mul(64)
        .saturating_add(pixel_y.saturating_mul(8))
        .saturating_add(pixel_x);
    memory.tile_set.get(index).copied().unwrap_or(0)
}

fn io(memory: &GameBoyMemory, index: usize) -> u8 {
    memory.io_ports.get(index).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: u32 = 0xffaa_0000;
    const GREEN: u32 = 0xff00_aa00;
    const BLUE: u32 = 0xff00_00aa;

    fn set_object(memory: &mut GameBoyMemory, index: usize, y: u8, x: u8, tile: u8, flags: u8) {
        let offset = index * 4;
        memory.oam[offset] = y;
        memory.oam[offset + 1] = x;
        memory.oam[offset + 2] = tile;
        memory.oam[offset + 3] = flags;
    }

    fn fill_tile_row(memory: &mut GameBoyMemory, tile: usize, colour_index: u32) {
        let start = tile * 64;
        for pixel in 0..8 {
            memory.tile_set[start + pixel] = colour_index;
        }
    }

    fn rendered_pixel(assembler: &VideoFrameAssembler, x: usize) -> [u8; 3] {
        let start = x * GAME_BOY_RGB_CHANNELS;
        [
            assembler.pixels[start],
            assembler.pixels[start + 1],
            assembler.pixels[start + 2],
        ]
    }

    #[test]
    fn gb_line_renderer_reads_decoded_background_tiles() {
        let mut assembler = VideoFrameAssembler::default();
        let mut memory = GameBoyMemory::default();
        let mut palettes = PaletteState::default();
        memory.io_ports[0x40] = LCDC_DISPLAY_ENABLE | LCDC_BG_ENABLE | LCDC_TILE_DATA;
        memory.io_ports[0x47] = 0xe4;
        memory.vram[0x1800] = 1;
        memory.tile_set[64] = 3;
        palettes.translate_bg(0xe4);

        assembler.write_gb_line(0, &memory, &palettes);

        assert_eq!(&assembler.pixels[0..3], &[0x00, 0x00, 0x00]);
    }

    #[test]
    fn object_selection_keeps_first_ten_y_overlapping_oam_entries() {
        let mut memory = GameBoyMemory::default();
        let lcd_control = LCDC_DISPLAY_ENABLE | LCDC_OBJ_ENABLE;
        for index in 0..11 {
            set_object(&mut memory, index, 16, 8 + index as u8, index as u8, 0);
        }
        memory.oam[1] = 0;

        let objects = select_objects_for_line(&memory, lcd_control, 0);

        assert_eq!(objects.len(), 10);
        assert_eq!(objects[0].index, 0);
        assert_eq!(objects[0].oam_x, 0);
        assert_eq!(objects[9].index, 9);
    }

    #[test]
    fn objects_after_first_ten_are_not_rendered() {
        let mut assembler = VideoFrameAssembler::default();
        let mut memory = GameBoyMemory::default();
        let mut palettes = PaletteState::default();
        memory.io_ports[0x40] = LCDC_DISPLAY_ENABLE | LCDC_OBJ_ENABLE;
        palettes.translated_obj[1] = RED;
        for index in 0..10 {
            set_object(&mut memory, index, 16, 0, 0, 0);
        }
        set_object(&mut memory, 10, 16, 8, 1, 0);
        fill_tile_row(&mut memory, 1, 1);

        assembler.write_gb_line(0, &memory, &palettes);

        assert_ne!(rendered_pixel(&assembler, 0), [0xaa, 0x00, 0x00]);
    }

    #[test]
    fn dmg_objects_prioritize_lower_x_before_oam_order() {
        let mut assembler = VideoFrameAssembler::default();
        let mut memory = GameBoyMemory::default();
        let mut palettes = PaletteState::default();
        memory.io_ports[0x40] = LCDC_DISPLAY_ENABLE | LCDC_OBJ_ENABLE;
        palettes.translated_obj[1] = RED;
        palettes.translated_obj[5] = BLUE;
        set_object(&mut memory, 0, 16, 12, 1, 0);
        set_object(&mut memory, 1, 16, 8, 2, 0x10);
        fill_tile_row(&mut memory, 1, 1);
        fill_tile_row(&mut memory, 2, 1);

        assembler.write_gb_line(0, &memory, &palettes);

        assert_eq!(rendered_pixel(&assembler, 4), [0x00, 0x00, 0xaa]);
    }

    #[test]
    fn cgb_objects_prioritize_oam_order_before_x() {
        let mut assembler = VideoFrameAssembler::default();
        let mut memory = GameBoyMemory::default();
        let mut palettes = PaletteState::default();
        memory.io_ports[0x40] = LCDC_DISPLAY_ENABLE | LCDC_OBJ_ENABLE;
        palettes.cgb_obj[1] = RED;
        palettes.cgb_obj[5] = BLUE;
        set_object(&mut memory, 0, 16, 12, 1, 0);
        set_object(&mut memory, 1, 16, 8, 2, 0x01);
        fill_tile_row(&mut memory, 1, 1);
        fill_tile_row(&mut memory, 2, 1);

        assembler.write_cgb_line(0, &memory, &palettes);

        assert_eq!(rendered_pixel(&assembler, 4), [0xaa, 0x00, 0x00]);
    }

    #[test]
    fn bg_priority_masks_highest_priority_object_without_showing_lower_object() {
        let mut assembler = VideoFrameAssembler::default();
        let mut memory = GameBoyMemory::default();
        let mut palettes = PaletteState::default();
        memory.io_ports[0x40] =
            LCDC_DISPLAY_ENABLE | LCDC_BG_ENABLE | LCDC_OBJ_ENABLE | LCDC_TILE_DATA;
        memory.vram[0x1800] = 3;
        palettes.translated_bg[1] = GREEN;
        palettes.translated_obj[1] = RED;
        palettes.translated_obj[5] = BLUE;
        fill_tile_row(&mut memory, 3, 1);
        set_object(&mut memory, 0, 16, 8, 1, 0x80);
        set_object(&mut memory, 1, 16, 8, 2, 0x10);
        fill_tile_row(&mut memory, 1, 1);
        fill_tile_row(&mut memory, 2, 1);

        assembler.write_gb_line(0, &memory, &palettes);

        assert_eq!(rendered_pixel(&assembler, 0), [0x00, 0xaa, 0x00]);
    }

    #[test]
    fn cgb_lcdc_bit_zero_does_not_disable_background_pixels() {
        let mut assembler = VideoFrameAssembler::default();
        let mut memory = GameBoyMemory::default();
        let mut palettes = PaletteState::default();
        memory.io_ports[0x40] = LCDC_DISPLAY_ENABLE | LCDC_TILE_DATA;
        memory.vram[0x1800] = 1;
        fill_tile_row(&mut memory, 1, 1);
        palettes.cgb_bg[1] = RED;

        assembler.write_cgb_line(0, &memory, &palettes);

        assert_eq!(rendered_pixel(&assembler, 0), [0xaa, 0x00, 0x00]);
    }

    #[test]
    fn cgb_lcdc_bit_zero_disables_bg_priority_not_bg_pixels() {
        let mut assembler = VideoFrameAssembler::default();
        let mut memory = GameBoyMemory::default();
        let mut palettes = PaletteState::default();
        memory.io_ports[0x40] = LCDC_DISPLAY_ENABLE | LCDC_OBJ_ENABLE | LCDC_TILE_DATA;
        memory.vram[0x1800] = 1;
        memory.vram[0x2000 + 0x1800] = 0x80;
        fill_tile_row(&mut memory, 1, 1);
        fill_tile_row(&mut memory, 2, 1);
        palettes.cgb_bg[1] = GREEN;
        palettes.cgb_obj[1] = RED;
        set_object(&mut memory, 0, 16, 8, 2, 0);

        assembler.write_cgb_line(0, &memory, &palettes);

        assert_eq!(rendered_pixel(&assembler, 0), [0xaa, 0x00, 0x00]);
    }

    #[test]
    fn cgb_lcdc_bit_zero_does_not_disable_window_pixels() {
        let mut assembler = VideoFrameAssembler::default();
        let mut memory = GameBoyMemory::default();
        let mut palettes = PaletteState::default();
        memory.io_ports[0x40] =
            LCDC_DISPLAY_ENABLE | LCDC_WINDOW_ENABLE | LCDC_WINDOW_TILE_MAP | LCDC_TILE_DATA;
        memory.io_ports[0x4a] = 0;
        memory.io_ports[0x4b] = 7;
        memory.vram[0x1800] = 1;
        memory.vram[0x1c00] = 2;
        fill_tile_row(&mut memory, 1, 1);
        fill_tile_row(&mut memory, 2, 2);
        palettes.cgb_bg[1] = BLUE;
        palettes.cgb_bg[2] = RED;

        assembler.write_cgb_line(0, &memory, &palettes);

        assert_eq!(rendered_pixel(&assembler, 0), [0xaa, 0x00, 0x00]);
    }

    #[test]
    fn dmg_lcdc_bit_zero_disables_window_pixels() {
        let mut assembler = VideoFrameAssembler::default();
        let mut memory = GameBoyMemory::default();
        let mut palettes = PaletteState::default();
        memory.io_ports[0x40] =
            LCDC_DISPLAY_ENABLE | LCDC_WINDOW_ENABLE | LCDC_WINDOW_TILE_MAP | LCDC_TILE_DATA;
        memory.io_ports[0x4a] = 0;
        memory.io_ports[0x4b] = 7;
        memory.vram[0x1c00] = 1;
        fill_tile_row(&mut memory, 1, 1);
        palettes.translated_bg[1] = RED;

        assembler.write_gb_line(0, &memory, &palettes);

        assert_ne!(rendered_pixel(&assembler, 0), [0xaa, 0x00, 0x00]);
    }

    #[test]
    fn cgb_line_renderer_uses_tile_attributes_and_palettes() {
        let mut assembler = VideoFrameAssembler::default();
        let mut memory = GameBoyMemory::default();
        let mut palettes = PaletteState::default();
        memory.io_ports[0x40] = LCDC_DISPLAY_ENABLE | LCDC_BG_ENABLE | LCDC_TILE_DATA;
        memory.vram[0x1800] = 1;
        memory.vram[0x2000 + 0x1800] = 0x02;
        memory.tile_set[64] = 1;
        palettes.cgb_bg[9] = 0xff11_2233;

        assembler.write_cgb_line(0, &memory, &palettes);

        assert_eq!(&assembler.pixels[0..3], &[0x11, 0x22, 0x33]);
    }

    #[test]
    fn sgb_colourise_maps_mono_tiles_through_character_palettes() {
        let mut assembler = VideoFrameAssembler::default();
        let mut sgb = SgbState::default();
        sgb.mono_data[0] = 2;
        sgb.character_palettes[0] = 1;
        sgb.palettes[6] = 0xff44_5566;

        assembler.colourise_sgb_frame(&sgb);

        assert_eq!(&assembler.pixels[0..3], &[0x44, 0x55, 0x66]);
    }

    #[test]
    fn sgb_colourise_has_visible_default_palette_before_commands() {
        let mut assembler = VideoFrameAssembler::default();
        let sgb = SgbState::default();

        assembler.colourise_sgb_frame(&sgb);

        assert_ne!(&assembler.pixels[0..3], &[0x00, 0x00, 0x00]);
    }

    #[test]
    fn publishing_marks_frame_complete() {
        let mut assembler = VideoFrameAssembler::default();
        let mut frames = GameBoyFrameRing::default();

        assembler.publish_frame(&mut frames);

        assert!(frames.latest_written_frame().is_some());
        assert!(!assembler.frame_in_progress);
    }
}
