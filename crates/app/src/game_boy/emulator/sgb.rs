use crate::game_boy::emulator::constants::{
    SGB_ATTRIBUTE_FILE_ENTRIES, SGB_ATTRIBUTE_FILES, SGB_CHARACTER_PALETTE_ENTRIES,
    SGB_MONO_PIXELS, SGB_PALETTE_COLORS, SGB_SYSTEM_PALETTE_COLORS, SGB_TRANSFER_VRAM_BYTES,
};
use crate::game_boy::emulator::memory::GameBoyMemory;

const SGBCOM_PAL01: u32 = 0x00;
const SGBCOM_PAL23: u32 = 0x01;
const SGBCOM_PAL03: u32 = 0x02;
const SGBCOM_PAL12: u32 = 0x03;
const SGBCOM_ATTR_BLK: u32 = 0x04;
const SGBCOM_ATTR_LIN: u32 = 0x05;
const SGBCOM_ATTR_DIV: u32 = 0x06;
const SGBCOM_ATTR_CHR: u32 = 0x07;
const SGBCOM_PAL_SET: u32 = 0x0a;
const SGBCOM_PAL_TRN: u32 = 0x0b;
const SGBCOM_MLT_REQ: u32 = 0x11;
const SGBCOM_ATTR_TRN: u32 = 0x15;
const SGBCOM_ATTR_SET: u32 = 0x16;
const SGBCOM_MASK_EN: u32 = 0x17;

const DEFAULT_SGB_PALETTE: [u32; 4] = [0xffff_ffff, 0xff88_b0b0, 0xff50_7878, 0xff00_0000];

#[derive(Debug)]
pub(crate) struct SgbState {
    pub(crate) reading_command: bool,
    pub(crate) awaiting_stop_bit: bool,
    pub(crate) stop_bit_received: bool,
    pub(crate) command_bytes: [[u32; 16]; 7],
    pub(crate) command_bits: [u8; 8],
    pub(crate) command: u32,
    pub(crate) read_command_bits: i32,
    pub(crate) read_command_bytes: i32,
    pub(crate) freeze_screen: bool,
    pub(crate) freeze_mode: u32,
    pub(crate) mult_enabled: bool,
    pub(crate) player_count: u32,
    pub(crate) packets_sent: u32,
    pub(crate) packets_to_send: u32,
    pub(crate) completed_commands: u32,
    pub(crate) last_command: u32,
    pub(crate) packet_errors: u32,
    pub(crate) read_joypad_id: u32,
    pub(crate) mono_data: Box<[u32]>,
    pub(crate) transfer_vram: Box<[u8]>,
    pub(crate) palettes: Box<[u32]>,
    pub(crate) system_palettes: Box<[u32]>,
    pub(crate) character_palettes: Box<[u32]>,
    pub(crate) attribute_files: Box<[u32]>,
}

impl Default for SgbState {
    fn default() -> Self {
        let mut state = Self {
            reading_command: false,
            awaiting_stop_bit: false,
            stop_bit_received: false,
            command_bytes: [[0; 16]; 7],
            command_bits: [0; 8],
            command: 0,
            read_command_bits: 0,
            read_command_bytes: 0,
            freeze_screen: false,
            freeze_mode: 0,
            mult_enabled: false,
            player_count: 0,
            packets_sent: 0,
            packets_to_send: 0,
            completed_commands: 0,
            last_command: 0,
            packet_errors: 0,
            read_joypad_id: 0,
            mono_data: zeroed_u32s(SGB_MONO_PIXELS),
            transfer_vram: zeroed_bytes(SGB_TRANSFER_VRAM_BYTES),
            palettes: zeroed_u32s(SGB_PALETTE_COLORS),
            system_palettes: zeroed_u32s(SGB_SYSTEM_PALETTE_COLORS),
            character_palettes: zeroed_u32s(SGB_CHARACTER_PALETTE_ENTRIES),
            attribute_files: zeroed_u32s(SGB_ATTRIBUTE_FILE_ENTRIES),
        };
        state.reset_display_palettes();
        state
    }
}

