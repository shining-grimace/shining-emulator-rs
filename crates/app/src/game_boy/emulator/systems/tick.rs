use bevy::prelude::*;

use crate::game_boy::emulator::bus::{run_hblank_dma, write16};
use crate::game_boy::emulator::constants::{MAX_ACCUMULATED_CLOCKS, MIN_CLOCKS_TO_EXECUTE};
use crate::game_boy::emulator::cpu::{CpuMode, CpuTiming};
use crate::game_boy::emulator::execution::perform_op;
use crate::game_boy::emulator::gpu::GpuMode;
use crate::game_boy::emulator::memory::GameBoyMemory;
use crate::game_boy::emulator::runtime::RuntimeControl;
use crate::game_boy::emulator::{GameBoyCore, GameBoyEmulator};
use crate::game_boy::frame_buffer::GameBoyFrameRing;
use crate::storage::LocalStorage;

const JOYP_IO_INDEX: usize = 0x00;
const SB_IO_INDEX: usize = 0x01;
const SC_IO_INDEX: usize = 0x02;
const DIV_IO_INDEX: usize = 0x04;
const TIMA_IO_INDEX: usize = 0x05;
const TMA_IO_INDEX: usize = 0x06;
const IF_IO_INDEX: usize = 0x0f;
const LCDC_IO_INDEX: usize = 0x40;
const STAT_IO_INDEX: usize = 0x41;
const LY_IO_INDEX: usize = 0x44;
const LYC_IO_INDEX: usize = 0x45;
const IE_IO_INDEX: usize = 0xff;

const INTERRUPT_VBLANK: u8 = 0x01;
const INTERRUPT_LCD_STAT: u8 = 0x02;
const INTERRUPT_TIMER: u8 = 0x04;
const INTERRUPT_SERIAL: u8 = 0x08;
const INTERRUPT_JOYPAD: u8 = 0x10;

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
        let clocks_passed = perform_op(emulator);
        emulator.cpu_registers.pc &= 0xffff;
        emulator.cpu_timing.clocks_acc =
            emulator.cpu_timing.clocks_acc.saturating_sub(clocks_passed);

        handle_interrupts(emulator);

        if emulator.cpu_mode == CpuMode::Stopped {
            if switch_running_speed(emulator) {
                emulator.cpu_timing.clocks_acc =
                    emulator.cpu_timing.clocks_acc.saturating_sub(131_072);
                emulator.cpu_mode = CpuMode::Running;
            }
            continue;
        }

        let display_enabled = display_enabled(&emulator.memory);
        update_ly_compare(emulator, display_enabled);
        update_timers(emulator, clocks_passed);
        emulator
            .audio_unit
            .simulate_placeholder(clocks_passed / emulator.gpu_timing.clock_factor.max(1));
        update_serial(emulator, clocks_passed);

        if display_enabled {
            emulator.gpu_timing.blanked_screen = false;
            let gpu_clocks = clocks_passed / emulator.gpu_timing.clock_factor.max(1);
            advance_ppu_timing(gpu_clocks, emulator, frames);
        } else {
            handle_lcd_disabled(emulator, frames);
        }
    }
}

fn handle_interrupts(emulator: &mut GameBoyCore) {
    let cpu_halted = emulator.cpu_mode == CpuMode::Halted;
    if !emulator.cpu_registers.ime && !cpu_halted {
        return;
    }

    let enabled = io(&emulator.memory, IE_IO_INDEX);
    let requested = io(&emulator.memory, IF_IO_INDEX);
    let triggered = enabled & requested & 0x1f;
    if triggered == 0 {
        return;
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

    if !cpu_halted || emulator.cpu_registers.ime {
        emulator.cpu_registers.sp = emulator.cpu_registers.sp.wrapping_sub(2) & 0xffff;
        let stack_pointer = emulator.cpu_registers.sp as u16;
        let program_counter = emulator.cpu_registers.pc as u16;
        write16(emulator, stack_pointer, program_counter);
        emulator.cpu_registers.pc = target;
    }
    emulator.cpu_mode = CpuMode::Running;
    emulator.cpu_registers.ime = false;
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

fn update_timers(emulator: &mut GameBoyCore, clocks_passed: i32) {
    emulator.cpu_timing.divider_count = emulator
        .cpu_timing
        .divider_count
        .saturating_add(clocks_passed.max(0) as u32);
    while emulator.cpu_timing.divider_count >= 256 {
        emulator.cpu_timing.divider_count -= 256;
        let divider = io(&emulator.memory, DIV_IO_INDEX).wrapping_add(1);
        write_io(&mut emulator.memory, DIV_IO_INDEX, divider);
    }

    if !emulator.cpu_timing.timer_running {
        return;
    }

    emulator.cpu_timing.timer_count = emulator
        .cpu_timing
        .timer_count
        .saturating_add(clocks_passed.max(0) as u32);
    while emulator.cpu_timing.timer_count >= emulator.cpu_timing.timer_inc_time.max(1) {
        emulator.cpu_timing.timer_count -= emulator.cpu_timing.timer_inc_time.max(1);
        let next_timer = io(&emulator.memory, TIMA_IO_INDEX).wrapping_add(1);
        if next_timer == 0 {
            let modulo = io(&emulator.memory, TMA_IO_INDEX);
            write_io(&mut emulator.memory, TIMA_IO_INDEX, modulo);
            request_interrupt(&mut emulator.memory, INTERRUPT_TIMER);
        } else {
            write_io(&mut emulator.memory, TIMA_IO_INDEX, next_timer);
        }
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
                set_ly(&mut emulator.memory, next_line);
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
                    set_ly(&mut emulator.memory, 0);
                    set_gpu_mode(emulator, GpuMode::ScanOam);
                    emulator.memory_access.oam = false;
                    emulator.memory_access.vram = true;
                    emulator.video_frame.begin_frame();
                    if io(&emulator.memory, STAT_IO_INDEX) & 0x20 != 0 {
                        request_lcd_stat_interrupt(&mut emulator.memory);
                    }
                } else {
                    set_ly(&mut emulator.memory, next_line);
                }
            }
        }
    }
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
    set_ly(&mut emulator.memory, 0);
    emulator.gpu_timing.time_in_mode = 0;
    emulator.gpu_mode = GpuMode::ScanOam;
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
        *interrupt_flags |= INTERRUPT_JOYPAD;
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
        emulator.memory.io_ports[0x55] = 0x80;
        emulator.memory.rom[0x0200] = 0x5a;
        let mut frames = GameBoyFrameRing::default();

        advance_ppu_timing(SCAN_VRAM_CLOCKS, &mut emulator, &mut frames);

        assert_eq!(emulator.memory.vram[0], 0x5a);
        assert_eq!(emulator.memory.io_ports[0x55], 0xff);
        assert!(emulator.memory_access.vram);
    }

    #[test]
    fn timer_overflow_reloads_modulo_and_requests_interrupt() {
        let mut emulator = GameBoyCore::default();
        emulator.cpu_timing.timer_running = true;
        emulator.cpu_timing.timer_inc_time = 16;
        emulator.cpu_timing.timer_count = 12;
        emulator.memory.io_ports[TIMA_IO_INDEX] = 0xff;
        emulator.memory.io_ports[TMA_IO_INDEX] = 0x42;

        update_timers(&mut emulator, 4);

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
