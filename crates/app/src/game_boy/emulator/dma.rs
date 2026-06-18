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

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct OamDmaState {
    pub(crate) pending_source_high: Option<u8>,
    pub(crate) source_high: u8,
    pub(crate) next_index: u8,
    pub(crate) active: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct VramDmaState {
    pub(crate) cpu_halt_m_cycles: i32,
}
