use crate::game_boy::emulator::constants::SRAM_CAPACITY_BYTES;
use crate::game_boy::emulator::rom::{MemoryBankController, RomProperties};

const RAM_SIZE_INDEX: usize = 0x0149;

#[derive(Debug)]
pub(crate) struct SramState {
    pub(crate) data: Box<[u8]>,
    pub(crate) has_battery: bool,
    pub(crate) has_timer: bool,
    pub(crate) timer_data: [u8; 5],
    pub(crate) timer_mode: u32,
    pub(crate) timer_latch: u32,
    pub(crate) bank_offset: u32,
    pub(crate) size_enum: u8,
    pub(crate) size_bytes: u32,
    pub(crate) bank_select_mask: u8,
    pub(crate) enable_flag: bool,
}

impl Default for SramState {
    fn default() -> Self {
        Self {
            data: vec![0; SRAM_CAPACITY_BYTES].into_boxed_slice(),
            has_battery: false,
            has_timer: false,
            timer_data: [0; 5],
            timer_mode: 0,
            timer_latch: 0,
            bank_offset: 0,
            size_enum: 0,
            size_bytes: 0,
            bank_select_mask: 0,
            enable_flag: false,
        }
    }
}

impl SramState {
    pub(crate) fn reset_for_rom_load(&mut self, properties: &RomProperties, rom_bytes: &[u8]) {
        self.data.fill(0);
        self.has_battery = false;
        self.has_timer = false;
        self.timer_data = [0; 5];
        self.timer_mode = 0;
        self.timer_latch = 0;
        self.bank_offset = 0;
        self.size_enum = 0;
        self.size_bytes = 0;
        self.bank_select_mask = 0;
        self.enable_flag = false;

        let cart_type = properties.cart_type as u8;
        let size_enum = rom_bytes.get(RAM_SIZE_INDEX).copied().unwrap_or_default();
        self.size_enum = size_enum;
        self.has_battery = matches!(
            cart_type,
            0x09 | 0x03 | 0x06 | 0x0f | 0x10 | 0x13 | 0x1b | 0x1e
        );
        self.has_timer = matches!(cart_type, 0x0f | 0x10);
        self.size_bytes = match size_enum {
            0x00 if properties.mbc == MemoryBankController::Mbc2 => 512,
            0x00 => 0,
            0x01 => 2_048,
            0x02 => 8_192,
            0x03 => 32_768,
            _ => 0,
        };
        if self.data.len() != SRAM_CAPACITY_BYTES {
            self.data = vec![0; SRAM_CAPACITY_BYTES].into_boxed_slice();
        }
    }
}
