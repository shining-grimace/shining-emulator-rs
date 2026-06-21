use bevy::prelude::*;

use crate::game_boy::emulator::bus::{
    begin_deferred_oam_dma, run_hblank_dma, step_oam_dma, step_system_counter, write16,
};
use crate::game_boy::emulator::constants::{MAX_ACCUMULATED_CLOCKS, MIN_CLOCKS_TO_EXECUTE};
use crate::game_boy::emulator::cpu::{CpuMode, CpuTiming};
use crate::game_boy::emulator::execution::perform_op;
use crate::game_boy::emulator::gpu::GpuMode;
use crate::game_boy::emulator::input::{
    JOYP_LOW_NIBBLE_MASK, JOYP_SELECT_MASK, JOYP_SELECT_NONE, joypad_low_nibble_falling_edge,
};
use crate::game_boy::emulator::memory::GameBoyMemory;
use crate::game_boy::emulator::runtime::RuntimeControl;
use crate::game_boy::emulator::video::select_objects_for_line;
use crate::game_boy::emulator::{GameBoyCore, GameBoyEmulator};
use crate::game_boy::frame_buffer::GameBoyFrameRing;
use crate::storage::LocalStorage;

const JOYP_IO_INDEX: usize = 0x00;
const SB_IO_INDEX: usize = 0x01;
const SC_IO_INDEX: usize = 0x02;
const TIMA_IO_INDEX: usize = 0x05;
const TMA_IO_INDEX: usize = 0x06;
const TAC_IO_INDEX: usize = 0x07;
const IF_IO_INDEX: usize = 0x0f;
const LCDC_IO_INDEX: usize = 0x40;
const STAT_IO_INDEX: usize = 0x41;
const SCX_IO_INDEX: usize = 0x43;
const LY_IO_INDEX: usize = 0x44;
const LYC_IO_INDEX: usize = 0x45;
const WY_IO_INDEX: usize = 0x4a;
const WX_IO_INDEX: usize = 0x4b;
const IE_IO_INDEX: usize = 0xff;

const LCDC_BG_ENABLE: u8 = 0x01;
const LCDC_OBJ_ENABLE: u8 = 0x02;
const LCDC_WINDOW_ENABLE: u8 = 0x20;

const INTERRUPT_VBLANK: u8 = 0x01;
const INTERRUPT_LCD_STAT: u8 = 0x02;
const INTERRUPT_TIMER: u8 = 0x04;
const INTERRUPT_SERIAL: u8 = 0x08;
const INTERRUPT_JOYPAD: u8 = 0x10;

const MACHINE_CYCLE_CLOCKS: i32 = 4;
const INTERRUPT_ACK_MACHINE_CYCLES: i32 = 5;
const STOP_SPEED_SWITCH_CLOCKS: i32 = 131_072;
const SCAN_OAM_CLOCKS: i32 = 80;
const SCAN_VRAM_CLOCKS: i32 = 172;
const MAX_SCAN_VRAM_CLOCKS: i32 = 289;
const SCANLINE_CLOCKS: i32 = 456;
const VBLANK_LINE_CLOCKS: i32 = SCANLINE_CLOCKS;
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

        execute_accumulated_clocks(emulator, &mut frames);
    }
}

