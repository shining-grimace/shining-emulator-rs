use crate::game_boy::emulator::constants::{
    IO_PORT_BYTES, OAM_BYTES, ROM_CAPACITY_BYTES, TILE_SET_PIXELS, VRAM_BYTES, WRAM_BYTES,
};

const INITIAL_IO_VALUES: &[(usize, u8)] = &[
    (5, 0x00),
    (6, 0x00),
    (7, 0x00),
    (16, 0x80),
    (17, 0xbf),
    (18, 0xf3),
    (20, 0xbf),
    (22, 0x3f),
    (23, 0x00),
    (25, 0xbf),
    (26, 0x7f),
    (27, 0xff),
    (28, 0x9f),
    (30, 0xbf),
    (32, 0xff),
    (33, 0x00),
    (34, 0x00),
    (35, 0xbf),
    (36, 0x77),
    (37, 0xf3),
    (64, 0x91),
    (66, 0x00),
    (67, 0x00),
    (69, 0x00),
    (71, 0xfc),
    (72, 0xff),
    (73, 0xff),
    (74, 0x00),
    (75, 0x00),
    (85, 0xff),
    (255, 0x00),
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
        self.vram.fill(0);
        self.io_ports.fill(0);
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
}
