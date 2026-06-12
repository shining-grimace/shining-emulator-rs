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
        if lcd_control & LCDC_DISPLAY_ENABLE == 0 || lcd_control & 0x23 == 0 {
            self.fill_line(line_no, BLACK_RGB);
            return;
        }

        let mut bg_colour_numbers = [0_u32; GAME_BOY_SCREEN_WIDTH];
        if lcd_control & LCDC_BG_ENABLE != 0 {
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

        if lcd_control & LCDC_WINDOW_ENABLE != 0 {
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
        if lcd_control & LCDC_DISPLAY_ENABLE == 0 || lcd_control & 0x23 == 0 {
            self.fill_line(line_no, BLACK_RGB);
            return;
        }

        let mut bg_colour_numbers = [0_u32; GAME_BOY_SCREEN_WIDTH];
        let mut bg_display_priorities = [false; GAME_BOY_SCREEN_WIDTH];
        if lcd_control & LCDC_BG_ENABLE != 0 {
            self.draw_cgb_background_line(
                line_no,
                lcd_control,
                memory,
                palettes,
                &mut bg_colour_numbers,
                &mut bg_display_priorities,
            );
        } else {
            self.fill_line(line_no, BLACK_RGB);
        }

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
        if lcd_control & LCDC_DISPLAY_ENABLE == 0 || lcd_control & 0x23 == 0 {
            fill_sgb_mono_line(sgb, line_no, 0);
            return;
        }

        if lcd_control & LCDC_BG_ENABLE != 0 {
            draw_sgb_background_line(line_no, lcd_control, memory, palettes, sgb);
        } else {
            fill_sgb_mono_line(sgb, line_no, 0);
        }

        if lcd_control & LCDC_WINDOW_ENABLE != 0 {
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
        let large_sprites = lcd_control & LCDC_OBJ_SIZE != 0;
        let sprite_height = if large_sprites { 16 } else { 8 };

        for sprite_index in (0..40).rev() {
            let offset = sprite_index * 4;
            let sprite_y = usize::from(memory.oam[offset]);
            let sprite_x = usize::from(memory.oam[offset + 1]);
            if sprite_y == 0 || sprite_y > 159 || sprite_x == 0 || sprite_x > 167 {
                continue;
            }
            if line_no + 16 < sprite_y || line_no >= sprite_y {
                continue;
            }

            let mut tile_no = usize::from(memory.oam[offset + 2]);
            let sprite_flags = memory.oam[offset + 3];
            let line_in_sprite = line_no + 16 - sprite_y;
            if large_sprites {
                if line_in_sprite >= 8 {
                    tile_no |= 0x01;
                } else {
                    tile_no &= 0xfe;
                }
            } else if line_in_sprite >= sprite_height {
                continue;
            }

            let mut pixel_y = line_in_sprite % 8;
            if sprite_flags & 0x40 != 0 {
                pixel_y = 7 - pixel_y;
                if large_sprites {
                    tile_no ^= 0x01;
                }
            }

            let palette_offset = if sprite_flags & 0x10 != 0 { 4 } else { 0 };
            let bg_priority = sprite_flags & 0x80 != 0;
            let screen_start_x = sprite_x.saturating_sub(8);
            let first_pixel_x = if sprite_x < 8 { 8 - sprite_x } else { 0 };
            let pixel_count = if sprite_x < 8 {
                sprite_x
            } else if sprite_x > GAME_BOY_SCREEN_WIDTH {
                168 - sprite_x
            } else {
                8
            };

            for drawn_pixel in 0..pixel_count {
                let screen_x = screen_start_x + drawn_pixel;
                if screen_x >= GAME_BOY_SCREEN_WIDTH {
                    continue;
                }
                let source_x = first_pixel_x + drawn_pixel;
                let pixel_x = if sprite_flags & 0x20 != 0 {
                    7 - source_x
                } else {
                    source_x
                };
                let colour_index = tile_pixel(memory, tile_no, pixel_x, pixel_y);
                if colour_index == 0 {
                    continue;
                }
                if bg_priority && bg_colour_numbers[screen_x] != 0 {
                    continue;
                }
                self.write_pixel(
                    line_no,
                    screen_x,
                    palettes.translated_obj[palette_offset + colour_index as usize],
                );
            }
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
        let large_sprites = lcd_control & LCDC_OBJ_SIZE != 0;
        let sprite_height = if large_sprites { 16 } else { 8 };

        for sprite_index in (0..40).rev() {
            let offset = sprite_index * 4;
            let sprite_y = usize::from(memory.oam[offset]);
            let sprite_x = usize::from(memory.oam[offset + 1]);
            if sprite_y == 0 || sprite_y > 159 || sprite_x == 0 || sprite_x > 167 {
                continue;
            }
            if line_no + 16 < sprite_y || line_no >= sprite_y {
                continue;
            }

            let mut tile_no = usize::from(memory.oam[offset + 2]);
            let sprite_flags = memory.oam[offset + 3];
            let line_in_sprite = line_no + 16 - sprite_y;
            if large_sprites {
                if line_in_sprite >= 8 {
                    tile_no |= 0x01;
                } else {
                    tile_no &= 0xfe;
                }
            } else if line_in_sprite >= sprite_height {
                continue;
            }
            if sprite_flags & 0x08 != 0 {
                tile_no += 384;
            }

            let mut pixel_y = line_in_sprite % 8;
            if sprite_flags & 0x40 != 0 {
                pixel_y = 7 - pixel_y;
                if large_sprites {
                    tile_no ^= 0x01;
                }
            }

            let palette_offset = usize::from(sprite_flags & 0x07) * 4;
            let bg_priority = sprite_flags & 0x80 != 0;
            let screen_start_x = sprite_x.saturating_sub(8);
            let first_pixel_x = if sprite_x < 8 { 8 - sprite_x } else { 0 };
            let pixel_count = if sprite_x < 8 {
                sprite_x
            } else if sprite_x > GAME_BOY_SCREEN_WIDTH {
                168 - sprite_x
            } else {
                8
            };

            for drawn_pixel in 0..pixel_count {
                let screen_x = screen_start_x + drawn_pixel;
                if screen_x >= GAME_BOY_SCREEN_WIDTH {
                    continue;
                }
                let source_x = first_pixel_x + drawn_pixel;
                let pixel_x = if sprite_flags & 0x20 != 0 {
                    7 - source_x
                } else {
                    source_x
                };
                let colour_index = tile_pixel(memory, tile_no, pixel_x, pixel_y);
                if colour_index == 0 {
                    continue;
                }
                if (bg_display_priorities[screen_x] || bg_priority)
                    && bg_colour_numbers[screen_x] != 0
                {
                    continue;
                }
                self.write_pixel(
                    line_no,
                    screen_x,
                    palettes.cgb_obj[palette_offset + colour_index as usize],
                );
            }
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
    let large_sprites = lcd_control & LCDC_OBJ_SIZE != 0;
    let sprite_height = if large_sprites { 16 } else { 8 };

    for sprite_index in (0..40).rev() {
        let offset = sprite_index * 4;
        let sprite_y = usize::from(memory.oam[offset]);
        let sprite_x = usize::from(memory.oam[offset + 1]);
        if sprite_y == 0 || sprite_y > 159 || sprite_x == 0 || sprite_x > 167 {
            continue;
        }
        if line_no + 16 < sprite_y || line_no >= sprite_y {
            continue;
        }

        let mut tile_no = usize::from(memory.oam[offset + 2]);
        let sprite_flags = memory.oam[offset + 3];
        let line_in_sprite = line_no + 16 - sprite_y;
        if large_sprites {
            if line_in_sprite >= 8 {
                tile_no |= 0x01;
            } else {
                tile_no &= 0xfe;
            }
        } else if line_in_sprite >= sprite_height {
            continue;
        }

        let mut pixel_y = line_in_sprite % 8;
        if sprite_flags & 0x40 != 0 {
            pixel_y = 7 - pixel_y;
            if large_sprites {
                tile_no ^= 0x01;
            }
        }

        let palette_offset = if sprite_flags & 0x10 != 0 { 4 } else { 0 };
        let bg_priority = sprite_flags & 0x80 != 0;
        let screen_start_x = sprite_x.saturating_sub(8);
        let first_pixel_x = if sprite_x < 8 { 8 - sprite_x } else { 0 };
        let pixel_count = if sprite_x < 8 {
            sprite_x
        } else if sprite_x > GAME_BOY_SCREEN_WIDTH {
            168 - sprite_x
        } else {
            8
        };

        for drawn_pixel in 0..pixel_count {
            let screen_x = screen_start_x + drawn_pixel;
            if screen_x >= GAME_BOY_SCREEN_WIDTH {
                continue;
            }
            let source_x = first_pixel_x + drawn_pixel;
            let pixel_x = if sprite_flags & 0x20 != 0 {
                7 - source_x
            } else {
                source_x
            };
            let colour_index = tile_pixel(memory, tile_no, pixel_x, pixel_y);
            if colour_index == 0 {
                continue;
            }
            let mono_index = sgb
                .mono_data
                .get(line_no * GAME_BOY_SCREEN_WIDTH + screen_x)
                .copied()
                .unwrap_or(0);
            if bg_priority && mono_index != palettes.sgb_translation_bg[0] {
                continue;
            }
            write_sgb_mono_pixel(
                sgb,
                line_no,
                screen_x,
                palettes.sgb_translation_obj[palette_offset + colour_index as usize],
            );
        }
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
    fn publishing_marks_frame_complete() {
        let mut assembler = VideoFrameAssembler::default();
        let mut frames = GameBoyFrameRing::default();

        assembler.publish_frame(&mut frames);

        assert!(frames.latest_written_frame().is_some());
        assert!(!assembler.frame_in_progress);
    }
}