impl SgbState {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.reading_command = false;
        self.awaiting_stop_bit = false;
        self.stop_bit_received = false;
        self.command_bytes = [[0; 16]; 7];
        self.command_bits = [0; 8];
        self.command = 0;
        self.read_command_bits = 0;
        self.read_command_bytes = 0;
        self.freeze_screen = false;
        self.freeze_mode = 0;
        self.mult_enabled = false;
        self.player_count = 0;
        self.packets_sent = 0;
        self.packets_to_send = 0;
        self.completed_commands = 0;
        self.last_command = 0;
        self.packet_errors = 0;
        self.read_joypad_id = 0;
        self.mono_data.fill(0);
        self.transfer_vram.fill(0);
        self.reset_display_palettes();
        self.system_palettes.fill(0);
        self.character_palettes.fill(0);
        self.attribute_files.fill(0);
    }

    fn reset_display_palettes(&mut self) {
        for palette in self.palettes.chunks_exact_mut(DEFAULT_SGB_PALETTE.len()) {
            palette.copy_from_slice(&DEFAULT_SGB_PALETTE);
        }
    }

    pub(crate) fn begin_command(&mut self) {
        if self.reading_command {
            return;
        }
        let continuing_command = self.packets_sent > 0 && self.packets_sent < self.packets_to_send;
        if !continuing_command {
            self.command_bytes = [[0; 16]; 7];
            self.packets_sent = 0;
            self.packets_to_send = 1;
            self.command = 0;
        }
        self.reading_command = true;
        self.awaiting_stop_bit = false;
        self.stop_bit_received = false;
        self.read_command_bits = 0;
        self.read_command_bytes = 0;
    }

    pub(crate) fn receive_command_bit(&mut self, bit: u8) {
        if !self.reading_command {
            return;
        }

        if self.awaiting_stop_bit {
            if bit == 0 {
                self.stop_bit_received = true;
            } else {
                self.abort_command();
            }
            return;
        }

        if self.read_command_bytes >= 16 {
            self.abort_command();
            return;
        }

        let bit_index = usize::try_from(self.read_command_bits).unwrap_or_default();
        if let Some(slot) = self.command_bits.get_mut(bit_index) {
            *slot = bit & 0x01;
        }
        self.read_command_bits += 1;
        if self.read_command_bits >= 8 {
            self.check_byte();
        }
    }

    pub(crate) fn finish_packet_if_ready(&mut self, memory: &GameBoyMemory) {
        if !self.reading_command
            || !self.stop_bit_received
            || self.read_command_bits != 0
            || self.read_command_bytes < 16
        {
            return;
        }

        self.finish_packet(memory);
    }

    fn check_byte(&mut self) {
        self.read_command_bits = 0;
        let mut byte = 0_u32;
        for bit in 0..8 {
            byte |= u32::from(self.command_bits[bit]) << bit;
        }
        let packet = usize::try_from(self.packets_sent)
            .unwrap_or_default()
            .min(6);
        let byte_index = usize::try_from(self.read_command_bytes)
            .unwrap_or_default()
            .min(15);
        self.command_bytes[packet][byte_index] = byte;
        self.read_command_bytes += 1;
        if self.read_command_bytes >= 16 {
            self.awaiting_stop_bit = true;
        }
        if self.read_command_bytes == 1 && self.packets_sent == 0 {
            self.packets_to_send = (self.command_bytes[0][0] & 0x07).max(1);
            self.command = (self.command_bytes[0][0] >> 3) & 0x1f;
        }
    }

    fn abort_command(&mut self) {
        self.packet_errors = self.packet_errors.saturating_add(1);
        self.reading_command = false;
        self.awaiting_stop_bit = false;
        self.stop_bit_received = false;
        self.read_command_bits = 0;
        self.read_command_bytes = 0;
        self.packets_sent = 0;
        self.packets_to_send = 1;
    }

    fn finish_packet(&mut self, memory: &GameBoyMemory) {
        self.packets_sent = self.packets_sent.saturating_add(1);
        self.read_command_bytes = 0;
        self.read_command_bits = 0;
        self.awaiting_stop_bit = false;
        self.stop_bit_received = false;
        self.reading_command = false;
        if self.packets_sent >= self.packets_to_send {
            self.check_packets(memory);
        }
    }

    fn check_packets(&mut self, memory: &GameBoyMemory) {
        self.completed_commands = self.completed_commands.saturating_add(1);
        self.last_command = self.command;
        match self.command {
            SGBCOM_PAL01 => {
                self.set_common_colour();
                self.set_palette_range(0, 1, 3);
                self.set_palette_range(1, 5, 7);
            }
            SGBCOM_PAL23 => {
                self.set_common_colour();
                self.set_palette_range(0, 9, 11);
                self.set_palette_range(1, 13, 15);
            }
            SGBCOM_PAL03 => {
                self.set_common_colour();
                self.set_palette_range(0, 1, 3);
                self.set_palette_range(1, 13, 15);
            }
            SGBCOM_PAL12 => {
                self.set_common_colour();
                self.set_palette_range(0, 5, 7);
                self.set_palette_range(1, 9, 11);
            }
            SGBCOM_ATTR_BLK => self.apply_attr_blk(),
            SGBCOM_ATTR_LIN => self.apply_attr_lin(),
            SGBCOM_ATTR_DIV => self.apply_attr_div(),
            SGBCOM_ATTR_CHR => self.apply_attr_chr(),
            SGBCOM_PAL_SET => self.apply_pal_set(),
            SGBCOM_PAL_TRN => self.apply_pal_trn(memory),
            SGBCOM_MLT_REQ => {
                self.mult_enabled = self.command_bytes[0][1] & 0x01 != 0;
                self.player_count = self.command_bytes[0][1] + 1;
                self.read_joypad_id = 0x0f;
            }
            SGBCOM_ATTR_TRN => self.apply_attr_trn(memory),
            SGBCOM_ATTR_SET => self.apply_attr_set(),
            SGBCOM_MASK_EN => {
                self.freeze_mode = self.command_bytes[0][1];
                self.freeze_screen = (1..4).contains(&self.freeze_mode);
            }
            _ => {}
        }
    }

    fn set_common_colour(&mut self) {
        let colour = remap_555_8888(self.command_bytes[0][1], self.command_bytes[0][2]);
        for index in [0, 4, 8, 12] {
            self.palettes[index] = colour;
        }
    }

    fn set_palette_range(
        &mut self,
        source_group: usize,
        destination_start: usize,
        destination_end: usize,
    ) {
        let mut source = 3 + source_group * 6;
        for destination in destination_start..=destination_end {
            self.palettes[destination] = remap_555_8888(
                self.command_bytes[0][source],
                self.command_bytes[0][source + 1],
            );
            source += 2;
        }
    }

    fn apply_attr_blk(&mut self) {
        let data_groups = self.command_bytes[0][1].min(0x12);
        let mut packet = 0_usize;
        let mut byte = 2_usize;
        for _ in 0..data_groups {
            let control = self.packet_byte(packet, byte) & 0x07;
            byte += 1;
            let palette_codes = self.packet_byte(packet, byte) & 0x3f;
            byte += 1;
            advance_packet_cursor(&mut packet, &mut byte);
            let x_left = self.packet_byte(packet, byte).min(19);
            byte += 1;
            let y_top = self.packet_byte(packet, byte).min(17);
            byte += 1;
            advance_packet_cursor(&mut packet, &mut byte);
            let x_right = self.packet_byte(packet, byte).min(19);
            byte += 1;
            let y_bottom = self.packet_byte(packet, byte).min(17);
            byte += 1;
            advance_packet_cursor(&mut packet, &mut byte);
            if x_left > x_right || y_top > y_bottom {
                break;
            }

            if control > 3 {
                let palette = (palette_codes & 0x30) >> 4;
                for y in 0..18 {
                    for x in 0..20 {
                        if x < x_left || x > x_right || y < y_top || y > y_bottom {
                            self.character_palettes[y as usize * 20 + x as usize] = palette;
                        }
                    }
                }
            }
            if control & 0x01 != 0 {
                let palette = palette_codes & 0x03;
                for y in y_top + 1..y_bottom {
                    for x in x_left + 1..x_right {
                        self.character_palettes[y as usize * 20 + x as usize] = palette;
                    }
                }
            }
            if control > 0 && control != 5 {
                let palette = if control == 1 {
                    palette_codes & 0x03
                } else if control == 4 {
                    (palette_codes & 0x30) >> 4
                } else {
                    (palette_codes & 0x0c) >> 2
                };
                for y in y_top..=y_bottom {
                    self.character_palettes[y as usize * 20 + x_left as usize] = palette;
                    self.character_palettes[y as usize * 20 + x_right as usize] = palette;
                }
                for x in x_left..=x_right {
                    self.character_palettes[y_top as usize * 20 + x as usize] = palette;
                    self.character_palettes[y_bottom as usize * 20 + x as usize] = palette;
                }
            }
        }
    }

    fn apply_attr_lin(&mut self) {
        let data_sets = self.command_bytes[0][1].min(0x6e);
        let mut packet = 0_usize;
        let mut byte = 2_usize;
        for _ in 0..data_sets {
            let value = self.packet_byte(packet, byte);
            let line = value & 0x1f;
            let palette = (value >> 5) & 0x03;
            if value & 0x80 != 0 {
                self.fill_character_rows(line, line, palette);
            } else {
                self.fill_character_columns(line, line, palette);
            }
            byte += 1;
            advance_packet_cursor(&mut packet, &mut byte);
        }
    }

    fn apply_attr_div(&mut self) {
        if self.command_bytes[0][1] & 0x40 != 0 {
            let h = self.command_bytes[0][2].min(17);
            self.fill_character_rows(0, h, (self.command_bytes[0][1] & 0x0c) >> 2);
            self.fill_character_rows(h, h, (self.command_bytes[0][1] & 0x30) >> 4);
            self.fill_character_rows(h + 1, 17, self.command_bytes[0][1] & 0x03);
        } else {
            let v = self.command_bytes[0][2].min(19);
            self.fill_character_columns(0, v, (self.command_bytes[0][1] & 0x0c) >> 2);
            self.fill_character_columns(v, v, (self.command_bytes[0][1] & 0x30) >> 4);
            self.fill_character_columns(v + 1, 19, self.command_bytes[0][1] & 0x03);
        }
    }

    fn apply_attr_chr(&mut self) {
        let start_x = self.command_bytes[0][1];
        let start_y = self.command_bytes[0][2];
        if start_x > 19 || start_y > 17 {
            return;
        }
        let data_sets = (self.command_bytes[0][3] | (self.command_bytes[0][4] << 8)) as usize;
        let data_sets = data_sets.min(360);
        let vertical = self.command_bytes[0][5] & 0x01 != 0;
        let mut x = start_x;
        let mut y = start_y;
        let mut packet = 0_usize;
        let mut byte = 6_usize;
        let mut entry = 0_u8;
        for _ in 0..data_sets {
            let value = self.packet_byte(packet, byte);
            let palette = attr_chr_palette(value, entry);
            self.character_palettes[y as usize * 20 + x as usize] = palette;
            if vertical {
                y += 1;
                if y >= 18 {
                    y = 0;
                    x += 1;
                    if x >= 20 {
                        x = 0;
                    }
                }
            } else {
                x += 1;
                if x >= 20 {
                    x = 0;
                    y += 1;
                    if y >= 18 {
                        y = 0;
                    }
                }
            }
            entry += 1;
            if entry >= 4 {
                entry = 0;
                byte += 1;
                advance_packet_cursor(&mut packet, &mut byte);
            }
        }
    }

    fn apply_pal_set(&mut self) {
        for palette in 0..4 {
            let source_palette = self.command_bytes[0][palette * 2 + 1]
                | ((self.command_bytes[0][palette * 2 + 2] & 0x01) << 8);
            for colour in 0..4 {
                let dst = palette * 4 + colour;
                let src = source_palette as usize * 4 + colour;
                if let Some(&value) = self.system_palettes.get(src) {
                    self.palettes[dst] = value;
                }
            }
        }
        let flags = self.command_bytes[0][9];
        if flags & 0x80 != 0 {
            self.apply_attribute_file(flags & 0x3f);
        }
        if flags & 0x40 != 0 {
            self.freeze_mode = 0;
            self.freeze_screen = false;
        }
    }

    fn apply_pal_trn(&mut self, memory: &GameBoyMemory) {
        if memory.io_ports.get(0x40).copied().unwrap_or(0) & 0x80 == 0 {
            return;
        }
        self.map_vram_for_transfer(memory);
        let mut source = 0;
        for palette in 0..512 {
            for colour in 0..4 {
                let Some(&lo) = self.transfer_vram.get(source) else {
                    return;
                };
                let Some(&hi) = self.transfer_vram.get(source + 1) else {
                    return;
                };
                self.system_palettes[palette * 4 + colour] =
                    remap_555_8888(u32::from(lo), u32::from(hi));
                source += 2;
            }
        }
    }

    fn apply_attr_trn(&mut self, memory: &GameBoyMemory) {
        if memory.io_ports.get(0x40).copied().unwrap_or(0) & 0x80 == 0 {
            return;
        }
        self.map_vram_for_transfer(memory);
        for file in 0..SGB_ATTRIBUTE_FILES {
            for y in 0..18 {
                for byte_column in 0..5 {
                    let source = file * 90 + y * 5 + byte_column;
                    let value = self.transfer_vram.get(source).copied().unwrap_or(0);
                    for entry in 0..4 {
                        let x = byte_column * 4 + entry;
                        let shift = 6 - entry * 2;
                        self.attribute_files[file * SGB_CHARACTER_PALETTE_ENTRIES + y * 20 + x] =
                            u32::from((value >> shift) & 0x03);
                    }
                }
            }
        }
    }

    fn apply_attr_set(&mut self) {
        let control = self.command_bytes[0][1];
        self.apply_attribute_file(control & 0x3f);
        if control & 0x40 != 0 {
            self.freeze_mode = 0;
            self.freeze_screen = false;
        }
    }

    fn apply_attribute_file(&mut self, file: u32) {
        let file = file as usize;
        if file >= SGB_ATTRIBUTE_FILES {
            return;
        }
        let start = file * SGB_CHARACTER_PALETTE_ENTRIES;
        let end = start + SGB_CHARACTER_PALETTE_ENTRIES;
        if let Some(source) = self.attribute_files.get(start..end) {
            self.character_palettes.copy_from_slice(source);
        }
    }

    fn map_vram_for_transfer(&mut self, memory: &GameBoyMemory) {
        let lcd_control = memory.io_ports.get(0x40).copied().unwrap_or(0);
        let map_start = if lcd_control & 0x08 != 0 {
            0x1c00
        } else {
            0x1800
        };
        let (chars_start, char_code_inverter) = if lcd_control & 0x10 != 0 {
            (0x0000, 0x0000)
        } else {
            (0x0800, 0x0080)
        };

        for chr_y in 0..18 {
            for chr_x in 0..20 {
                let map_index = chr_y * 32 + chr_x;
                let tile = memory.vram.get(map_start + map_index).copied().unwrap_or(0);
                let zero_based_tile = usize::from(tile ^ char_code_inverter);
                let chars_data_start = chars_start + zero_based_tile * 16;
                let destination = 16 * (chr_y * 20 + chr_x);
                for byte in 0..16 {
                    if let (Some(dst), Some(&src)) = (
                        self.transfer_vram.get_mut(destination + byte),
                        memory.vram.get(chars_data_start + byte),
                    ) {
                        *dst = src;
                    }
                }
            }
        }
    }

    fn fill_character_rows(&mut self, start: u32, end: u32, palette: u32) {
        if start > end || start > 17 {
            return;
        }
        for y in start..=end.min(17) {
            for x in 0..20 {
                self.character_palettes[y as usize * 20 + x] = palette;
            }
        }
    }

    fn fill_character_columns(&mut self, start: u32, end: u32, palette: u32) {
        if start > end || start > 19 {
            return;
        }
        for y in 0..18 {
            for x in start..=end.min(19) {
                self.character_palettes[y * 20 + x as usize] = palette;
            }
        }
    }

    fn packet_byte(&self, packet: usize, byte: usize) -> u32 {
        self.command_bytes
            .get(packet)
            .and_then(|packet| packet.get(byte))
            .copied()
            .unwrap_or(0)
    }
}

