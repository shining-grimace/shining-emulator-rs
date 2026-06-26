#![allow(dead_code)]

mod audio_unit;
mod bus;
mod cheats;
mod constants;
mod cpu;
mod dma;
mod execution;
mod gpu;
mod input;
mod loader;
mod machine;
mod memory;
mod palettes;
mod rom;
mod runtime;
mod save_state;
mod sgb;
mod sram;
mod systems;
mod video;

#[cfg(test)]
mod accuracy_tests;

use bevy::prelude::*;

use crate::game_boy::emulator::gpu::GpuMode;

pub(crate) use audio_unit::{
    GameBoyAudioBalance, GameBoyAudioChannel, GameBoyAudioCommand, GameBoyAudioEvent,
};
pub(crate) use cheats::{CheatCode, CheatCodeType, parse_cheat_code};
pub(crate) use loader::{
    GameBoyLoadStatus, GameBoyRomLoadRequest, GameBoyRomLoadTaskState, begin_game_boy_rom_load,
    finish_game_boy_rom_load, has_pending_game_boy_rom_load,
};
pub(crate) use machine::GameBoyCore;
pub(crate) use save_state::{apply_save_state, encode_save_state};
pub(crate) use sgb::SgbState;
pub(super) use systems::{
    apply_emulator_control_events, persist_dirty_sram, spawn_game_boy_emulator, sync_joypad_input,
    tick_game_boy_emulator,
};

#[derive(Clone, Copy, Component, Debug, Default)]
pub(crate) struct GameBoyEmulator;

#[derive(Bundle, Default)]
pub(crate) struct GameBoyEmulatorBundle {
    emulator: GameBoyEmulator,
    core: GameBoyCore,
}

impl GameBoyEmulatorBundle {
    pub(crate) fn bootstrap_video_output() -> Self {
        let mut bundle = Self::default();
        bundle.core.runtime.is_running = true;
        bundle.core.cpu_timing.clock_frequency_hz = constants::GB_CLOCK_HZ;
        bundle.core.cpu_timing.system_counter = 0;
        bundle.core.cpu_timing.tima_reload_delay = 0;
        bundle.core.gpu_mode = GpuMode::ScanOam;
        bundle.core.memory_access.oam = false;
        bundle.core.memory_access.vram = true;
        bundle
    }
}
