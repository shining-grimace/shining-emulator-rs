use crate::game_boy::emulator::constants::{
    IO_PORT_BYTES, OAM_BYTES, ROM_CAPACITY_BYTES, TILE_SET_PIXELS, VRAM_BYTES, WRAM_BYTES,
};

const INITIAL_IO_VALUES: &[(usize, u8)] = &[
    (0x00, 0xcf),
    (0x01, 0x00),
    (0x02, 0x7e),
    (0x05, 0x00),
    (0x06, 0x00),
    (0x07, 0xf8),
    (0x0f, 0xe1),
    (0x10, 0x80),
    (0x11, 0xbf),
    (0x12, 0xf3),
    (0x13, 0xff),
    (0x14, 0xbf),
    (0x16, 0x3f),
    (0x17, 0x00),
    (0x18, 0xff),
    (0x19, 0xbf),
    (0x1a, 0x7f),
    (0x1b, 0xff),
    (0x1c, 0x9f),
    (0x1d, 0xff),
    (0x1e, 0xbf),
    (0x20, 0xff),
    (0x21, 0x00),
    (0x22, 0x00),
    (0x23, 0xbf),
    (0x24, 0x77),
    (0x25, 0xf3),
    (0x26, 0xf1),
    (0x40, 0x91),
    (0x41, 0x85),
    (0x42, 0x00),
    (0x43, 0x00),
    (0x45, 0x00),
    (0x46, 0xff),
    (0x47, 0xfc),
    (0x48, 0xff),
    (0x49, 0xff),
    (0x4a, 0x00),
    (0x4b, 0x00),
    (0x50, 0x01),
    (0x55, 0xff),
    (0xff, 0x00),
];

#[derive(Debug)]
pub(crate) struct GameBoyMemory {
    pub(crate) rom: Box<[u8]>,
    pub(crate) wram: Box<[u8]>,
    pub(crate) vram: Box<[u8]>,
    pub(crate) io_ports: Box<[u8]>,
    pub(crate) oam: [u8; OAM_BYTES],
    pub(crate) tile_set: Box<[u32]>,
}

impl Default for GameBoyMemory {
    fn default() -> Self {
        Self {
            rom: zeroed_bytes(ROM_CAPACITY_BYTES),
            wram: zeroed_bytes(WRAM_BYTES),
            vram: zeroed_bytes(VRAM_BYTES),
            io_ports: zeroed_bytes(IO_PORT_BYTES),
            oam: [0; OAM_BYTES],
            tile_set: zeroed_u32s(TILE_SET_PIXELS),
        }
    }
}

impl GameBoyMemory {
    pub(crate) fn reset_for_rom_load(&mut self, rom_bytes: &[u8]) -> bool {
        self.rom.fill(0);
        self.wram.fill(0);
        self.vram.fill(0);
        self.io_ports.fill(0);
        self.oam.fill(0);
        self.tile_set.fill(0);

        let copy_len = rom_bytes.len().min(self.rom.len());
        let Some(destination) = self.rom.get_mut(0..copy_len) else {
            return false;
        };
        destination.copy_from_slice(&rom_bytes[..copy_len]);
        true
    }

    pub(crate) fn reset_io_ports_for_rom_load(&mut self) {
        for &(index, value) in INITIAL_IO_VALUES {
            self.write_io_port(index, value);
        }
    }

    pub(crate) fn write_io_port(&mut self, index: usize, value: u8) {
        if let Some(port) = self.io_ports.get_mut(index) {
            *port = value;
        }
    }

    pub(crate) fn write_vram(&mut self, vram_bank_offset: u32, address: u16, value: u8) {
        let relative_address = usize::from(address & 0x1fff);
        let bank_offset = usize::try_from(vram_bank_offset).unwrap_or_default();
        let Some(vram_index) = bank_offset.checked_add(relative_address) else {
            return;
        };
        let Some(slot) = self.vram.get_mut(vram_index) else {
            return;
        };
        *slot = value;

        if relative_address < 0x1800 {
            self.decode_tile_row(bank_offset, relative_address);
        }
    }

    fn decode_tile_row(&mut self, bank_offset: usize, relative_address: usize) {
        let row_address = relative_address & 0x1ffe;
        let Some(byte1) = self.vram.get(bank_offset + row_address).copied() else {
            return;
        };
        let Some(byte2) = self.vram.get(bank_offset + row_address + 1).copied() else {
            return;
        };

        let mut output_address = row_address * 4;
        if bank_offset != 0 {
            output_address += 24_576;
        }
        let Some(output) = self
            .tile_set
            .get_mut(output_address..output_address.saturating_add(8))
        else {
            return;
        };

        for (pixel, slot) in output.iter_mut().enumerate() {
            let shift = 7_u8.saturating_sub(pixel as u8);
            let low = (byte1 >> shift) & 0x01;
            let high = ((byte2 >> shift) & 0x01) << 1;
            *slot = u32::from(high | low);
        }
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
    fn allocated_memory_matches_the_legacy_gbc_capacity() {
        let memory = GameBoyMemory::default();

        assert_eq!(memory.rom.len(), ROM_CAPACITY_BYTES);
        assert_eq!(memory.wram.len(), WRAM_BYTES);
        assert_eq!(memory.vram.len(), VRAM_BYTES);
        assert_eq!(memory.io_ports.len(), IO_PORT_BYTES);
        assert_eq!(memory.oam.len(), OAM_BYTES);
        assert_eq!(memory.tile_set.len(), TILE_SET_PIXELS);
    }

    #[test]
    fn vram_writes_decode_tile_rows_to_colour_indices() {
        let mut memory = GameBoyMemory::default();

        memory.write_vram(0, 0, 0b1000_0001);
        memory.write_vram(0, 1, 0b0100_0001);

        assert_eq!(&memory.tile_set[0..8], &[1, 2, 0, 0, 0, 0, 0, 3]);
    }
}
