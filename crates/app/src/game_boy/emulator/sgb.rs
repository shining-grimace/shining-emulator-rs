use crate::game_boy::emulator::constants::{
    SGB_CHARACTER_PALETTE_ENTRIES, SGB_MONO_PIXELS, SGB_PALETTE_COLORS, SGB_SYSTEM_PALETTE_COLORS,
    SGB_TRANSFER_VRAM_BYTES,
};

#[derive(Debug)]
pub(crate) struct SgbState {
    pub(crate) reading_command: bool,
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
    pub(crate) read_joypad_id: u32,
    pub(crate) mono_data: Box<[u32]>,
    pub(crate) transfer_vram: Box<[u8]>,
    pub(crate) palettes: Box<[u32]>,
    pub(crate) system_palettes: Box<[u32]>,
    pub(crate) character_palettes: Box<[u32]>,
}

impl Default for SgbState {
    fn default() -> Self {
        Self {
            reading_command: false,
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
            read_joypad_id: 0,
            mono_data: zeroed_u32s(SGB_MONO_PIXELS),
            transfer_vram: zeroed_bytes(SGB_TRANSFER_VRAM_BYTES),
            palettes: zeroed_u32s(SGB_PALETTE_COLORS),
            system_palettes: zeroed_u32s(SGB_SYSTEM_PALETTE_COLORS),
            character_palettes: zeroed_u32s(SGB_CHARACTER_PALETTE_ENTRIES),
        }
    }
}

impl SgbState {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.reading_command = false;
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
        self.read_joypad_id = 0;
        self.mono_data.fill(0);
        self.transfer_vram.fill(0);
        self.palettes.fill(0);
        self.system_palettes.fill(0);
        self.character_palettes.fill(0);
    }
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
    }
}
