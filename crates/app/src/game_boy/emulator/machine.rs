use bevy::prelude::*;

use crate::game_boy::emulator::audio_unit::AudioUnitState;
use crate::game_boy::emulator::cheats::CheatTable;
use crate::game_boy::emulator::constants::{GB_CLOCK_HZ, SGB_CLOCK_HZ};
use crate::game_boy::emulator::cpu::{CpuMode, CpuRegisters, CpuTiming, SerialState};
use crate::game_boy::emulator::dma::DmaState;
use crate::game_boy::emulator::gpu::{GpuMode, GpuTiming, LineRenderer, MemoryAccess};
use crate::game_boy::emulator::memory::GameBoyMemory;
use crate::game_boy::emulator::palettes::{CgbPaletteRegisters, PaletteState};
use crate::game_boy::emulator::rom::{RomProperties, RomState};
use crate::game_boy::emulator::runtime::RuntimeControl;
use crate::game_boy::emulator::sgb::SgbState;
use crate::game_boy::emulator::sram::SramState;
use crate::game_boy::emulator::video::VideoFrameAssembler;

const SC_IO_INDEX: usize = 0x02;
const DMA_IO_INDEX: usize = 0x46;
const KEY1_IO_INDEX: usize = 0x4d;
const VBK_IO_INDEX: usize = 0x4f;
const HDMA1_IO_INDEX: usize = 0x51;
const HDMA2_IO_INDEX: usize = 0x52;
const HDMA3_IO_INDEX: usize = 0x53;
const HDMA4_IO_INDEX: usize = 0x54;
const HDMA5_IO_INDEX: usize = 0x55;
const RP_IO_INDEX: usize = 0x56;
const SVBK_IO_INDEX: usize = 0x70;
const NR52_IO_INDEX: usize = 0x26;
const STAT_IO_INDEX: usize = 0x41;
const LY_IO_INDEX: usize = 0x44;

#[derive(Component, Debug)]
pub(crate) struct GameBoyCore {
    pub(crate) runtime: RuntimeControl,
    pub(crate) cpu_registers: CpuRegisters,
    pub(crate) cpu_timing: CpuTiming,
    pub(crate) cpu_mode: CpuMode,
    pub(crate) serial: SerialState,
    pub(crate) dma: DmaState,
    pub(crate) gpu_timing: GpuTiming,
    pub(crate) gpu_mode: GpuMode,
    pub(crate) line_renderer: LineRenderer,
    pub(crate) memory_access: MemoryAccess,
    pub(crate) memory: GameBoyMemory,
    pub(crate) palettes: PaletteState,
    pub(crate) cgb_palette_registers: CgbPaletteRegisters,
    pub(crate) rom: RomState,
    pub(crate) sram: SramState,
    pub(crate) sgb: SgbState,
    pub(crate) audio_unit: AudioUnitState,
    pub(crate) video_frame: VideoFrameAssembler,
    pub(crate) cheats: CheatTable,
}

impl Default for GameBoyCore {
    fn default() -> Self {
        Self {
            runtime: RuntimeControl::default(),
            cpu_registers: CpuRegisters::default(),
            cpu_timing: CpuTiming::default(),
            cpu_mode: CpuMode::default(),
            serial: SerialState::default(),
            dma: DmaState::default(),
            gpu_timing: GpuTiming::default(),
            gpu_mode: GpuMode::default(),
            line_renderer: LineRenderer::default(),
            memory_access: MemoryAccess::default(),
            memory: GameBoyMemory::default(),
            palettes: PaletteState::default(),
            cgb_palette_registers: CgbPaletteRegisters::default(),
            rom: RomState::default(),
            sram: SramState::default(),
            sgb: SgbState::default(),
            audio_unit: AudioUnitState::default(),
            video_frame: VideoFrameAssembler::default(),
            cheats: CheatTable::default(),
        }
    }
}

impl GameBoyCore {
    pub(crate) fn reset_for_rom_load(
        &mut self,
        properties: RomProperties,
        rom_id: String,
        opened_file_name: String,
        rom_bytes: &[u8],
    ) -> bool {
        if !self.memory.reset_for_rom_load(rom_bytes) {
            return false;
        }

        self.rom
            .reset_for_rom_load(properties, rom_id, opened_file_name);
        self.sram.reset_for_rom_load(&properties, rom_bytes);
        self.sgb.reset_for_rom_load();
        self.cgb_palette_registers.reset_for_rom_load();
        self.cpu_registers.reset_for_rom_load();
        self.cpu_timing.reset_for_rom_load();
        self.cpu_mode.reset_for_rom_load();
        self.serial.reset_for_rom_load();
        self.dma.reset_for_rom_load();
        self.gpu_timing.reset_for_rom_load();
        self.gpu_mode.reset_for_rom_load();
        self.memory_access.reset_for_rom_load();
        self.video_frame.reset_for_rom_load();
        self.memory.reset_io_ports_for_rom_load();
        self.palettes.reset_for_rom_load(&self.memory.io_ports);
        self.configure_model_specific_state(&properties);
        self.configure_boot_bypass_ppu_state();
        self.runtime.reset_for_rom_load();

        true
    }