pub(crate) fn persist_dirty_sram(
    storage: Res<LocalStorage>,
    mut emulators: Query<&mut GameBoyCore, With<GameBoyEmulator>>,
) {
    for mut emulator in &mut emulators {
        if !emulator.sram.is_dirty() {
            continue;
        }
        if emulator.rom.current_rom_id.is_empty() {
            continue;
        }

        let rom_id = emulator.rom.current_rom_id.clone();
        let Some(data) = emulator.sram.save_data() else {
            continue;
        };
        match storage.save_sram(&rom_id, data) {
            Ok(()) => emulator.sram.clear_dirty(),
            Err(error) => warn!("failed to persist Game Boy SRAM: {error}"),
        }
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

fn execute_accumulated_clocks(emulator: &mut GameBoyCore, frames: &mut GameBoyFrameRing) {
    apply_joypad_state_change(emulator);

    while emulator.cpu_timing.clocks_acc > 0 {
        if emulator.dma.vram.cpu_halt_m_cycles > 0 {
            emulator.dma.vram.cpu_halt_m_cycles -= 1;
            step_machine_cycle(emulator, frames);
            continue;
        }

        if emulator.cpu_mode == CpuMode::Stopped {
            if switch_running_speed(emulator) {
                step_machine_cycles(
                    emulator,
                    frames,
                    machine_cycles_for_clocks(STOP_SPEED_SWITCH_CLOCKS),
                );
                emulator.cpu_mode = CpuMode::Running;
            } else {
                step_machine_cycle(emulator, frames);
            }
            continue;
        }

        match handle_interrupts(emulator) {
            InterruptAction::None => {}
            InterruptAction::WakeHalt => {
                step_machine_cycle(emulator, frames);
                continue;
            }
            InterruptAction::Service => {
                step_machine_cycles(emulator, frames, INTERRUPT_ACK_MACHINE_CYCLES);
                continue;
            }
        }

        let clocks_passed = perform_op(emulator);
        emulator.cpu_registers.pc &= 0xffff;
        step_machine_cycles(emulator, frames, machine_cycles_for_clocks(clocks_passed));
        begin_deferred_oam_dma(emulator);
        finish_instruction_interrupt_state(emulator);
    }
}

fn machine_cycles_for_clocks(clocks: i32) -> i32 {
    clocks
        .max(MACHINE_CYCLE_CLOCKS)
        .saturating_add(MACHINE_CYCLE_CLOCKS - 1)
        / MACHINE_CYCLE_CLOCKS
}

fn step_machine_cycles(
    emulator: &mut GameBoyCore,
    frames: &mut GameBoyFrameRing,
    machine_cycles: i32,
) {
    for _ in 0..machine_cycles.max(1) {
        step_machine_cycle(emulator, frames);
    }
}

fn step_machine_cycle(emulator: &mut GameBoyCore, frames: &mut GameBoyFrameRing) {
    emulator.cpu_timing.clocks_acc = emulator
        .cpu_timing
        .clocks_acc
        .saturating_sub(MACHINE_CYCLE_CLOCKS);

    let display_enabled = display_enabled(&emulator.memory);
    update_ly_compare(emulator, display_enabled);
    if emulator.cpu_mode != CpuMode::Stopped {
        step_system_counter(emulator, MACHINE_CYCLE_CLOCKS as u16);
    }
    advance_cartridge_rtc(emulator);
    step_oam_dma(emulator);
    emulator
        .audio_unit
        .advance_ticks(MACHINE_CYCLE_CLOCKS / emulator.gpu_timing.clock_factor.max(1));
    update_serial(emulator, MACHINE_CYCLE_CLOCKS);

    if display_enabled {
        emulator.gpu_timing.blanked_screen = false;
        let gpu_clocks = MACHINE_CYCLE_CLOCKS / emulator.gpu_timing.clock_factor.max(1);
        advance_ppu_timing(gpu_clocks, emulator, frames);
    } else {
        handle_lcd_disabled(emulator, frames);
    }
}

fn advance_cartridge_rtc(emulator: &mut GameBoyCore) {
    let clock_factor = emulator.gpu_timing.clock_factor.max(1);
    let normal_speed_clocks = (MACHINE_CYCLE_CLOCKS / clock_factor).max(1);
    emulator.sram.advance_rtc(normal_speed_clocks as u32);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InterruptAction {
    None,
    WakeHalt,
    Service,
}

fn handle_interrupts(emulator: &mut GameBoyCore) -> InterruptAction {
    let cpu_halted = emulator.cpu_mode == CpuMode::Halted;
    let enabled = io(&emulator.memory, IE_IO_INDEX);
    let requested = io(&emulator.memory, IF_IO_INDEX);
    let triggered = enabled & requested & 0x1f;
    if triggered == 0 {
        return InterruptAction::None;
    }

    if cpu_halted {
        emulator.cpu_mode = CpuMode::Running;
        if !emulator.cpu_registers.ime {
            return InterruptAction::WakeHalt;
        }
    } else if !emulator.cpu_registers.ime {
        return InterruptAction::None;
    }

    let (mask, target) = if triggered & INTERRUPT_VBLANK != 0 {
        (0x1e, 0x0040)
    } else if triggered & INTERRUPT_LCD_STAT != 0 {
        (0x1d, 0x0048)
    } else if triggered & INTERRUPT_TIMER != 0 {
        (0x1b, 0x0050)
    } else if triggered & INTERRUPT_SERIAL != 0 {
        (0x17, 0x0058)
    } else {
        (0x0f, 0x0060)
    };
    write_io(&mut emulator.memory, IF_IO_INDEX, requested & mask);
    emulator.cpu_registers.sp = emulator.cpu_registers.sp.wrapping_sub(2) & 0xffff;
    let stack_pointer = emulator.cpu_registers.sp as u16;
    let program_counter = emulator.cpu_registers.pc as u16;
    write16(emulator, stack_pointer, program_counter);
    emulator.cpu_registers.pc = target;
    emulator.cpu_registers.ime = false;
    emulator.cpu_registers.ime_enable_delay = 0;
    InterruptAction::Service
}

fn finish_instruction_interrupt_state(emulator: &mut GameBoyCore) {
    if emulator.cpu_registers.ime_enable_delay == 0 {
        return;
    }

    emulator.cpu_registers.ime_enable_delay -= 1;
    if emulator.cpu_registers.ime_enable_delay == 0 {
        emulator.cpu_registers.ime = true;
    }
}

fn switch_running_speed(emulator: &mut GameBoyCore) -> bool {
    let key1 = io(&emulator.memory, 0x4d);
    let speed_change_requested = emulator.rom.properties.cgb_flag && key1 & 0x01 != 0;
    if speed_change_requested {
        if key1 & 0x80 == 0 {
            write_io(&mut emulator.memory, 0x4d, 0x80);
            emulator.cpu_timing.clock_frequency_hz = 8_400_000;
            emulator.gpu_timing.clock_factor = 2;
        } else {
            write_io(&mut emulator.memory, 0x4d, 0x00);
            emulator.cpu_timing.clock_frequency_hz = 4_194_304;
            emulator.gpu_timing.clock_factor = 1;
        }
    }
    speed_change_requested
}

fn update_ly_compare(emulator: &mut GameBoyCore, display_enabled: bool) {
    if io(&emulator.memory, LY_IO_INDEX) == io(&emulator.memory, LYC_IO_INDEX) && display_enabled {
        let stat = io(&emulator.memory, STAT_IO_INDEX);
        write_io(&mut emulator.memory, STAT_IO_INDEX, stat | 0x04);
        if stat & 0x40 != 0 && emulator.gpu_timing.last_ly_compare == 0 {
            request_lcd_stat_interrupt(&mut emulator.memory);
        }
        emulator.gpu_timing.last_ly_compare = 1;
    } else {
        let stat = io(&emulator.memory, STAT_IO_INDEX);
        write_io(&mut emulator.memory, STAT_IO_INDEX, stat & 0xfb);
        emulator.gpu_timing.last_ly_compare = 0;
    }
}

fn update_serial(emulator: &mut GameBoyCore, clocks_passed: i32) {
    if !emulator.serial.is_transferring {
        return;
    }

    if !emulator.serial.clock_is_external {
        emulator.serial.timer = emulator.serial.timer.saturating_sub(clocks_passed);
        if emulator.serial.timer <= 0 {
            emulator.serial.is_transferring = false;
            let serial_control = io(&emulator.memory, SC_IO_INDEX) & 0x03;
            write_io(&mut emulator.memory, SC_IO_INDEX, serial_control);
            request_interrupt(&mut emulator.memory, INTERRUPT_SERIAL);
            write_io(&mut emulator.memory, SB_IO_INDEX, 0xff);
        }
    } else if emulator.serial.timer == 1 {
        emulator.serial.timer = 0;
    }
}

fn advance_ppu_timing(clocks: i32, emulator: &mut GameBoyCore, frames: &mut GameBoyFrameRing) {
    emulator.gpu_timing.time_in_mode = emulator
        .gpu_timing
        .time_in_mode
        .saturating_add(clocks.max(0));

    loop {
        let mode_clock_target = mode_clock_target(emulator);

        if emulator.gpu_timing.time_in_mode < mode_clock_target {
            break;
        }

        emulator.gpu_timing.time_in_mode -= mode_clock_target;
        match emulator.gpu_mode {
            GpuMode::ScanOam => {
                emulator.gpu_timing.line_scan_vram_clocks = scan_vram_clocks(emulator);
                set_gpu_mode(emulator, GpuMode::ScanVram);
                emulator.memory_access.oam = false;
                emulator.memory_access.vram = false;
            }
            GpuMode::ScanVram => {
                let line = ly(&emulator.memory);
                set_gpu_mode(emulator, GpuMode::HBlank);
                emulator.memory_access.oam = true;
                emulator.memory_access.vram = true;
                if io(&emulator.memory, STAT_IO_INDEX) & 0x08 != 0 {
                    request_lcd_stat_interrupt(&mut emulator.memory);
                }
                if line < FIRST_VBLANK_LINE {
                    run_hblank_dma(emulator);
                    write_visible_line(emulator, line);
                }
            }
            GpuMode::HBlank => {
                let next_line = ly(&emulator.memory).saturating_add(1);
                set_ly(emulator, next_line, true);
                if next_line == FIRST_VBLANK_LINE {
                    set_gpu_mode(emulator, GpuMode::VBlank);
                    request_interrupt(&mut emulator.memory, INTERRUPT_VBLANK);
                    if !emulator.sgb.freeze_screen {
                        if emulator.rom.properties.sgb_flag {
                            emulator.video_frame.colourise_sgb_frame(&emulator.sgb);
                        }
                        emulator.video_frame.publish_frame(frames);
                    }
                    emulator.memory_access.oam = true;
                    emulator.memory_access.vram = true;
                    if io(&emulator.memory, STAT_IO_INDEX) & 0x10 != 0 {
                        request_lcd_stat_interrupt(&mut emulator.memory);
                    }
                } else {
                    set_gpu_mode(emulator, GpuMode::ScanOam);
                    emulator.memory_access.oam = false;
                    emulator.memory_access.vram = true;
                    if io(&emulator.memory, STAT_IO_INDEX) & 0x20 != 0 {
                        request_lcd_stat_interrupt(&mut emulator.memory);
                    }
                }
            }
            GpuMode::VBlank => {
                let next_line = ly(&emulator.memory).saturating_add(1);
                if next_line >= FRAME_LINE_COUNT {
                    set_ly(emulator, 0, true);
                    set_gpu_mode(emulator, GpuMode::ScanOam);
                    emulator.memory_access.oam = false;
                    emulator.memory_access.vram = true;
                    emulator.video_frame.begin_frame();
                    if io(&emulator.memory, STAT_IO_INDEX) & 0x20 != 0 {
                        request_lcd_stat_interrupt(&mut emulator.memory);
                    }
                } else {
                    set_ly(emulator, next_line, true);
                }
            }
        }
    }
}

fn mode_clock_target(emulator: &GameBoyCore) -> i32 {
    match emulator.gpu_mode {
        GpuMode::ScanOam => SCAN_OAM_CLOCKS,
        GpuMode::ScanVram => emulator.gpu_timing.line_scan_vram_clocks,
        GpuMode::HBlank => SCANLINE_CLOCKS
            .saturating_sub(SCAN_OAM_CLOCKS)
            .saturating_sub(emulator.gpu_timing.line_scan_vram_clocks),
        GpuMode::VBlank => VBLANK_LINE_CLOCKS,
    }
}

fn scan_vram_clocks(emulator: &GameBoyCore) -> i32 {
    let memory = &emulator.memory;
    let lcd_control = io(memory, LCDC_IO_INDEX);
    let line = ly(memory);
    let mut clocks = SCAN_VRAM_CLOCKS + i32::from(io(memory, SCX_IO_INDEX) & 0x07);

    if window_visible_on_line(memory, lcd_control, line, emulator.rom.properties.cgb_flag) {
        clocks += 6;
    }

    if lcd_control & LCDC_OBJ_ENABLE != 0 {
        clocks +=
            object_penalty_clocks(memory, lcd_control, line, emulator.rom.properties.cgb_flag);
    }

    clocks.min(MAX_SCAN_VRAM_CLOCKS)
}

fn window_visible_on_line(
    memory: &GameBoyMemory,
    lcd_control: u8,
    line: u8,
    cgb_mode: bool,
) -> bool {
    let bg_window_enabled = cgb_mode || lcd_control & LCDC_BG_ENABLE != 0;
    bg_window_enabled
        && lcd_control & LCDC_WINDOW_ENABLE != 0
        && io(memory, WY_IO_INDEX) <= line
        && io(memory, WX_IO_INDEX) < 167
}

fn object_penalty_clocks(memory: &GameBoyMemory, lcd_control: u8, line: u8, cgb_mode: bool) -> i32 {
    let mut objects: Vec<_> = select_objects_for_line(memory, lcd_control, usize::from(line))
        .into_iter()
        .filter(|object| object.oam_x < 168)
        .collect();
    objects.sort_by_key(|object| (object.screen_x, object.index));

    let scroll_x = i32::from(io(memory, SCX_IO_INDEX));
    let window_start_x = if window_visible_on_line(memory, lcd_control, line, cgb_mode) {
        Some(i32::from(io(memory, WX_IO_INDEX)).saturating_sub(7))
    } else {
        None
    };
    let mut considered_tiles = Vec::with_capacity(10);
    let mut penalty = 0;

    for object in objects {
        if object.oam_x == 0 {
            penalty += 11;
            continue;
        }

        let (tile_key, tile_pixel) = object_penalty_tile(object.screen_x, scroll_x, window_start_x);
        if !considered_tiles.contains(&tile_key) {
            considered_tiles.push(tile_key);
            penalty += (5 - tile_pixel).max(0);
        }
        penalty += 6;
    }

    penalty
}

fn object_penalty_tile(screen_x: i32, scroll_x: i32, window_start_x: Option<i32>) -> (i32, i32) {
    if let Some(window_start_x) = window_start_x {
        if screen_x >= window_start_x {
            let window_x = screen_x - window_start_x;
            return (1_000 + window_x.div_euclid(8), window_x.rem_euclid(8));
        }
    }

    let background_x = screen_x + scroll_x;
    (background_x.div_euclid(8), background_x.rem_euclid(8))
}

fn write_visible_line(emulator: &mut GameBoyCore, line: u8) {
    match emulator.line_renderer {
        crate::game_boy::emulator::gpu::LineRenderer::Gb => {
            emulator
                .video_frame
                .write_gb_line(line, &emulator.memory, &emulator.palettes);
        }
        crate::game_boy::emulator::gpu::LineRenderer::Sgb => {
            emulator.video_frame.write_sgb_line(
                line,
                &emulator.memory,
                &emulator.palettes,
                &mut emulator.sgb,
            );
        }
        crate::game_boy::emulator::gpu::LineRenderer::Cgb => {
            emulator
                .video_frame
                .write_cgb_line(line, &emulator.memory, &emulator.palettes);
        }
    }
}

fn handle_lcd_disabled(emulator: &mut GameBoyCore, frames: &mut GameBoyFrameRing) {
    if emulator.gpu_timing.blanked_screen {
        return;
    }

    emulator.memory_access.oam = true;
    emulator.memory_access.vram = true;
    set_ly(emulator, 0, false);
    emulator.gpu_timing.time_in_mode = 0;
    emulator.gpu_mode = GpuMode::HBlank;
    let stat = io(&emulator.memory, STAT_IO_INDEX);
    write_io(&mut emulator.memory, STAT_IO_INDEX, stat & 0xfc);
    emulator.gpu_timing.blanked_screen = true;
    emulator.video_frame.begin_frame();
    emulator.video_frame.clear_black();
    emulator.video_frame.publish_frame(frames);
}

fn apply_joypad_state_change(emulator: &mut GameBoyCore) {
    if !emulator.runtime.joypad_state_changed {
        return;
    }

    let (old_joyp, new_joyp) = {
        let Some(joyp) = emulator.memory.io_ports.get_mut(JOYP_IO_INDEX) else {
            warn!("Game Boy IO ports are unavailable while applying joypad state");
            return;
        };

        let old_joyp = *joyp;
        let select = old_joyp & JOYP_SELECT_MASK;
        let low_nibble = if emulator.rom.properties.sgb_flag
            && emulator.sgb.mult_enabled
            && select == JOYP_SELECT_NONE
        {
            emulator.sgb.read_joypad_id as u8 & JOYP_LOW_NIBBLE_MASK
        } else {
            emulator.runtime.joypad.low_nibble_for_select(select)
        };
        let new_joyp = 0xc0 | select | low_nibble;
        *joyp = new_joyp;
        (old_joyp, new_joyp)
    };

    if joypad_low_nibble_falling_edge(old_joyp, new_joyp) {
        if let Some(interrupt_flags) = emulator.memory.io_ports.get_mut(IF_IO_INDEX) {
            *interrupt_flags |= INTERRUPT_JOYPAD;
        } else {
            warn!("Game Boy interrupt flags are unavailable while applying joypad state");
        }
    }

    emulator.runtime.joypad_state_changed = false;
}

fn ly(memory: &GameBoyMemory) -> u8 {
    memory.io_ports.get(LY_IO_INDEX).copied().unwrap_or(0)
}

fn set_ly(emulator: &mut GameBoyCore, line: u8, display_enabled: bool) {
    if let Some(ly) = emulator.memory.io_ports.get_mut(LY_IO_INDEX) {
        *ly = line;
    } else {
        warn!("Game Boy LY register is unavailable");
    }
    update_ly_compare(emulator, display_enabled);
}

fn set_gpu_mode(emulator: &mut GameBoyCore, mode: GpuMode) {
    emulator.gpu_mode = mode;
    let mode_bits = match mode {
        GpuMode::HBlank => 0,
        GpuMode::VBlank => 1,
        GpuMode::ScanOam => 2,
        GpuMode::ScanVram => 3,
    };
    let stat = io(&emulator.memory, STAT_IO_INDEX);
    write_io(
        &mut emulator.memory,
        STAT_IO_INDEX,
        (stat & 0xfc) | mode_bits,
    );
}

fn display_enabled(memory: &GameBoyMemory) -> bool {
    io(memory, LCDC_IO_INDEX) & 0x80 != 0
}

fn request_lcd_stat_interrupt(memory: &mut GameBoyMemory) {
    request_interrupt(memory, INTERRUPT_LCD_STAT);
}

fn request_interrupt(memory: &mut GameBoyMemory, interrupt: u8) {
    if let Some(interrupt_flags) = memory.io_ports.get_mut(IF_IO_INDEX) {
        *interrupt_flags |= interrupt;
    } else {
        warn!("Game Boy interrupt flags are unavailable");
    }
}

fn io(memory: &GameBoyMemory, index: usize) -> u8 {
    memory.io_ports.get(index).copied().unwrap_or(0)
}

fn write_io(memory: &mut GameBoyMemory, index: usize, value: u8) {
    memory.write_io_port(index, value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_boy::emulator::gpu::{GpuTiming, MemoryAccess};
    use crate::game_boy::emulator::input::JoypadInputNibbles;

    #[test]
    fn joypad_press_updates_selected_nibble_and_requests_interrupt() {
        let mut emulator = GameBoyCore::default();
        emulator.runtime = RuntimeControl {
            joypad: JoypadInputNibbles {
                button: 0x0f,
                direction: 0x0d,
            },
            joypad_state_changed: true,
            ..Default::default()
        };
        emulator.memory.io_ports[JOYP_IO_INDEX] = 0xef;

        apply_joypad_state_change(&mut emulator);

        assert_eq!(emulator.memory.io_ports[JOYP_IO_INDEX], 0xed);
        assert_eq!(emulator.memory.io_ports[IF_IO_INDEX], 0x10);
        assert!(!emulator.runtime.joypad_state_changed);
    }

    #[test]
    fn joypad_release_updates_selected_nibble_without_interrupt() {
        let mut emulator = GameBoyCore::default();
        emulator.runtime = RuntimeControl {
            joypad: JoypadInputNibbles {
                button: 0x0f,
                direction: 0x0f,
            },
            joypad_state_changed: true,
            ..Default::default()
        };
        emulator.memory.io_ports[JOYP_IO_INDEX] = 0xed;

        apply_joypad_state_change(&mut emulator);

        assert_eq!(emulator.memory.io_ports[JOYP_IO_INDEX], 0xef);
        assert_eq!(emulator.memory.io_ports[IF_IO_INDEX] & INTERRUPT_JOYPAD, 0);
        assert!(!emulator.runtime.joypad_state_changed);
    }

    #[test]
    fn joypad_unselected_row_change_does_not_request_interrupt() {
        let mut emulator = GameBoyCore::default();
        emulator.runtime = RuntimeControl {
            joypad: JoypadInputNibbles {
                button: 0x0e,
                direction: 0x0f,
            },
            joypad_state_changed: true,
            ..Default::default()
        };
        emulator.memory.io_ports[JOYP_IO_INDEX] = 0xef;

        apply_joypad_state_change(&mut emulator);

        assert_eq!(emulator.memory.io_ports[JOYP_IO_INDEX], 0xef);
        assert_eq!(emulator.memory.io_ports[IF_IO_INDEX] & INTERRUPT_JOYPAD, 0);
        assert!(!emulator.runtime.joypad_state_changed);
    }

    #[test]
    fn accumulated_clock_execution_runs_cpu_opcodes() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_registers.pc = 0x0100;
        emulator.cpu_timing.clocks_acc = 8;
        emulator.memory.rom[0x0100] = 0x3e;
        emulator.memory.rom[0x0101] = 0x77;
        let mut frames = GameBoyFrameRing::default();

        execute_accumulated_clocks(&mut emulator, &mut frames);

        assert_eq!(emulator.cpu_registers.a, 0x77);
        assert_eq!(emulator.cpu_registers.pc, 0x0102);
        assert_eq!(emulator.cpu_timing.clocks_acc, 0);
    }

    #[test]
    fn machine_cycle_step_drains_four_clocks_and_advances_timer() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_timing.clocks_acc = 8;
        emulator.cpu_timing.system_counter = 0x000c;
        emulator.memory.io_ports[TAC_IO_INDEX] = 0x05;
        let mut frames = GameBoyFrameRing::default();

        step_machine_cycle(&mut emulator, &mut frames);

        assert_eq!(emulator.cpu_timing.clocks_acc, 4);
        assert_eq!(emulator.cpu_timing.system_counter, 0x0010);
        assert_eq!(emulator.memory.io_ports[TIMA_IO_INDEX], 1);
    }

    #[test]
    fn stopped_machine_cycle_does_not_advance_system_counter() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_mode = CpuMode::Stopped;
        emulator.cpu_timing.clocks_acc = 4;
        emulator.cpu_timing.system_counter = 0x000c;
        emulator.memory.io_ports[TAC_IO_INDEX] = 0x05;
        let mut frames = GameBoyFrameRing::default();

        step_machine_cycle(&mut emulator, &mut frames);

        assert_eq!(emulator.cpu_timing.clocks_acc, 0);
        assert_eq!(emulator.cpu_timing.system_counter, 0x000c);
        assert_eq!(emulator.memory.io_ports[TIMA_IO_INDEX], 0);
    }

    #[test]
    fn interrupt_service_path_consumes_five_machine_cycles() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_registers.ime = true;
        emulator.cpu_registers.pc = 0x1234;
        emulator.cpu_registers.sp = 0xfffe;
        emulator.cpu_timing.clocks_acc = INTERRUPT_ACK_MACHINE_CYCLES * MACHINE_CYCLE_CLOCKS;
        emulator.memory.io_ports[IE_IO_INDEX] = INTERRUPT_VBLANK;
        emulator.memory.io_ports[IF_IO_INDEX] = INTERRUPT_VBLANK;
        let mut frames = GameBoyFrameRing::default();

        execute_accumulated_clocks(&mut emulator, &mut frames);

        assert_eq!(emulator.cpu_timing.clocks_acc, 0);
        assert_eq!(emulator.cpu_registers.pc, 0x0040);
        assert_eq!(emulator.cpu_registers.sp, 0xfffc);
        assert_eq!(emulator.memory.io_ports[IF_IO_INDEX] & INTERRUPT_VBLANK, 0);
    }

    #[test]
    fn vram_dma_halt_cycles_drain_without_executing_cpu_opcodes() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_registers.pc = 0x0100;
        emulator.cpu_timing.clocks_acc = 2 * MACHINE_CYCLE_CLOCKS;
        emulator.dma.vram.cpu_halt_m_cycles = 2;
        emulator.memory.rom[0x0100] = 0x04;
        let mut frames = GameBoyFrameRing::default();

        execute_accumulated_clocks(&mut emulator, &mut frames);

        assert_eq!(emulator.dma.vram.cpu_halt_m_cycles, 0);
        assert_eq!(emulator.cpu_timing.clocks_acc, 0);
        assert_eq!(emulator.cpu_registers.b, 0);
        assert_eq!(emulator.cpu_registers.pc, 0x0100);
    }

    #[test]
    fn ei_enables_interrupts_after_the_following_instruction() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_registers.pc = 0x0100;
        emulator.cpu_timing.clocks_acc = MACHINE_CYCLE_CLOCKS;
        emulator.memory.rom[0x0100] = 0xfb;
        emulator.memory.rom[0x0101] = 0x00;
        let mut frames = GameBoyFrameRing::default();

        execute_accumulated_clocks(&mut emulator, &mut frames);

        assert!(!emulator.cpu_registers.ime);
        assert_eq!(emulator.cpu_registers.ime_enable_delay, 1);

        emulator.cpu_timing.clocks_acc = MACHINE_CYCLE_CLOCKS;
        execute_accumulated_clocks(&mut emulator, &mut frames);

        assert!(emulator.cpu_registers.ime);
        assert_eq!(emulator.cpu_registers.ime_enable_delay, 0);
        assert_eq!(emulator.cpu_registers.pc, 0x0102);
    }

    #[test]
    fn pending_interrupt_after_ei_waits_until_after_next_instruction() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_registers.pc = 0x0100;
        emulator.cpu_registers.sp = 0xfffe;
        emulator.cpu_timing.clocks_acc =
            (2 * MACHINE_CYCLE_CLOCKS) + (INTERRUPT_ACK_MACHINE_CYCLES * MACHINE_CYCLE_CLOCKS);
        emulator.memory.rom[0x0100] = 0xfb;
        emulator.memory.rom[0x0101] = 0x00;
        emulator.memory.io_ports[IE_IO_INDEX] = INTERRUPT_VBLANK;
        emulator.memory.io_ports[IF_IO_INDEX] = INTERRUPT_VBLANK;
        let mut frames = GameBoyFrameRing::default();

        execute_accumulated_clocks(&mut emulator, &mut frames);

        assert_eq!(emulator.cpu_registers.pc, 0x0040);
        assert_eq!(emulator.cpu_registers.sp, 0xfffc);
        assert_eq!(emulator.memory.io_ports[0xfc], 0x02);
        assert_eq!(emulator.memory.io_ports[0xfd], 0x01);
    }

    #[test]
    fn di_cancels_delayed_ei() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_registers.pc = 0x0100;
        emulator.cpu_timing.clocks_acc = 3 * MACHINE_CYCLE_CLOCKS;
        emulator.memory.rom[0x0100] = 0xfb;
        emulator.memory.rom[0x0101] = 0xf3;
        emulator.memory.rom[0x0102] = 0x00;
        let mut frames = GameBoyFrameRing::default();

        execute_accumulated_clocks(&mut emulator, &mut frames);

        assert!(!emulator.cpu_registers.ime);
        assert_eq!(emulator.cpu_registers.ime_enable_delay, 0);
        assert_eq!(emulator.cpu_registers.pc, 0x0103);
    }

    #[test]
    fn halted_cpu_wakes_without_servicing_when_ime_is_disabled() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_mode = CpuMode::Halted;
        emulator.cpu_registers.pc = 0x0101;
        emulator.cpu_registers.sp = 0xfffe;
        emulator.cpu_timing.clocks_acc = MACHINE_CYCLE_CLOCKS;
        emulator.memory.io_ports[IE_IO_INDEX] = INTERRUPT_TIMER;
        emulator.memory.io_ports[IF_IO_INDEX] = INTERRUPT_TIMER;
        let mut frames = GameBoyFrameRing::default();

        execute_accumulated_clocks(&mut emulator, &mut frames);

        assert_eq!(emulator.cpu_mode, CpuMode::Running);
        assert_eq!(emulator.cpu_registers.pc, 0x0101);
        assert_eq!(emulator.cpu_registers.sp, 0xfffe);
        assert_eq!(
            emulator.memory.io_ports[IF_IO_INDEX] & INTERRUPT_TIMER,
            INTERRUPT_TIMER
        );
    }

    #[test]
    fn halt_bug_repeats_next_single_byte_instruction() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_registers.pc = 0x0100;
        emulator.cpu_timing.clocks_acc = 3 * MACHINE_CYCLE_CLOCKS;
        emulator.memory.rom[0x0100] = 0x76;
        emulator.memory.rom[0x0101] = 0x04;
        emulator.memory.rom[0x0102] = 0x00;
        emulator.memory.io_ports[IE_IO_INDEX] = INTERRUPT_TIMER;
        emulator.memory.io_ports[IF_IO_INDEX] = INTERRUPT_TIMER;
        let mut frames = GameBoyFrameRing::default();

        execute_accumulated_clocks(&mut emulator, &mut frames);

        assert_eq!(emulator.cpu_registers.b, 2);
        assert_eq!(emulator.cpu_registers.pc, 0x0102);
        assert_eq!(
            emulator.memory.io_ports[IF_IO_INDEX] & INTERRUPT_TIMER,
            INTERRUPT_TIMER
        );
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
        emulator.memory.io_ports[LCDC_IO_INDEX] = 0x91;
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

    #[test]
    fn mode_three_timing_includes_scroll_window_and_object_penalties() {
        let mut emulator = GameBoyCore::default();
        emulator.memory.io_ports[LCDC_IO_INDEX] =
            0x80 | LCDC_OBJ_ENABLE | LCDC_WINDOW_ENABLE | 0x01;
        emulator.memory.io_ports[SCX_IO_INDEX] = 7;
        emulator.memory.io_ports[WY_IO_INDEX] = 0;
        emulator.memory.io_ports[WX_IO_INDEX] = 7;
        emulator.memory.oam[0] = 16;
        emulator.memory.oam[1] = 8;

        assert_eq!(scan_vram_clocks(&emulator), 196);
    }

    #[test]
    fn cgb_lcdc_bit_zero_keeps_window_mode_three_penalty() {
        let mut emulator = GameBoyCore::default();
        emulator.rom.properties.cgb_flag = true;
        emulator.memory.io_ports[LCDC_IO_INDEX] = 0x80 | LCDC_WINDOW_ENABLE;
        emulator.memory.io_ports[WY_IO_INDEX] = 0;
        emulator.memory.io_ports[WX_IO_INDEX] = 7;

        assert_eq!(scan_vram_clocks(&emulator), SCAN_VRAM_CLOCKS + 6);
    }

    #[test]
    fn dmg_lcdc_bit_zero_suppresses_window_mode_three_penalty() {
        let mut emulator = GameBoyCore::default();
        emulator.memory.io_ports[LCDC_IO_INDEX] = 0x80 | LCDC_WINDOW_ENABLE;
        emulator.memory.io_ports[WY_IO_INDEX] = 0;
        emulator.memory.io_ports[WX_IO_INDEX] = 7;

        assert_eq!(scan_vram_clocks(&emulator), SCAN_VRAM_CLOCKS);
    }

    #[test]
    fn mode_three_uses_dynamic_duration_before_entering_hblank() {
        let mut emulator = GameBoyCore {
            gpu_timing: GpuTiming::default(),
            gpu_mode: GpuMode::ScanVram,
            memory_access: MemoryAccess {
                oam: false,
                vram: false,
                ..Default::default()
            },
            memory: GameBoyMemory::default(),
            ..Default::default()
        };
        emulator.memory.io_ports[LCDC_IO_INDEX] = 0x91;
        emulator.memory.io_ports[SCX_IO_INDEX] = 7;
        emulator.gpu_timing.line_scan_vram_clocks = scan_vram_clocks(&emulator);
        let mut frames = GameBoyFrameRing::default();

        advance_ppu_timing(178, &mut emulator, &mut frames);

        assert_eq!(emulator.gpu_mode, GpuMode::ScanVram);
        assert!(!emulator.memory_access.vram);

        advance_ppu_timing(1, &mut emulator, &mut frames);

        assert_eq!(emulator.gpu_mode, GpuMode::HBlank);
        assert_eq!(emulator.gpu_timing.time_in_mode, 0);
        assert!(emulator.memory_access.vram);
        assert!(emulator.memory_access.oam);
    }

    #[test]
    fn hblank_uses_scanline_remainder_after_dynamic_mode_three() {
        let mut emulator = GameBoyCore {
            gpu_timing: GpuTiming::default(),
            gpu_mode: GpuMode::HBlank,
            memory: GameBoyMemory::default(),
            ..Default::default()
        };
        emulator.memory.io_ports[LCDC_IO_INDEX] = 0x91;
        emulator.memory.io_ports[SCX_IO_INDEX] = 7;
        emulator.gpu_timing.line_scan_vram_clocks = scan_vram_clocks(&emulator);
        let mut frames = GameBoyFrameRing::default();

        advance_ppu_timing(196, &mut emulator, &mut frames);

        assert_eq!(ly(&emulator.memory), 0);
        assert_eq!(emulator.gpu_mode, GpuMode::HBlank);

        advance_ppu_timing(1, &mut emulator, &mut frames);

        assert_eq!(ly(&emulator.memory), 1);
        assert_eq!(emulator.gpu_mode, GpuMode::ScanOam);
        assert_eq!(emulator.gpu_timing.time_in_mode, 0);
    }

    #[test]
    fn lyc_compare_updates_immediately_when_hblank_advances_line() {
        let mut emulator = GameBoyCore {
            gpu_timing: GpuTiming::default(),
            gpu_mode: GpuMode::HBlank,
            memory: GameBoyMemory::default(),
            ..Default::default()
        };
        emulator.memory.io_ports[LCDC_IO_INDEX] = 0x91;
        emulator.memory.io_ports[LYC_IO_INDEX] = 1;
        emulator.memory.io_ports[STAT_IO_INDEX] = 0x40;
        let mut frames = GameBoyFrameRing::default();

        advance_ppu_timing(204, &mut emulator, &mut frames);

        assert_eq!(ly(&emulator.memory), 1);
        assert_eq!(emulator.memory.io_ports[STAT_IO_INDEX] & 0x04, 0x04);
        assert_eq!(
            emulator.memory.io_ports[IF_IO_INDEX] & INTERRUPT_LCD_STAT,
            INTERRUPT_LCD_STAT
        );
    }

    #[test]
    fn hblank_dma_runs_after_vram_access_reopens() {
        let mut emulator = GameBoyCore {
            gpu_timing: GpuTiming::default(),
            gpu_mode: GpuMode::ScanVram,
            memory_access: MemoryAccess {
                vram: false,
                ..Default::default()
            },
            memory: GameBoyMemory::default(),
            ..Default::default()
        };
        emulator.rom.properties.cgb_flag = true;
        emulator.memory.io_ports[LCDC_IO_INDEX] = 0x91;
        emulator.memory.io_ports[0x51] = 0x02;
        emulator.memory.io_ports[0x52] = 0x00;
        emulator.memory.io_ports[0x53] = 0x00;
        emulator.memory.io_ports[0x54] = 0x00;
        emulator.memory.io_ports[0x55] = 0x00;
        emulator.memory.rom[0x0200] = 0x5a;
        let mut frames = GameBoyFrameRing::default();

        advance_ppu_timing(SCAN_VRAM_CLOCKS, &mut emulator, &mut frames);

        assert_eq!(emulator.memory.vram[0], 0x5a);
        assert_eq!(emulator.memory.io_ports[0x55], 0xff);
        assert_eq!(emulator.dma.vram.cpu_halt_m_cycles, 8);
        assert!(emulator.memory_access.vram);
    }

    #[test]
    fn timer_overflow_reloads_modulo_and_requests_interrupt() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_timing.clocks_acc = 2 * MACHINE_CYCLE_CLOCKS;
        emulator.cpu_timing.system_counter = 0x000c;
        emulator.memory.io_ports[TIMA_IO_INDEX] = 0xff;
        emulator.memory.io_ports[TMA_IO_INDEX] = 0x42;
        emulator.memory.io_ports[TAC_IO_INDEX] = 0x05;
        let mut frames = GameBoyFrameRing::default();

        step_machine_cycle(&mut emulator, &mut frames);

        assert_eq!(emulator.memory.io_ports[TIMA_IO_INDEX], 0x00);
        assert_eq!(emulator.cpu_timing.tima_reload_delay, 1);
        assert_eq!(emulator.memory.io_ports[IF_IO_INDEX] & INTERRUPT_TIMER, 0);

        step_machine_cycle(&mut emulator, &mut frames);

        assert_eq!(emulator.memory.io_ports[TIMA_IO_INDEX], 0x42);
        assert_eq!(
            emulator.memory.io_ports[IF_IO_INDEX] & INTERRUPT_TIMER,
            INTERRUPT_TIMER
        );
    }

    #[test]
    fn lcd_disabled_publishes_a_black_frame_once() {
        let mut emulator = GameBoyCore::default();
        let mut frames = GameBoyFrameRing::default();

        handle_lcd_disabled(&mut emulator, &mut frames);

        let frame = frames
            .latest_written_frame()
            .expect("black frame should publish");
        assert!(frame.pixels().iter().all(|byte| *byte == 0));
        assert!(emulator.gpu_timing.blanked_screen);
    }

    #[test]
    fn lcd_disabled_reports_hblank_stat_mode_for_vram_wait_loops() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_registers.pc = 0x0100;
        emulator.cpu_timing.clocks_acc = 200;
        emulator.memory.io_ports[LCDC_IO_INDEX] = 0x91;
        emulator.memory.io_ports[STAT_IO_INDEX] = 0x02;
        emulator.memory.rom[0x0100] = 0x3e;
        emulator.memory.rom[0x0101] = 0x40;
        emulator.memory.rom[0x0102] = 0xe0;
        emulator.memory.rom[0x0103] = 0x40;
        emulator.memory.rom[0x0104] = 0xf0;
        emulator.memory.rom[0x0105] = 0x41;
        emulator.memory.rom[0x0106] = 0xe6;
        emulator.memory.rom[0x0107] = 0x02;
        emulator.memory.rom[0x0108] = 0x20;
        emulator.memory.rom[0x0109] = 0xfa;
        emulator.memory.rom[0x010a] = 0x3e;
        emulator.memory.rom[0x010b] = 0x99;
        emulator.memory.rom[0x010c] = 0x76;
        let mut frames = GameBoyFrameRing::default();

        execute_accumulated_clocks(&mut emulator, &mut frames);

        assert_eq!(emulator.cpu_registers.a, 0x99);
        assert_eq!(emulator.memory.io_ports[STAT_IO_INDEX] & 0x03, 0);
        assert!(frames.latest_written_frame().is_some());
    }
}
