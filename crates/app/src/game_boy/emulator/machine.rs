use bevy::prelude::*;

use crate::game_boy::emulator::audio_unit::AudioUnitState;
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
        self.runtime.reset_for_rom_load();

        true
    }

    fn configure_model_specific_state(&mut self, properties: &RomProperties) {
        if properties.cgb_flag {
            self.cpu_timing.clock_frequency_hz = GB_CLOCK_HZ;
            self.cpu_registers.a = 0x11;
            self.line_renderer = LineRenderer::Cgb;
            self.memory.write_io_port(38, 0xf1);
            self.cgb_palette_registers.bg_index = 0;
            self.cgb_palette_registers.obj_index = 0;
        } else if properties.sgb_flag {
            self.cpu_timing.clock_frequency_hz = SGB_CLOCK_HZ;
            self.cpu_registers.a = 0x01;
            self.line_renderer = LineRenderer::Sgb;
            self.memory.write_io_port(38, 0xf0);
            self.sgb.read_joypad_id = 0x0c;
            self.cgb_palette_registers.bg_index = 0;
            self.cgb_palette_registers.obj_index = 0;
        } else {
            self.cpu_timing.clock_frequency_hz = GB_CLOCK_HZ;
            self.cpu_registers.a = 0x01;
            self.line_renderer = LineRenderer::Gb;
            self.memory.write_io_port(38, 0xf1);
        }
    }
}
