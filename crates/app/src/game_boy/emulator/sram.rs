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
    dirty: bool,
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
            dirty: false,
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
        self.dirty = false;

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

    pub(crate) fn persistence_len(&self) -> Option<usize> {
        if !self.has_battery || self.size_bytes == 0 {
            return None;
        }
        usize::try_from(self.size_bytes)
            .ok()
            .map(|size_bytes| size_bytes.min(self.data.len()))
    }

    pub(crate) fn load_save_data(&mut self, saved_data: &[u8]) {
        let Some(size_bytes) = self.persistence_len() else {
            return;
        };
        let copy_len = saved_data.len().min(size_bytes).min(self.data.len());
        self.data[..copy_len].copy_from_slice(&saved_data[..copy_len]);
        self.dirty = false;
    }

    pub(crate) fn save_data(&self) -> Option<&[u8]> {
        self.persistence_len()
            .and_then(|size_bytes| self.data.get(..size_bytes))
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub(crate) fn read_data(&self, address: usize) -> u8 {
        let Some(index) = self.data_index(address) else {
            return 0xff;
        };
        self.data.get(index).copied().unwrap_or(0xff)
    }

    pub(crate) fn write_data(&mut self, address: usize, value: u8) {
        let Some(index) = self.data_index(address) else {
            return;
        };
        if let Some(slot) = self.data.get_mut(index) {
            if *slot == value {
                return;
            }
            *slot = value;
            self.dirty |= self.has_battery;
        }
    }

    pub(crate) fn write_timer_data(&mut self, timer_index: usize, value: u8) {
        if let Some(slot) = self.timer_data.get_mut(timer_index) {
            *slot = value;
        }
    }

    fn data_index(&self, address: usize) -> Option<usize> {
        if self.size_bytes == 0 {
            return None;
        }
        let bank_offset = usize::try_from(self.bank_offset).ok()?;
        let size = usize::try_from(self.size_bytes).ok()?;
        let offset = (address & 0x1fff) % size.min(0x2000);
        bank_offset
            .checked_add(offset)
            .filter(|index| *index < size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_sram_loads_bytes_and_tracks_dirty_writes() {
        let mut sram = SramState {
            has_battery: true,
            size_bytes: 8_192,
            ..Default::default()
        };
        let mut saved_data = vec![0; 8_192];
        saved_data[0x0123] = 0x5a;

        sram.load_save_data(&saved_data);
        assert_eq!(sram.read_data(0x0123), 0x5a);
        assert!(!sram.is_dirty());

        sram.write_data(0x0123, 0x99);
        assert!(sram.is_dirty());
        assert_eq!(
            sram.save_data().and_then(|data| data.get(0x0123)),
            Some(&0x99)
        );
    }
}
