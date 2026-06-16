mod emulator;
mod frame_buffer;
mod frame_renderer;

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::game_boy::emulator::{
    GameBoyRomLoadTaskState, apply_emulator_control_events, begin_game_boy_rom_load,
    finish_game_boy_rom_load, has_pending_game_boy_rom_load, persist_dirty_sram,
    spawn_game_boy_emulator, sync_joypad_input, tick_game_boy_emulator,
};
use crate::game_boy::frame_buffer::{GAME_BOY_FRAME_RATE_HZ, GameBoyFrameRing};
use crate::game_boy::frame_renderer::{
    GameBoyFrameTexture, resize_game_boy_frame_display, spawn_game_boy_frame_display,
    update_game_boy_frame_texture,
};

pub struct GameBoyPlugin;

pub(crate) use emulator::{
    GameBoyCore, GameBoyEmulator, GameBoyLoadStatus, GameBoyRomLoadRequest, apply_save_state,
    encode_save_state,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, SystemSet)]
enum GameBoyUpdateSet {
    RomLoad,
    Emulation,
}

impl Plugin for GameBoyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(GAME_BOY_FRAME_RATE_HZ))
            .init_resource::<GameBoyFrameRing>()
            .init_resource::<GameBoyFrameTexture>()
            .init_resource::<GameBoyRomLoadRequest>()
            .init_resource::<GameBoyLoadStatus>()
            .init_resource::<GameBoyRomLoadTaskState>()
            .add_systems(
                OnEnter(AppState::Gameplay),
                (
                    spawn_game_boy_emulator,
                    spawn_game_boy_frame_display,
                    begin_game_boy_rom_load,
                )
                    .chain(),
            )
            .configure_sets(
                Update,
                (GameBoyUpdateSet::RomLoad, GameBoyUpdateSet::Emulation)
                    .chain()
                    .after(crate::input::InputSet::UpdateGameBoyState)
                    .run_if(in_state(AppState::Gameplay)),
            )
            .add_systems(
                Update,
                finish_game_boy_rom_load
                    .run_if(has_pending_game_boy_rom_load)
                    .in_set(GameBoyUpdateSet::RomLoad),
            )
            .add_systems(
                Update,
                (
                    apply_emulator_control_events,
                    sync_joypad_input,
                    tick_game_boy_emulator,
                    persist_dirty_sram,
                    update_game_boy_frame_texture,
                    resize_game_boy_frame_display,
                )
                    .chain()
                    .in_set(GameBoyUpdateSet::Emulation),
            );
    }
}
