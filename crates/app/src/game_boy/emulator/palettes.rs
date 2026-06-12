#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PaletteState {
    pub(crate) translated_bg: [u32; 4],
    pub(crate) translated_obj: [u32; 8],
    pub(crate) sgb_translation_bg: [u32; 4],
    pub(crate) sgb_translation_obj: [u32; 8],
    pub(crate) cgb_bg: [u32; 32],
    pub(crate) cgb_obj: [u32; 32],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CgbPaletteRegisters {
    pub(crate) bg_data: [u8; 64],
    pub(crate) bg_index: u32,
    pub(crate) bg_increment: u32,
    pub(crate) obj_data: [u8; 64],
    pub(crate) obj_index: u32,
    pub(crate) obj_increment: u32,
}

impl Default for CgbPaletteRegisters {
    fn default() -> Self {
        Self {
            bg_data: [0; 64],
            bg_index: 0,
            bg_increment: 0,
            obj_data: [0; 64],
            obj_index: 0,
            obj_increment: 0,
        }
    }
}

impl CgbPaletteRegisters {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.bg_data = [0; 64];
        self.bg_index = 0;
        self.bg_increment = 0;
        self.obj_data = [0; 64];
        self.obj_index = 0;
        self.obj_increment = 0;
    }
}
