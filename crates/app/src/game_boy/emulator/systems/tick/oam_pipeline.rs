//! PPU OAM ownership is separate from CPU access and the final OAM contents.
//! Object selection reads one entry every two dots; mode-3 reads use the FIFO.
use super::{GameBoyCore, GpuMode, LCDC_IO_INDEX, LY_IO_INDEX, io};
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
    if core.gpu_mode == GpuMode::ScanOam {
        scan(core, start, end, dma_owned);
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

#[cfg(test)]
mod tests;
