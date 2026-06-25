pub(super) const GB_CLOCK_HZ: i64 = 4_194_304;
pub(super) const SGB_CLOCK_HZ: i64 = 4_295_454;
pub(super) const GBC_CLOCK_HZ: i64 = 8_400_000;

pub(super) const CLOCK_MULTIPLIERS: [i64; 21] = [
    1, 1, 1, 1, 1, 2, 1, 4, 2, 4, 1, 5, 3, 7, 2, 5, 3, 5, 8, 12, 20,
];
pub(super) const CLOCK_DIVISORS: [i64; 21] = [
    20, 12, 8, 5, 3, 5, 2, 7, 3, 5, 1, 4, 2, 4, 1, 2, 1, 1, 1, 1, 1,
];
pub(super) const DEFAULT_CLOCK_MULTIPLIER_INDEX: i32 = 10;

pub(super) const MIN_CLOCKS_TO_EXECUTE: i32 = 2_000;
pub(super) const MAX_ACCUMULATED_CLOCKS: i32 = 1_000_000;

pub(super) const ROM_CAPACITY_BYTES: usize = 256 * 16_384;
pub(super) const WRAM_BYTES: usize = 8 * 4_096;
pub(super) const VRAM_BYTES: usize = 2 * 8_192;
pub(super) const IO_PORT_BYTES: usize = 256;
pub(super) const OAM_BYTES: usize = 160;
pub(super) const TILE_SET_PIXELS: usize = 2 * 384 * 8 * 8;
pub(super) const SRAM_CAPACITY_BYTES: usize = 16 * 8_192;

pub(super) const SGB_MONO_PIXELS: usize = 160 * 152;
pub(super) const SGB_TRANSFER_VRAM_BYTES: usize = 4_096;
pub(super) const SGB_PALETTE_COLORS: usize = 4 * 4;
pub(super) const SGB_SYSTEM_PALETTE_COLORS: usize = 512 * 4;
pub(super) const SGB_CHARACTER_PALETTE_ENTRIES: usize = 18 * 20;
pub(super) const SGB_ATTRIBUTE_FILES: usize = 45;
pub(super) const SGB_ATTRIBUTE_FILE_ENTRIES: usize =
    SGB_ATTRIBUTE_FILES * SGB_CHARACTER_PALETTE_ENTRIES;
pub(super) const SGB_BORDER_TILES: usize = 256;
pub(super) const SGB_BORDER_TILE_BYTES: usize = 32;
pub(super) const SGB_BORDER_TILE_BYTES_TOTAL: usize = SGB_BORDER_TILES * SGB_BORDER_TILE_BYTES;
pub(super) const SGB_BORDER_TILE_MAP_WIDTH: usize = 32;
pub(super) const SGB_BORDER_TILE_MAP_HEIGHT: usize = 28;
pub(super) const SGB_BORDER_TILE_MAP_ENTRIES: usize =
    SGB_BORDER_TILE_MAP_WIDTH * SGB_BORDER_TILE_MAP_HEIGHT;
pub(super) const SGB_BORDER_PALETTES: usize = 3;
pub(super) const SGB_BORDER_COLORS_PER_PALETTE: usize = 16;
pub(super) const SGB_BORDER_PALETTE_COLORS: usize =
    SGB_BORDER_PALETTES * SGB_BORDER_COLORS_PER_PALETTE;

pub(super) const AUDIO_BUFFER_FRAMES: usize = 12_000;
pub(super) const AUDIO_WAVEFORM_SAMPLES: usize = 32;

pub(super) const ROM_BANK_SIZE: u32 = 0x4000;
pub(super) const INITIAL_ROM_BANK_OFFSET: u32 = ROM_BANK_SIZE;
pub(super) const INITIAL_WRAM_BANK_OFFSET: u32 = 0x1000;
pub(super) const INITIAL_VRAM_BANK_OFFSET: u32 = 0x0000;
