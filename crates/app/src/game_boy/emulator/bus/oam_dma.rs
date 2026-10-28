//! OAM DMA has its own address decoder and owns only its source bus plus OAM.
//! References: Pan Docs OAM DMA Transfer; Mooneye `oam_dma_start` and `sources-GS`.
use super::{GameBoyCore, read_sram, read8_unrestricted};

pub(crate) fn begin_deferred_oam_dma(core: &mut GameBoyCore) {
    // Called at the end of the actual FF46 write M-cycle. Keep the old DMA
    // running through the following M-cycle when this is a restart.
    if core.dma.oam.pending_source_high.is_some() {
        core.dma.oam.startup_pending = true;
    }
}

pub(crate) fn step_oam_dma(core: &mut GameBoyCore) {
    let starting = core.dma.oam.startup_pending;
    // Older save states used startup_pending with no separate pending source.
    let legacy_startup = starting && core.dma.oam.pending_source_high.is_none();
    if core.dma.oam.active && !legacy_startup {
        let index = core.dma.oam.next_index;
        let source = u16::from_be_bytes([core.dma.oam.source_high, index]);
        let byte = read_source(core, source);
        core.dma.oam.bus_value = byte;
        if let Some(slot) = core.memory.oam.get_mut(usize::from(index)) {
            *slot = byte;
            let word = usize::from(index & !1);
            core.dma.oam.oam_word = [core.memory.oam[word], core.memory.oam[word + 1]];
        }
        core.dma.oam.next_index = index.saturating_add(1);
        if core.dma.oam.next_index >= 160 {
            core.dma.oam.active = false;
        }
    }
    if starting {
        core.dma.oam.startup_pending = false;
        if let Some(source) = core.dma.oam.pending_source_high.take() {
            core.dma.oam.source_high = source;
            core.dma.oam.next_index = 0;
            core.dma.oam.active = true;
            core.dma.oam.bus_value = 0xff;
        }
    }
}

fn read_source(core: &GameBoyCore, address: u16) -> u8 {
    match address {
        0x8000..=0x9fff => {
            let bank = usize::try_from(core.memory_access.vram_bank_offset).unwrap_or_default();
            // DMA bypasses CPU VRAM accessibility and cheat patches.
            core.memory
                .vram
                .get(bank + usize::from(address & 0x1fff))
                .copied()
                .unwrap_or(0xff)
        }
        0xe000..=0xffff if core.rom.properties.cgb_flag => 0xff,
        0xe000..=0xffff => read8_unrestricted(core, address & 0xdfff),
        0xa000..=0xbfff => read_sram(core, usize::from(address)),
        _ => read8_unrestricted(core, address),
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Bus {
    Cartridge,
    Wram,
    Vram,
}

fn source_bus(core: &GameBoyCore) -> Bus {
    match core.dma.oam.source_high {
        0x80..=0x9f => Bus::Vram,
        0xc0..=0xff => Bus::Wram,
        _ => Bus::Cartridge,
    }
}

pub(super) fn oam_dma_blocks_cpu_access(core: &GameBoyCore, address: u16) -> bool {
    if !core.dma.oam.active || address >= 0xff00 {
        // IO (including FF46), HRAM, and IE are on the internal CPU bus.
        return false;
    }
    if address >= 0xfe00 {
        return true;
    }
    let target = match address {
        0x8000..=0x9fff => Bus::Vram,
        0xc000..=0xfdff => Bus::Wram,
        _ => Bus::Cartridge,
    };
    let source = source_bus(core);
    if core.rom.properties.cgb_flag {
        source == target || (core.dma.oam.source_high >= 0xe0 && target == Bus::Cartridge)
    } else {
        // DMG cartridge and WRAM share a bus; VRAM is separate.
        (source == Bus::Vram) == (target == Bus::Vram)
    }
}

#[cfg(test)]
mod tests;
