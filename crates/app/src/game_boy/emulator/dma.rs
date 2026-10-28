#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DmaState {
    pub(crate) oam: OamDmaState,
    pub(crate) vram: VramDmaState,
}

impl DmaState {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.oam = OamDmaState::default();
        self.vram = VramDmaState::default();
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct OamDmaState {
    pub(crate) pending_source_high: Option<u8>,
    pub(crate) source_high: u8,
    pub(crate) next_index: u8,
    pub(crate) active: bool,
    pub(crate) startup_pending: bool,
    pub(crate) bus_value: u8,
    pub(crate) oam_word: [u8; 2],
}

impl Default for OamDmaState {
    fn default() -> Self {
        Self {
            pending_source_high: None,
            source_high: 0,
            next_index: 0,
            active: false,
            startup_pending: false,
            bus_value: 0xff,
            oam_word: [0xff; 2],
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct VramDmaState {
    pub(crate) cpu_halt_m_cycles: i32,
}
