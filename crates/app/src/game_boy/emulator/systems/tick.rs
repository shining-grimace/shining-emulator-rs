use bevy::prelude::*;

use crate::game_boy::emulator::constants::{MAX_ACCUMULATED_CLOCKS, MIN_CLOCKS_TO_EXECUTE};
use crate::game_boy::emulator::cpu::CpuTiming;
use crate::game_boy::emulator::gpu::GpuMode;
use crate::game_boy::emulator::memory::GameBoyMemory;
use crate::game_boy::emulator::runtime::RuntimeControl;
use crate::game_boy::emulator::{GameBoyCore, GameBoyEmulator};
use crate::game_boy::frame_buffer::GameBoyFrameRing;

const JOYP_IO_INDEX: usize = 0x00;
const IF_IO_INDEX: usize = 0x0f;
const LY_IO_INDEX: usize = 0x44;

const SCAN_OAM_CLOCKS: i32 = 80;
const SCAN_VRAM_CLOCKS: i32 = 172;
const HBLANK_CLOCKS: i32 = 204;
const VBLANK_LINE_CLOCKS: i32 = 456;
const FIRST_VBLANK_LINE: u8 = 144;
const FRAME_LINE_COUNT: u8 = 154;

pub(crate) fn tick_game_boy_emulator(
    time: Res<Time>,
    mut frames: ResMut<GameBoyFrameRing>,
    mut emulators: Query<&mut GameBoyCore, With<GameBoyEmulator>>,
) {
    for mut emulator in &mut emulators {
        let emulator = &mut *emulator;
        if !emulator.runtime.is_running || emulator.runtime.is_paused {
            continue;
        }

        accumulate_clocks(
            &mut emulator.cpu_timing,
            &emulator.runtime,
            time.delta_secs(),
        );
        if emulator.cpu_timing.clocks_acc < MIN_CLOCKS_TO_EXECUTE {
            continue;
        }

        execute_accumulated_clocks_scaffold(emulator, &mut frames);
    }
}

fn accumulate_clocks(cpu_timing: &mut CpuTiming, control: &RuntimeControl, delta_seconds: f32) {
    let Some(adjusted_frequency) = adjusted_clock_frequency(cpu_timing, control) else {
        return;
    };
    let clocks_to_add =
        ((delta_seconds as f64) * adjusted_frequency as f64).clamp(0.0, i32::MAX as f64) as i32;

    let approximate_multiplier = control
        .clock_multiply
        .checked_div(control.clock_divide)
        .unwrap_or(1)
        .saturating_add(1)
        .max(1);
    let cap = i32::try_from((MAX_ACCUMULATED_CLOCKS as i64).saturating_mul(approximate_multiplier))
        .unwrap_or(i32::MAX);

    cpu_timing.clocks_acc = cpu_timing.clocks_acc.saturating_add(clocks_to_add).min(cap);
}

fn adjusted_clock_frequency(cpu_timing: &CpuTiming, control: &RuntimeControl) -> Option<i64> {
    if control.clock_divide <= 0 {
        return None;
    }

    Some(
        cpu_timing
            .clock_frequency_hz
            .saturating_mul(control.clock_multiply)
            / control.clock_divide,
    )
}

fn execute_accumulated_clocks_scaffold(emulator: &mut GameBoyCore, frames: &mut GameBoyFrameRing) {
    apply_joypad_state_change(emulator);

    let gpu_clock_factor = emulator.gpu_timing.clock_factor.max(1);
    let gpu_clocks = emulator.cpu_timing.clocks_acc / gpu_clock_factor;
    emulator.cpu_timing.clocks_acc %= gpu_clock_factor;
    advance_ppu_timing(gpu_clocks, emulator, frames);
}

