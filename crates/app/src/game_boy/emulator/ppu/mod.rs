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
mod mixer;
mod objects;
mod state;
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
    if p.x >= 160 {
        return true;
    }
    if p.object.is_none() && p.waiting_object.is_none() && window(&core, p) {
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

fn window(core: &Context<'_>, p: &mut PixelPipeline) -> bool {
    let lcd = io(core, 0x40);
    let enabled = lcd & 0x20 != 0 && (core.cgb || lcd & 1 != 0);
    if p.window_active && !enabled {
        p.window_active = false;
        p.fine = p.x.wrapping_add(io(core, 0x43)) & 7;
        background::restart(p, p.x / 8);
        return true;
    }
    let wx = io(core, 0x4b);
    if !p.window_active
        && enabled
        && p.window_y_triggered
        && wx < 167
        && p.dummy == 0
        && p.fine == 0
        && p.bg.len != 0
        && p.x == wx.saturating_sub(7)
    {
        p.window_active = true;
        p.fine = 7_u8.saturating_sub(wx);
        background::restart(p, 0);
        return true;
    }
    false
}

#[cfg(test)]
mod tests;
