//! PPU OAM ownership is separate from CPU access and the final OAM contents.
//! Mode-3 fetch positions follow this core's existing scanline penalty model;
//! exact dot placement still depends on the future pixel FIFO implementation.
use super::{
    GameBoyCore, GpuMode, LCDC_IO_INDEX, LY_IO_INDEX, SCX_IO_INDEX, WX_IO_INDEX, io,
    object_penalty_tile, window_visible_on_line,
};
use crate::game_boy::emulator::gpu::OamScanline;
use crate::game_boy::emulator::video::{
    SelectedObject, select_objects_for_line, select_objects_from_oam,
};

pub(super) fn line_objects(core: &GameBoyCore) -> Vec<SelectedObject> {
    let lcd = io(&core.memory, LCDC_IO_INDEX);
    let line = usize::from(io(&core.memory, LY_IO_INDEX));
    if core.gpu_timing.oam_scanline.valid {
        select_objects_from_oam(&core.gpu_timing.oam_scanline.bytes, lcd, line)
    } else {
        // Standalone renderer tests and old save states without PPU latches.
        select_objects_for_line(&core.memory, lcd, line)
    }
}

pub(super) fn observe(core: &mut GameBoyCore, start: i32, end: i32, dma_owned: bool) {
    match core.gpu_mode {
        GpuMode::ScanOam => scan(core, start, end, dma_owned),
        GpuMode::ScanVram => fetch(core, end, dma_owned),
        _ => {}
    }
}

fn scan(core: &mut GameBoyCore, start: i32, end: i32, dma_owned: bool) {
    if start == 0 {
        core.gpu_timing.oam_scanline = OamScanline {
            valid: true,
            ..Default::default()
        };
    }
    let line = i32::from(io(&core.memory, LY_IO_INDEX));
    let height = if io(&core.memory, LCDC_IO_INDEX) & 4 == 0 {
        8
    } else {
        16
    };
    let cache = &mut core.gpu_timing.oam_scanline;
    // One OAM entry every two dots. DMA makes its Y/X read off-screen,
    // without changing the actual OAM bytes being copied by the DMA engine.
    for index in start / 2..(end / 2).min(40) {
        if dma_owned || cache.selected.count_ones() == 10 {
            continue;
        }
        let index = usize::try_from(index).unwrap_or_default();
        let offset = index * 4;
        let top = i32::from(core.memory.oam[offset]) - 16;
        if line >= top && line < top + height {
            cache.bytes[offset..offset + 4].copy_from_slice(&core.memory.oam[offset..offset + 4]);
            cache.selected |= 1 << index;
        }
    }
}

fn fetch(core: &mut GameBoyCore, end: i32, dma_owned: bool) {
    if !core.gpu_timing.oam_scanline.valid || io(&core.memory, LCDC_IO_INDEX) & 2 == 0 {
        return;
    }
    let scroll = i32::from(io(&core.memory, SCX_IO_INDEX));
    let lcd = io(&core.memory, LCDC_IO_INDEX);
    let line = io(&core.memory, LY_IO_INDEX);
    let window = window_visible_on_line(&core.memory, lcd, line, core.rom.properties.cgb_flag)
        .then(|| i32::from(io(&core.memory, WX_IO_INDEX)) - 7);
    let cache = &mut core.gpu_timing.oam_scanline;
    let mut indices = [0_usize; 10];
    let mut mask = cache.selected;
    let mut count = 0;
    while mask != 0 {
        indices[count] = usize::try_from(mask.trailing_zeros()).unwrap_or_default();
        count += 1;
        mask &= mask - 1;
    }
    indices[..count].sort_by_key(|index| (cache.bytes[index * 4 + 1], *index));
    let mut stalls = 0;
    let mut tiles = [i32::MIN; 10];
    let mut tile_count = 0;
    for &index in &indices[..count] {
        let offset = index * 4;
        let x = cache.bytes[offset + 1];
        if x >= 168 {
            continue;
        }
        let screen_x = i32::from(x) - 8;
        let window_stall = i32::from(window.is_some_and(|wx| screen_x >= wx)) * 6;
        let fetch_dot = 12 + (scroll & 7) + screen_x.max(0) + stalls + window_stall;
        if cache.fetched & (1 << index) == 0 && fetch_dot <= end {
            let word = if dma_owned {
                core.dma.oam.oam_word
            } else {
                [core.memory.oam[offset + 2], core.memory.oam[offset + 3]]
            };
            cache.bytes[offset + 2..offset + 4].copy_from_slice(&word);
            cache.fetched |= 1 << index;
        }
        if x == 0 {
            stalls += 11;
            continue;
        }
        let (tile, pixel) = object_penalty_tile(screen_x, scroll, window);
        if !tiles[..tile_count].contains(&tile) {
            tiles[tile_count] = tile;
            tile_count += 1;
            stalls += (5 - pixel).max(0);
        }
        stalls += 6;
    }
}

#[cfg(test)]
mod tests;