fn advance_ppu_timing(clocks: i32, emulator: &mut GameBoyCore, frames: &mut GameBoyFrameRing) {
    emulator.gpu_timing.time_in_mode = emulator
        .gpu_timing
        .time_in_mode
        .saturating_add(clocks.max(0));

    loop {
        let mode_clock_target = match emulator.gpu_mode {
            GpuMode::ScanOam => SCAN_OAM_CLOCKS,
            GpuMode::ScanVram => SCAN_VRAM_CLOCKS,
            GpuMode::HBlank => HBLANK_CLOCKS,
            GpuMode::VBlank => VBLANK_LINE_CLOCKS,
        };

        if emulator.gpu_timing.time_in_mode < mode_clock_target {
            break;
        }

        emulator.gpu_timing.time_in_mode -= mode_clock_target;
        match emulator.gpu_mode {
            GpuMode::ScanOam => {
                emulator.gpu_mode = GpuMode::ScanVram;
                emulator.memory_access.oam = false;
                emulator.memory_access.vram = false;
            }
            GpuMode::ScanVram => {
                let line = ly(&emulator.memory);
                if line < FIRST_VBLANK_LINE {
                    emulator.video_frame.write_random_greyscale_line(line);
                }
                emulator.gpu_mode = GpuMode::HBlank;
                emulator.memory_access.oam = true;
                emulator.memory_access.vram = true;
            }
            GpuMode::HBlank => {
                let next_line = ly(&emulator.memory).saturating_add(1);
                set_ly(&mut emulator.memory, next_line);
                if next_line == FIRST_VBLANK_LINE {
                    emulator.gpu_mode = GpuMode::VBlank;
                    request_vblank_interrupt(&mut emulator.memory);
                    emulator.video_frame.publish_frame(frames);
                    emulator.memory_access.oam = true;
                    emulator.memory_access.vram = true;
                } else {
                    emulator.gpu_mode = GpuMode::ScanOam;
                    emulator.memory_access.oam = false;
                    emulator.memory_access.vram = true;
                }
            }
            GpuMode::VBlank => {
                let next_line = ly(&emulator.memory).saturating_add(1);
                if next_line >= FRAME_LINE_COUNT {
                    set_ly(&mut emulator.memory, 0);
                    emulator.gpu_mode = GpuMode::ScanOam;
                    emulator.memory_access.oam = false;
                    emulator.memory_access.vram = true;
                    emulator.video_frame.begin_frame();
                } else {
                    set_ly(&mut emulator.memory, next_line);
                }
            }
        }
    }
}

fn apply_joypad_state_change(emulator: &mut GameBoyCore) {
    if !emulator.runtime.joypad_state_changed {
        return;
    }

    let Some(joyp) = emulator.memory.io_ports.get_mut(JOYP_IO_INDEX) else {
        warn!("Game Boy IO ports are unavailable while applying joypad state");
        return;
    };

    match *joyp & 0x30 {
        0x20 => {
            *joyp &= 0xf0;
            *joyp |= emulator.runtime.joypad.direction;
        }
        0x10 => {
            *joyp &= 0xf0;
            *joyp |= emulator.runtime.joypad.button;
        }
        _ => {}
    }

    if let Some(interrupt_flags) = emulator.memory.io_ports.get_mut(IF_IO_INDEX) {
        *interrupt_flags |= 0x10;
    } else {
        warn!("Game Boy interrupt flags are unavailable while applying joypad state");
    }

    emulator.runtime.joypad_state_changed = false;
}

fn ly(memory: &GameBoyMemory) -> u8 {
    memory.io_ports.get(LY_IO_INDEX).copied().unwrap_or(0)
}

fn set_ly(memory: &mut GameBoyMemory, line: u8) {
    if let Some(ly) = memory.io_ports.get_mut(LY_IO_INDEX) {
        *ly = line;
    } else {
        warn!("Game Boy LY register is unavailable");
    }
}

fn request_vblank_interrupt(memory: &mut GameBoyMemory) {
    if let Some(interrupt_flags) = memory.io_ports.get_mut(IF_IO_INDEX) {
        *interrupt_flags |= 0x01;
    } else {
        warn!("Game Boy interrupt flags are unavailable while entering VBlank");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_boy::emulator::gpu::{GpuTiming, MemoryAccess};
    use crate::game_boy::emulator::input::JoypadInputNibbles;

    #[test]
    fn joypad_changes_update_the_selected_input_nibble_and_request_interrupt() {
        let mut emulator = GameBoyCore::default();
        emulator.runtime = RuntimeControl {
            joypad: JoypadInputNibbles {
                button: 0x0e,
                direction: 0x0d,
            },
            joypad_state_changed: true,
            ..Default::default()
        };
        emulator.memory.io_ports[JOYP_IO_INDEX] = 0x20;

        apply_joypad_state_change(&mut emulator);

        assert_eq!(emulator.memory.io_ports[JOYP_IO_INDEX], 0x2d);
        assert_eq!(emulator.memory.io_ports[IF_IO_INDEX], 0x10);
        assert!(!emulator.runtime.joypad_state_changed);
    }

    #[test]
    fn ppu_timing_writes_visible_lines_and_publishes_on_vblank() {
        let mut emulator = GameBoyCore {
            gpu_timing: GpuTiming::default(),
            gpu_mode: GpuMode::ScanOam,
            memory_access: MemoryAccess::default(),
            memory: GameBoyMemory::default(),
            ..Default::default()
        };
        let mut frames = GameBoyFrameRing::default();

        advance_ppu_timing(
            VBLANK_LINE_CLOCKS * i32::from(FIRST_VBLANK_LINE),
            &mut emulator,
            &mut frames,
        );

        assert_eq!(ly(&emulator.memory), FIRST_VBLANK_LINE);
        assert_eq!(emulator.gpu_mode, GpuMode::VBlank);
        assert_eq!(emulator.memory.io_ports[IF_IO_INDEX], 0x01);
        assert!(frames.latest_written_frame().is_some());
    }
}
