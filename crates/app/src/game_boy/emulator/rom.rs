use crate::game_boy::emulator::constants::INITIAL_ROM_BANK_OFFSET;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum MemoryBankController {
    #[default]
    None,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc4,
    Mbc5,
    Mmm01,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RomProperties {
    pub(crate) valid: bool,
    pub(crate) title: [u8; 17],
    pub(crate) mbc: MemoryBankController,
    pub(crate) cgb_flag: bool,
    pub(crate) sgb_flag: bool,
    pub(crate) has_sram: bool,
    pub(crate) has_rumble: bool,
    pub(crate) size_bytes: i32,
    pub(crate) bank_select_mask: u32,
    pub(crate) mbc_mode: u32,
    pub(crate) cart_type: u32,
    pub(crate) check_sum: u32,
    pub(crate) size_enum: u32,
}

#[derive(Debug)]
pub(crate) struct RomState {
    pub(crate) properties: RomProperties,
    pub(crate) bank_offset: u32,
    pub(crate) current_opened_file: String,
}

impl Default for RomState {
    fn default() -> Self {
        Self {
            properties: RomProperties::default(),
            bank_offset: INITIAL_ROM_BANK_OFFSET,
            current_opened_file: String::new(),
        }
    }
}

impl RomState {
    pub(crate) fn reset_for_rom_load(
        &mut self,
        properties: RomProperties,
        current_opened_file: String,
    ) {
        self.properties = properties;
        self.bank_offset = INITIAL_ROM_BANK_OFFSET;
        self.current_opened_file = current_opened_file;
    }
}
