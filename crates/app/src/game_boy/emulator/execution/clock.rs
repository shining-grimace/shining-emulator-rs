//! CPU bus cycles share the peripheral clock used by HALT and interrupt entry.
//! Raw bus helpers remain untimed for DMA, debugger reads, and test inspection.
use crate::game_boy::emulator::systems::step_machine_cycle;
use crate::game_boy::emulator::{GameBoyCore, bus};
use crate::game_boy::frame_buffer::GameBoyFrameRing;

pub(super) struct CpuClock<'a> {
    frames: &'a mut GameBoyFrameRing,
    pub(super) clocks: i32,
}

impl<'a> CpuClock<'a> {
    pub(super) fn new(frames: &'a mut GameBoyFrameRing) -> Self {
        Self { frames, clocks: 0 }
    }

    pub(super) fn idle(&mut self, core: &mut GameBoyCore) {
        self.wait_for_dma(core);
        self.tick(core);
    }

    fn wait_for_dma(&mut self, core: &mut GameBoyCore) {
        // VRAM DMA may start in the middle of an instruction at HBlank. Its
        // stalls advance peripherals but do not count as instruction cycles.
        while core.dma.vram.cpu_halt_m_cycles > 0 {
            core.dma.vram.cpu_halt_m_cycles -= 1;
            step_machine_cycle(core, self.frames);
        }
    }

    fn tick(&mut self, core: &mut GameBoyCore) {
        step_machine_cycle(core, self.frames);
        self.clocks += 4;
    }
}

pub(super) fn read8(core: &mut GameBoyCore, clock: &mut CpuClock<'_>, address: u16) -> u8 {
    clock.wait_for_dma(core);
    let value = bus::read8(core, address);
    clock.tick(core);
    value
}

pub(super) fn write8(core: &mut GameBoyCore, clock: &mut CpuClock<'_>, address: u16, value: u8) {
    clock.wait_for_dma(core);
    bus::write8(core, address, value);
    clock.tick(core);
    bus::begin_deferred_oam_dma(core);
}

pub(super) fn write16(core: &mut GameBoyCore, clock: &mut CpuClock<'_>, address: u16, value: u16) {
    let [low, high] = value.to_le_bytes();
    write8(core, clock, address, low);
    write8(core, clock, address.wrapping_add(1), high);
}