    fn configure_model_specific_state(&mut self, properties: &RomProperties) {
        if properties.cgb_flag {
            self.cpu_timing.clock_frequency_hz = GB_CLOCK_HZ;
            self.cpu_registers.a = 0x11;
            self.cpu_registers.f = 0x80;
            self.cpu_registers.b = 0x00;
            self.cpu_registers.c = 0x00;
            self.cpu_registers.d = 0xff;
            self.cpu_registers.e = 0x56;
            self.cpu_registers.h = 0x00;
            self.cpu_registers.l = 0x0d;
            self.line_renderer = LineRenderer::Cgb;
            self.memory.write_io_port(SC_IO_INDEX, 0x7f);
            self.memory.write_io_port(NR52_IO_INDEX, 0xf1);
            self.memory.write_io_port(DMA_IO_INDEX, 0x00);
            self.memory.write_io_port(KEY1_IO_INDEX, 0x7e);
            self.memory.write_io_port(VBK_IO_INDEX, 0xfe);
            self.memory.write_io_port(HDMA1_IO_INDEX, 0xff);
            self.memory.write_io_port(HDMA2_IO_INDEX, 0xff);
            self.memory.write_io_port(HDMA3_IO_INDEX, 0xff);
            self.memory.write_io_port(HDMA4_IO_INDEX, 0xff);
            self.memory.write_io_port(HDMA5_IO_INDEX, 0xff);
            self.memory.write_io_port(RP_IO_INDEX, 0x3e);
            self.memory.write_io_port(SVBK_IO_INDEX, 0xf8);
            self.cgb_palette_registers.bg_index = 0;
            self.cgb_palette_registers.obj_index = 0;
        } else if properties.sgb_flag {
            self.cpu_timing.clock_frequency_hz = SGB_CLOCK_HZ;
            self.cpu_registers.a = 0x01;
            self.cpu_registers.f = 0x00;
            self.cpu_registers.b = 0x00;
            self.cpu_registers.c = 0x14;
            self.cpu_registers.d = 0x00;
            self.cpu_registers.e = 0x00;
            self.cpu_registers.h = 0xc0;
            self.cpu_registers.l = 0x60;
            self.line_renderer = LineRenderer::Sgb;
            self.memory.write_io_port(NR52_IO_INDEX, 0xf0);
            self.sgb.read_joypad_id = 0x0c;
            self.cgb_palette_registers.bg_index = 0;
            self.cgb_palette_registers.obj_index = 0;
        } else {
            self.cpu_timing.clock_frequency_hz = GB_CLOCK_HZ;
            self.cpu_registers.a = 0x01;
            self.cpu_registers.f = if properties.check_sum == 0 {
                0x80
            } else {
                0xb0
            };
            self.cpu_registers.b = 0x00;
            self.cpu_registers.c = 0x13;
            self.cpu_registers.d = 0x00;
            self.cpu_registers.e = 0xd8;
            self.cpu_registers.h = 0x01;
            self.cpu_registers.l = 0x4d;
            self.line_renderer = LineRenderer::Gb;
            self.memory.write_io_port(NR52_IO_INDEX, 0xf1);
        }
    }

