//! Dot-stepped LCD fetchers. CPU memory accesses remain aligned to M-cycles,
//! while fetch reads, FIFO consumption, and LCD pixel mixing run every dot.
//! References: <https://gbdev.io/pandocs/pixel_fifo.html> and Rendering.html.
use super::GameBoyCore;
use super::gpu::{LineRenderer, OamScanline};
use super::memory::GameBoyMemory;
use super::palettes::PaletteState;
use super::sgb::SgbState;
use super::video::VideoFrameAssembler;

mod background;
mod fetch;
mod mixer;
mod objects;
mod state;
mod window;
pub(crate) use state::PixelPipeline;

struct Context<'a> {
    memory: &'a GameBoyMemory,
    palettes: &'a PaletteState,
    oam: &'a mut OamScanline,
    video: &'a mut VideoFrameAssembler,
    sgb: &'a mut SgbState,
    renderer: LineRenderer,
    cgb: bool,
    dma_word: [u8; 2],
    tile_select_change: Option<bool>,
}

fn context(core: &mut GameBoyCore) -> (Context<'_>, &mut PixelPipeline) {
    (
        Context {
            memory: &core.memory,
            palettes: &core.palettes,
            oam: &mut core.gpu_timing.oam_scanline,
            video: &mut core.video_frame,
            sgb: &mut core.sgb,
            renderer: core.line_renderer,
            cgb: core.rom.properties.cgb_flag,
            dma_word: core.dma.oam.oam_word,
            tile_select_change: None,
        },
        &mut core.gpu_timing.pixel_pipeline,
    )
}

fn io(core: &Context<'_>, index: usize) -> u8 {
    core.memory.io_ports[index]
}

pub(crate) fn scanline_start(core: &mut GameBoyCore) {
    if core.memory.io_ports[0x44] == core.memory.io_ports[0x4a] {
        core.gpu_timing.pixel_pipeline.window_y_triggered = true;
    }
}

pub(crate) fn begin(core: &mut GameBoyCore) {
    if !core.gpu_timing.oam_scanline.valid {
        scanline_start(core);
        let selected = crate::game_boy::emulator::video::select_objects_for_line(
            &core.memory,
            core.memory.io_ports[0x40],
            usize::from(core.memory.io_ports[0x44]),
        );
        core.gpu_timing.oam_scanline.bytes = core.memory.oam;
        core.gpu_timing.oam_scanline.selected =
            selected.iter().fold(0, |mask, o| mask | (1 << o.index));
        core.gpu_timing.oam_scanline.valid = true;
    }
    let (core, p) = context(core);
    let window_line = p.window_line;
    let window_y_triggered = p.window_y_triggered;
    *p = PixelPipeline {
        cgb_fetch: p.cgb_fetch,
        active: true,
        dummy: 8,
        fine: io(&core, 0x43) & 7,
        window_line,
        window_y_triggered,
        ..Default::default()
    };
    core.oam.fetched = 0;
    background::begin(&core, p);
}

/// Returns true exactly when the 160th visible pixel has left the LCD FIFO.
pub(crate) fn tick(core: &mut GameBoyCore, dma_owned: bool) -> bool {
    if !core.gpu_timing.pixel_pipeline.active {
        begin(core);
    }
    let (mut core, p) = context(core);
    core.tile_select_change = p.cgb_fetch.tile_select_change.take();
    if p.x >= 160 {
        return true;
    }
    if p.object.is_none() && p.waiting_object.is_none() && window::tick(&core, p) {
        background::tick(&core, p);
        return false;
    }
    if objects::tick(&mut core, p, dma_owned) {
        return false;
    }
    if let Some(pixel) = p.bg.pop() {
        if p.dummy > 0 {
            p.dummy -= 1;
            p.objects.pop();
        } else if p.fine > 0 {
            p.fine -= 1;
        } else {
            let object = p.objects.pop().unwrap_or_default();
            mixer::emit(&mut core, p, pixel, object);
        }
    }
    background::tick(&core, p);
    if p.x == 160 {
        if p.window_used {
            p.window_line = p.window_line.wrapping_add(1);
        }
        return true;
    }
    false
}

#[cfg(test)]
mod tests;

/// Writes are observed on the next PPU dot, even if an object stalls the BG fetcher.
pub(crate) fn lcdc_write(core: &mut GameBoyCore, old: u8, new: u8) {
    if core.rom.properties.cgb_flag
        && core.gpu_mode == super::gpu::GpuMode::ScanVram
        && old & new & 0x80 != 0
        && (old ^ new) & 0x10 != 0
    {
        core.gpu_timing.pixel_pipeline.cgb_fetch.tile_select_change = Some(new & 0x10 != 0);
    }
}

pub(crate) fn frame_start(core: &mut GameBoyCore) {
    let cgb_fetch = core.gpu_timing.pixel_pipeline.cgb_fetch;
    core.gpu_timing.pixel_pipeline = PixelPipeline {
        cgb_fetch,
        ..Default::default()
    };
}