fn attr_chr_palette(value: u32, entry: u8) -> u32 {
    match entry {
        0 => (value & 0xc0) >> 6,
        1 => (value & 0x30) >> 4,
        2 => (value & 0x0c) >> 2,
        _ => value & 0x03,
    }
}

fn advance_packet_cursor(packet: &mut usize, byte: &mut usize) {
    if *byte >= 16 {
        *byte = 0;
        *packet = (*packet + 1).min(6);
    }
}

fn remap_555_8888(lo: u32, hi: u32) -> u32 {
    let value = lo | (hi << 8);
    let red = (value & 0x1f) * 255 / 31;
    let green = ((value >> 5) & 0x1f) * 255 / 31;
    let blue = ((value >> 10) & 0x1f) * 255 / 31;
    0xff00_0000 | (red << 16) | (green << 8) | blue
}

fn zeroed_bytes(len: usize) -> Box<[u8]> {
    vec![0; len].into_boxed_slice()
}

fn zeroed_u32s(len: usize) -> Box<[u32]> {
    vec![0; len].into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocated_sgb_buffers_match_the_legacy_module() {
        let state = SgbState::default();

        assert_eq!(state.mono_data.len(), SGB_MONO_PIXELS);
        assert_eq!(state.transfer_vram.len(), SGB_TRANSFER_VRAM_BYTES);
        assert_eq!(state.palettes.len(), SGB_PALETTE_COLORS);
        assert_eq!(state.system_palettes.len(), SGB_SYSTEM_PALETTE_COLORS);
        assert_eq!(
            state.character_palettes.len(),
            SGB_CHARACTER_PALETTE_ENTRIES
        );
        assert_eq!(state.attribute_files.len(), SGB_ATTRIBUTE_FILE_ENTRIES);
    }

    #[test]
    fn default_display_palettes_are_visible_before_sgb_commands() {
        let mut state = SgbState::default();

        assert_eq!(&state.palettes[0..4], DEFAULT_SGB_PALETTE);
        assert_eq!(&state.palettes[4..8], DEFAULT_SGB_PALETTE);

        state.palettes.fill(0);
        state.reset_for_rom_load();

        assert_eq!(&state.palettes[0..4], DEFAULT_SGB_PALETTE);
        assert_eq!(&state.palettes[12..16], DEFAULT_SGB_PALETTE);
    }

    #[test]
    fn attr_chr_counts_individual_character_datasets() {
        let mut state = SgbState::default();
        state.character_palettes.fill(9);
        state.command_bytes[0][1] = 18;
        state.command_bytes[0][2] = 0;
        state.command_bytes[0][3] = 4;
        state.command_bytes[0][5] = 0;
        state.command_bytes[0][6] = 0b00_01_10_11;

        state.apply_attr_chr();

        assert_eq!(state.character_palettes[18], 0);
        assert_eq!(state.character_palettes[19], 1);
        assert_eq!(state.character_palettes[20], 2);
        assert_eq!(state.character_palettes[21], 3);
        assert_eq!(state.character_palettes[22], 9);
    }

    #[test]
    fn attr_chr_wraps_vertical_writes_to_next_column() {
        let mut state = SgbState::default();
        state.character_palettes.fill(9);
        state.command_bytes[0][1] = 18;
        state.command_bytes[0][2] = 16;
        state.command_bytes[0][3] = 4;
        state.command_bytes[0][5] = 1;
        state.command_bytes[0][6] = 0b00_01_10_11;

        state.apply_attr_chr();

        assert_eq!(state.character_palettes[16 * 20 + 18], 0);
        assert_eq!(state.character_palettes[17 * 20 + 18], 1);
        assert_eq!(state.character_palettes[19], 2);
        assert_eq!(state.character_palettes[20 + 19], 3);
    }

    #[test]
    fn attr_chr_wraps_past_screen_end_to_start() {
        let mut state = SgbState::default();
        state.character_palettes.fill(9);
        state.command_bytes[0][1] = 19;
        state.command_bytes[0][2] = 17;
        state.command_bytes[0][3] = 2;
        state.command_bytes[0][5] = 0;
        state.command_bytes[0][6] = 0b01_10_00_00;

        state.apply_attr_chr();

        assert_eq!(state.character_palettes[17 * 20 + 19], 1);
        assert_eq!(state.character_palettes[0], 2);
    }

    #[test]
    fn attr_chr_default_clamps_out_of_range_count_to_documented_grid_size() {
        let mut state = SgbState::default();
        state.character_palettes.fill(9);
        state.command_bytes[0][3] = 0x69;
        state.command_bytes[0][4] = 0x01;
        state.command_bytes[0][6] = 0xff;

        state.apply_attr_chr();

        assert_eq!(state.character_palettes[0], 3);
    }

    #[test]
    fn attr_lin_sets_vertical_and_horizontal_lines() {
        let mut state = SgbState::default();
        state.command_bytes[0][1] = 2;
        state.command_bytes[0][2] = (2 << 5) | 3;
        state.command_bytes[0][3] = 0x80 | (1 << 5) | 4;

        state.apply_attr_lin();

        for y in 0..18 {
            if y != 4 {
                assert_eq!(state.character_palettes[y * 20 + 3], 2);
            }
        }
        for x in 0..20 {
            assert_eq!(state.character_palettes[4 * 20 + x], 1);
        }
    }

    #[test]
    fn attr_trn_decodes_transferred_attribute_files() {
        let mut state = SgbState::default();
        let mut memory = GameBoyMemory::default();
        memory.io_ports[0x40] = 0x80 | 0x10;
        for index in 0..256 {
            let y = index / 20;
            let x = index % 20;
            memory.vram[0x1800 + y * 32 + x] = index as u8;
        }
        memory.vram[0] = 0b00_01_10_11;
        memory.vram[4049] = 0b11_10_01_00;

        state.apply_attr_trn(&memory);

        assert_eq!(&state.attribute_files[0..4], &[0, 1, 2, 3]);
        let last_row = 44 * SGB_CHARACTER_PALETTE_ENTRIES + 17 * 20;
        assert_eq!(
            &state.attribute_files[last_row + 16..last_row + 20],
            &[3, 2, 1, 0]
        );
    }

    #[test]
    fn attr_set_applies_attribute_file_and_can_cancel_mask() {
        let mut state = SgbState::default();
        let file = 2;
        let start = file * SGB_CHARACTER_PALETTE_ENTRIES;
        for (index, palette) in state.attribute_files[start..start + SGB_CHARACTER_PALETTE_ENTRIES]
            .iter_mut()
            .enumerate()
        {
            *palette = (index % 4) as u32;
        }
        state.freeze_screen = true;
        state.freeze_mode = 1;
        state.command_bytes[0][1] = file as u32 | 0x40;

        state.apply_attr_set();

        assert_eq!(
            &state.character_palettes[0..8],
            &state.attribute_files[start..start + 8]
        );
        assert!(!state.freeze_screen);
        assert_eq!(state.freeze_mode, 0);
    }

    #[test]
    fn pal_set_applies_requested_attribute_file() {
        let mut state = SgbState::default();
        let file = 3;
        let start = file * SGB_CHARACTER_PALETTE_ENTRIES;
        for (index, palette) in state.attribute_files[start..start + SGB_CHARACTER_PALETTE_ENTRIES]
            .iter_mut()
            .enumerate()
        {
            *palette = (3 - index % 4) as u32;
        }
        state.command_bytes[0][9] = 0x80 | file as u32;

        state.apply_pal_set();

        assert_eq!(
            &state.character_palettes[0..8],
            &state.attribute_files[start..start + 8]
        );
    }
}
