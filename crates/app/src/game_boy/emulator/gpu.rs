use crate::game_boy::emulator::constants::{INITIAL_VRAM_BANK_OFFSET, INITIAL_WRAM_BANK_OFFSET};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GpuMode {
    HBlank,
    VBlank,
    ScanOam,
    ScanVram,
}

impl Default for GpuMode {
    fn default() -> Self {
        Self::VBlank
    }
}

impl GpuMode {
    pub(crate) fn reset_for_rom_load(&mut self) {
        *self = Self::ScanOam;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LineRenderer {
    Gb,
    Sgb,
    Cgb,
}

impl Default for LineRenderer {
    fn default() -> Self {
        Self::Gb
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GpuTiming {
    pub(crate) clock_factor: i32,
    pub(crate) time_in_mode: i32,
    pub(crate) last_ly_compare: u32,
    pub(crate) blanked_screen: bool,
    pub(crate) need_clear: bool,
}

impl Default for GpuTiming {
    fn default() -> Self {
        Self {
            clock_factor: 1,
            time_in_mode: 0,
            last_ly_compare: 0,
            blanked_screen: false,
            need_clear: true,
        }
    }
}

impl GpuTiming {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.clock_factor = 1;
        self.time_in_mode = 0;
        self.last_ly_compare = 1;
        self.blanked_screen = false;
        self.need_clear = false;
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MemoryAccess {
    pub(crate) oam: bool,
    pub(crate) vram: bool,
    pub(crate) wram_bank_offset: u32,
    pub(crate) vram_bank_offset: u32,
}

impl Default for MemoryAccess {
    fn default() -> Self {
        Self {
            oam: false,
            vram: false,
            wram_bank_offset: 0,
            vram_bank_offset: 0,
        }
    }
}

impl MemoryAccess {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.oam = true;
        self.vram = true;
        self.wram_bank_offset = INITIAL_WRAM_BANK_OFFSET;
        self.vram_bank_offset = INITIAL_VRAM_BANK_OFFSET;
    }
}
