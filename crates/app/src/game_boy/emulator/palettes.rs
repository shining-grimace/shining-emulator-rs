const STOCK_PALETTE_BG: [u32; 4] = [0xffffffff, 0xff88b0b0, 0xff507878, 0xff000000];
const STOCK_PALETTE_OBJ1: [u32; 4] = [0xffffffff, 0xff5050f0, 0xff2020a0, 0xff000000];
const STOCK_PALETTE_OBJ2: [u32; 4] = [0xffffffff, 0xffa0a0a0, 0xff404040, 0xff000000];

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PaletteState {
    pub(crate) translated_bg: [u32; 4],
    pub(crate) translated_obj: [u32; 8],
    pub(crate) sgb_translation_bg: [u32; 4],
    pub(crate) sgb_translation_obj: [u32; 8],
    pub(crate) cgb_bg: [u32; 32],
    pub(crate) cgb_obj: [u32; 32],
}

impl PaletteState {
    pub(crate) fn reset_for_rom_load(&mut self, io_ports: &[u8]) {
        self.cgb_bg = [0; 32];
        self.cgb_obj = [0; 32];
        self.translate_bg(io_ports.get(0x47).copied().unwrap_or(0xfc));
        self.translate_obj1(io_ports.get(0x48).copied().unwrap_or(0xff));
        self.translate_obj2(io_ports.get(0x49).copied().unwrap_or(0xff));
    }

    pub(crate) fn translate_bg(&mut self, palette_data: u8) {
        self.translated_bg[0] = STOCK_PALETTE_BG[usize::from(palette_data & 0x03)];
        self.translated_bg[1] = STOCK_PALETTE_BG[usize::from((palette_data & 0x0c) / 4)];
        self.translated_bg[2] = STOCK_PALETTE_BG[usize::from((palette_data & 0x30) / 16)];
        self.translated_bg[3] = STOCK_PALETTE_BG[usize::from((palette_data & 0xc0) / 64)];
        self.sgb_translation_bg[0] = u32::from(palette_data & 0x03);
        self.sgb_translation_bg[1] = u32::from((palette_data & 0x0c) / 4);
        self.sgb_translation_bg[2] = u32::from((palette_data & 0x30) / 16);
        self.sgb_translation_bg[3] = u32::from((palette_data & 0xc0) / 64);
    }

    pub(crate) fn translate_obj1(&mut self, palette_data: u8) {
        self.translated_obj[1] = STOCK_PALETTE_OBJ1[usize::from((palette_data & 0x0c) / 4)];
        self.translated_obj[2] = STOCK_PALETTE_OBJ1[usize::from((palette_data & 0x30) / 16)];
        self.translated_obj[3] = STOCK_PALETTE_OBJ1[usize::from((palette_data & 0xc0) / 64)];
        self.sgb_translation_obj[1] = u32::from((palette_data & 0x0c) / 4);
        self.sgb_translation_obj[2] = u32::from((palette_data & 0x30) / 16);
        self.sgb_translation_obj[3] = u32::from((palette_data & 0xc0) / 64);
    }

    pub(crate) fn translate_obj2(&mut self, palette_data: u8) {
        self.translated_obj[5] = STOCK_PALETTE_OBJ2[usize::from((palette_data & 0x0c) / 4)];
        self.translated_obj[6] = STOCK_PALETTE_OBJ2[usize::from((palette_data & 0x30) / 16)];
        self.translated_obj[7] = STOCK_PALETTE_OBJ2[usize::from((palette_data & 0xc0) / 64)];
        self.sgb_translation_obj[5] = u32::from((palette_data & 0x0c) / 4);
        self.sgb_translation_obj[6] = u32::from((palette_data & 0x30) / 16);
        self.sgb_translation_obj[7] = u32::from((palette_data & 0xc0) / 64);
    }
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
