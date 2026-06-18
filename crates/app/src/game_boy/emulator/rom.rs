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
    pub(crate) fixed_bank_offset: u32,
    pub(crate) bank_offset: u32,
    pub(crate) mbc1_lower_bank: u8,
    pub(crate) mbc1_upper_bank: u8,
    pub(crate) suspicious_mbc_warning_logged: bool,
    pub(crate) current_rom_id: String,
    pub(crate) current_opened_file: String,
}

impl Default for RomState {
    fn default() -> Self {
        Self {
            properties: RomProperties::default(),
            fixed_bank_offset: 0,
            bank_offset: INITIAL_ROM_BANK_OFFSET,
            mbc1_lower_bank: 1,
            mbc1_upper_bank: 0,
            suspicious_mbc_warning_logged: false,
            current_rom_id: String::new(),
            current_opened_file: String::new(),
        }
    }
}

impl RomState {
    pub(crate) fn reset_for_rom_load(
        &mut self,
        properties: RomProperties,
        current_rom_id: String,
        current_opened_file: String,
    ) {
        self.properties = properties;
        self.fixed_bank_offset = 0;
        self.bank_offset = INITIAL_ROM_BANK_OFFSET;
        self.mbc1_lower_bank = 1;
        self.mbc1_upper_bank = 0;
        self.suspicious_mbc_warning_logged = false;
        self.current_rom_id = current_rom_id;
        self.current_opened_file = current_opened_file;
    }
}
