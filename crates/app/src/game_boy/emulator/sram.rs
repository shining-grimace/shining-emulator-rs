use crate::game_boy::emulator::constants::{GB_CLOCK_HZ, SRAM_CAPACITY_BYTES};
use crate::game_boy::emulator::rom::{MemoryBankController, RomProperties};

const RAM_SIZE_INDEX: usize = 0x0149;
const RTC_REGISTER_COUNT: usize = 5;
const RTC_SECONDS_INDEX: usize = 0;
const RTC_MINUTES_INDEX: usize = 1;
const RTC_HOURS_INDEX: usize = 2;
const RTC_DAY_LOW_INDEX: usize = 3;
const RTC_DAY_HIGH_INDEX: usize = 4;
const RTC_DAY_HIGH_DAY_BIT: u8 = 0x01;
const RTC_DAY_HIGH_HALT: u8 = 0x40;
const RTC_DAY_HIGH_CARRY: u8 = 0x80;
const RTC_SECOND_CLOCKS: u32 = GB_CLOCK_HZ as u32;

#[derive(Debug)]
pub(crate) struct SramState {
    pub(crate) data: Box<[u8]>,
    pub(crate) has_battery: bool,
    pub(crate) has_timer: bool,
    pub(crate) timer_data: [u8; RTC_REGISTER_COUNT],
    latched_timer_data: [u8; RTC_REGISTER_COUNT],
    timer_latched: bool,
    timer_clock_accumulator: u32,
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
            timer_data: [0; RTC_REGISTER_COUNT],
            latched_timer_data: [0; RTC_REGISTER_COUNT],
            timer_latched: false,
            timer_clock_accumulator: 0,
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
        self.timer_data = [0; RTC_REGISTER_COUNT];
        self.latched_timer_data = [0; RTC_REGISTER_COUNT];
        self.timer_latched = false;
        self.timer_clock_accumulator = 0;
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
            0x04 => 131_072,
            0x05 => 65_536,
            _ => 0,
        };
        self.bank_select_mask = match self.size_bytes {
            0..=8_192 => 0,
            32_768 => 0x03,
            65_536 => 0x07,
            131_072 => 0x0f,
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
        let value = self.data.get(index).copied().unwrap_or(0xff);
        if self.size_bytes == 512 {
            0xf0 | (value & 0x0f)
        } else {
            value
        }
    }

    pub(crate) fn write_data(&mut self, address: usize, value: u8) {
        let Some(index) = self.data_index(address) else {
            return;
        };
        if let Some(slot) = self.data.get_mut(index) {
            if *slot == value {
                return;
            }
            let value = if self.size_bytes == 512 {
                value & 0x0f
            } else {
                value
            };
            *slot = value;
            self.dirty |= self.has_battery;
        }
    }

    pub(crate) fn read_timer_data(&self, timer_index: usize) -> u8 {
        let data = if self.timer_latched {
            &self.latched_timer_data
        } else {
            &self.timer_data
        };
        data.get(timer_index).copied().unwrap_or(0xff)
    }

    pub(crate) fn write_timer_data(&mut self, timer_index: usize, value: u8) {
        if let Some(slot) = self.timer_data.get_mut(timer_index) {
            *slot = masked_timer_value(timer_index, value);
        }
    }

    pub(crate) fn latch_timer_data(&mut self, value: u8) {
        let latch = u32::from(value & 0x01);
        if self.timer_latch == 0 && latch == 1 {
            self.latched_timer_data = self.timer_data;
            self.timer_latched = true;
        }
        self.timer_latch = latch;
    }

    pub(crate) fn advance_rtc(&mut self, clocks: u32) {
        if !self.has_timer || self.timer_data[RTC_DAY_HIGH_INDEX] & RTC_DAY_HIGH_HALT != 0 {
            return;
        }

        self.timer_clock_accumulator = self.timer_clock_accumulator.saturating_add(clocks);
        while self.timer_clock_accumulator >= RTC_SECOND_CLOCKS {
            self.timer_clock_accumulator -= RTC_SECOND_CLOCKS;
            self.increment_rtc_second();
        }
    }

    fn increment_rtc_second(&mut self) {
        self.timer_data[RTC_SECONDS_INDEX] = self.timer_data[RTC_SECONDS_INDEX].saturating_add(1);
        if self.timer_data[RTC_SECONDS_INDEX] < 60 {
            return;
        }

        self.timer_data[RTC_SECONDS_INDEX] = 0;
        self.timer_data[RTC_MINUTES_INDEX] = self.timer_data[RTC_MINUTES_INDEX].saturating_add(1);
        if self.timer_data[RTC_MINUTES_INDEX] < 60 {
            return;
        }

        self.timer_data[RTC_MINUTES_INDEX] = 0;
        self.timer_data[RTC_HOURS_INDEX] = self.timer_data[RTC_HOURS_INDEX].saturating_add(1);
        if self.timer_data[RTC_HOURS_INDEX] < 24 {
            return;
        }

        self.timer_data[RTC_HOURS_INDEX] = 0;
        self.increment_rtc_day();
    }

    fn increment_rtc_day(&mut self) {
        let day_high = self.timer_data[RTC_DAY_HIGH_INDEX];
        let mut day = u16::from(self.timer_data[RTC_DAY_LOW_INDEX])
            | (u16::from(day_high & RTC_DAY_HIGH_DAY_BIT) << 8);

        if day == 0x01ff {
            day = 0;
            self.timer_data[RTC_DAY_HIGH_INDEX] = (day_high | RTC_DAY_HIGH_CARRY) & 0xc0;
        } else {
            day += 1;
            self.timer_data[RTC_DAY_HIGH_INDEX] = (day_high
                & (RTC_DAY_HIGH_HALT | RTC_DAY_HIGH_CARRY))
                | (((day >> 8) as u8) & RTC_DAY_HIGH_DAY_BIT);
        }
        self.timer_data[RTC_DAY_LOW_INDEX] = day as u8;
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

fn masked_timer_value(timer_index: usize, value: u8) -> u8 {
    match timer_index {
        RTC_SECONDS_INDEX | RTC_MINUTES_INDEX => value & 0x3f,
        RTC_HOURS_INDEX => value & 0x1f,
        RTC_DAY_LOW_INDEX => value,
        RTC_DAY_HIGH_INDEX => {
            value & (RTC_DAY_HIGH_DAY_BIT | RTC_DAY_HIGH_HALT | RTC_DAY_HIGH_CARRY)
        }
        _ => 0xff,
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

    #[test]
    fn large_header_ram_sizes_are_backed_by_sram() {
        let mut rom = vec![0; 0x150];
        rom[RAM_SIZE_INDEX] = 0x04;
        let mut sram = SramState::default();

        sram.reset_for_rom_load(
            &RomProperties {
                mbc: MemoryBankController::Mbc5,
                ..Default::default()
            },
            &rom,
        );

        assert_eq!(sram.size_bytes, 131_072);
        assert_eq!(sram.bank_select_mask, 0x0f);

        rom[RAM_SIZE_INDEX] = 0x05;
        sram.reset_for_rom_load(
            &RomProperties {
                mbc: MemoryBankController::Mbc5,
                ..Default::default()
            },
            &rom,
        );

        assert_eq!(sram.size_bytes, 65_536);
        assert_eq!(sram.bank_select_mask, 0x07);
    }

    #[test]
    fn mbc2_ram_uses_nibbles_and_echoes_bottom_nine_address_bits() {
        let mut sram = SramState {
            size_bytes: 512,
            ..Default::default()
        };

        sram.write_data(0xa000, 0xab);

        assert_eq!(sram.read_data(0xa000), 0xfb);
        assert_eq!(sram.read_data(0xa200), 0xfb);
    }

    #[test]
    fn mbc3_rtc_ticks_latches_halts_and_carries_days() {
        let mut sram = SramState {
            has_timer: true,
            ..Default::default()
        };

        sram.write_timer_data(RTC_SECONDS_INDEX, 58);
        sram.advance_rtc(RTC_SECOND_CLOCKS);
        assert_eq!(sram.read_timer_data(RTC_SECONDS_INDEX), 59);

        sram.latch_timer_data(0);
        sram.latch_timer_data(1);
        sram.advance_rtc(RTC_SECOND_CLOCKS);
        assert_eq!(sram.read_timer_data(RTC_SECONDS_INDEX), 59);
        assert_eq!(sram.timer_data[RTC_SECONDS_INDEX], 0);
        assert_eq!(sram.timer_data[RTC_MINUTES_INDEX], 1);

        sram.latch_timer_data(0);
        sram.latch_timer_data(1);
        assert_eq!(sram.read_timer_data(RTC_SECONDS_INDEX), 0);
        assert_eq!(sram.read_timer_data(RTC_MINUTES_INDEX), 1);

        sram.write_timer_data(RTC_DAY_HIGH_INDEX, RTC_DAY_HIGH_HALT);
        sram.advance_rtc(RTC_SECOND_CLOCKS);
        assert_eq!(sram.timer_data[RTC_SECONDS_INDEX], 0);

        sram.write_timer_data(RTC_DAY_LOW_INDEX, 0xff);
        sram.write_timer_data(RTC_DAY_HIGH_INDEX, RTC_DAY_HIGH_DAY_BIT);
        sram.write_timer_data(RTC_HOURS_INDEX, 23);
        sram.write_timer_data(RTC_MINUTES_INDEX, 59);
        sram.write_timer_data(RTC_SECONDS_INDEX, 59);
        sram.advance_rtc(RTC_SECOND_CLOCKS);

        assert_eq!(sram.timer_data[RTC_DAY_LOW_INDEX], 0);
        assert_eq!(
            sram.timer_data[RTC_DAY_HIGH_INDEX] & RTC_DAY_HIGH_CARRY,
            RTC_DAY_HIGH_CARRY
        );
    }
}