    fn configure_boot_bypass_ppu_state(&mut self) {
        self.gpu_mode = GpuMode::VBlank;
        self.gpu_timing.time_in_mode = 0;
        self.memory_access.oam = true;
        self.memory_access.vram = true;
        self.memory.write_io_port(LY_IO_INDEX, 0x00);
        self.memory.write_io_port(
            STAT_IO_INDEX,
            (self.memory.io_ports[STAT_IO_INDEX] & 0xfc) | 0x01,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_core(properties: RomProperties) -> GameBoyCore {
        let mut core = GameBoyCore::default();
        assert!(core.reset_for_rom_load(properties, String::new(), String::new(), &[]));
        core
    }

    #[test]
    fn dmg_boot_bypass_applies_dmg_handoff_state() {
        let core = reset_core(RomProperties {
            check_sum: 1,
            ..Default::default()
        });

        assert_eq!(core.cpu_registers.pc, 0x0100);
        assert_eq!(core.cpu_registers.sp, 0xfffe);
        assert_eq!(core.cpu_registers.a, 0x01);
        assert_eq!(core.cpu_registers.f, 0xb0);
        assert_eq!(core.cpu_registers.b, 0x00);
        assert_eq!(core.cpu_registers.c, 0x13);
        assert_eq!(core.cpu_registers.d, 0x00);
        assert_eq!(core.cpu_registers.e, 0xd8);
        assert_eq!(core.cpu_registers.h, 0x01);
        assert_eq!(core.cpu_registers.l, 0x4d);
        assert!(!core.cpu_registers.ime);
        assert_eq!(core.line_renderer, LineRenderer::Gb);
        assert_eq!(core.cpu_timing.clock_frequency_hz, GB_CLOCK_HZ);
        assert_eq!(core.memory.io_ports[0x00], 0xcf);
        assert_eq!(core.memory.io_ports[0x02], 0x7e);
        assert_eq!(core.memory.io_ports[0x07], 0xf8);
        assert_eq!(core.memory.io_ports[0x0f], 0xe1);
        assert_eq!(core.memory.io_ports[0x26], 0xf1);
        assert_eq!(core.memory.io_ports[0x40], 0x91);
        assert_eq!(core.memory.io_ports[0x46], 0xff);
        assert_eq!(core.memory.io_ports[0x47], 0xfc);
        assert_eq!(core.memory.io_ports[0x50], 0x01);
        assert_eq!(core.memory.io_ports[0xff], 0x00);
        assert_eq!(core.gpu_mode, GpuMode::VBlank);
        assert!(core.memory_access.oam);
        assert!(core.memory_access.vram);
        assert_eq!(core.memory.io_ports[0x41] & 0x03, 0x01);
        assert_eq!(core.memory.io_ports[0x44], 0x00);
    }

    #[test]
    fn dmg_zero_header_checksum_clears_boot_checksum_flags() {
        let core = reset_core(RomProperties::default());

        assert_eq!(core.cpu_registers.f, 0x80);
    }

    #[test]
    fn sgb_boot_bypass_applies_sgb_handoff_state() {
        let core = reset_core(RomProperties {
            sgb_flag: true,
            ..Default::default()
        });

        assert_eq!(core.cpu_registers.a, 0x01);
        assert_eq!(core.cpu_registers.f, 0x00);
        assert_eq!(core.cpu_registers.b, 0x00);
        assert_eq!(core.cpu_registers.c, 0x14);
        assert_eq!(core.cpu_registers.d, 0x00);
        assert_eq!(core.cpu_registers.e, 0x00);
        assert_eq!(core.cpu_registers.h, 0xc0);
        assert_eq!(core.cpu_registers.l, 0x60);
        assert_eq!(core.line_renderer, LineRenderer::Sgb);
        assert_eq!(core.cpu_timing.clock_frequency_hz, SGB_CLOCK_HZ);
        assert_eq!(core.memory.io_ports[0x26], 0xf0);
        assert_eq!(core.sgb.read_joypad_id, 0x0c);
    }

    #[test]
    fn cgb_boot_bypass_applies_cgb_handoff_state() {
        let core = reset_core(RomProperties {
            cgb_flag: true,
            ..Default::default()
        });

        assert_eq!(core.cpu_registers.a, 0x11);
        assert_eq!(core.cpu_registers.f, 0x80);
        assert_eq!(core.cpu_registers.b, 0x00);
        assert_eq!(core.cpu_registers.c, 0x00);
        assert_eq!(core.cpu_registers.d, 0xff);
        assert_eq!(core.cpu_registers.e, 0x56);
        assert_eq!(core.cpu_registers.h, 0x00);
        assert_eq!(core.cpu_registers.l, 0x0d);
        assert_eq!(core.line_renderer, LineRenderer::Cgb);
        assert_eq!(core.cpu_timing.clock_frequency_hz, GB_CLOCK_HZ);
        assert_eq!(core.memory.io_ports[0x02], 0x7f);
        assert_eq!(core.memory.io_ports[0x46], 0x00);
        assert_eq!(core.memory.io_ports[0x4d], 0x7e);
        assert_eq!(core.memory.io_ports[0x4f], 0xfe);
        assert_eq!(core.memory.io_ports[0x51], 0xff);
        assert_eq!(core.memory.io_ports[0x52], 0xff);
        assert_eq!(core.memory.io_ports[0x53], 0xff);
        assert_eq!(core.memory.io_ports[0x54], 0xff);
        assert_eq!(core.memory.io_ports[0x55], 0xff);
        assert_eq!(core.memory.io_ports[0x56], 0x3e);
        assert_eq!(core.memory.io_ports[0x70], 0xf8);
    }
}
