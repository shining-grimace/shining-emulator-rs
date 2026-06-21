use std::fmt;

use serde::{Deserialize, Serialize};

use crate::game_boy::emulator::GameBoyCore;
use crate::game_boy::emulator::cpu::{CpuMode, CpuRegisters, CpuTiming, SerialState};
use crate::game_boy::emulator::dma::{DmaState, OamDmaState, VramDmaState};
use crate::game_boy::emulator::gpu::{GpuMode, GpuTiming, LineRenderer, MemoryAccess};
use crate::game_boy::emulator::input::JoypadInputNibbles;
use crate::game_boy::emulator::memory::GameBoyMemory;
use crate::game_boy::emulator::palettes::{CgbPaletteRegisters, PaletteState};
use crate::game_boy::emulator::rom::{MemoryBankController, RomProperties, RomState};
use crate::game_boy::emulator::runtime::RuntimeControl;
use crate::game_boy::emulator::sgb::SgbState;
use crate::game_boy::emulator::sram::SramState;

const SAVE_STATE_VERSION: u32 = 1;

pub(crate) fn encode_save_state(core: &GameBoyCore) -> Result<Vec<u8>, SaveStateError> {
    serde_json::to_vec(&EmulatorSaveState::from_core(core)).map_err(SaveStateError::Json)
}

pub(crate) fn apply_save_state(core: &mut GameBoyCore, bytes: &[u8]) -> Result<(), SaveStateError> {
    let state: EmulatorSaveState = serde_json::from_slice(bytes).map_err(SaveStateError::Json)?;
    if state.version != SAVE_STATE_VERSION {
        return Err(SaveStateError::UnsupportedVersion(state.version));
    }
    if state.rom.current_rom_id != core.rom.current_rom_id {
        return Err(SaveStateError::RomMismatch {
            expected: core.rom.current_rom_id.clone(),
            actual: state.rom.current_rom_id,
        });
    }

    core.runtime = state.runtime.into();
    core.cpu_registers = state.cpu_registers.into();
    core.cpu_timing = state.cpu_timing.into();
    core.cpu_mode = state.cpu_mode.into();
    core.serial = state.serial.into();
    core.dma = state.dma.into();
    core.gpu_timing = state.gpu_timing.into();
    core.gpu_mode = state.gpu_mode.into();
    core.line_renderer = state.line_renderer.into();
    core.memory_access = state.memory_access.into();
    state.memory.apply(&mut core.memory)?;
    state.palettes.apply(&mut core.palettes)?;
    state
        .cgb_palette_registers
        .apply(&mut core.cgb_palette_registers)?;
    core.rom = state.rom.into();
    state.sram.apply(&mut core.sram)?;
    state.sgb.apply(&mut core.sgb)?;

    core.audio_unit
        .reset_for_rom_load(core.cpu_timing.clock_frequency_hz);
    core.video_frame.reset_for_rom_load();

    Ok(())
}

#[derive(Debug)]
pub(crate) enum SaveStateError {
    Json(serde_json::Error),
    UnsupportedVersion(u32),
    RomMismatch {
        expected: String,
        actual: String,
    },
    InvalidFieldLength {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for SaveStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "{error}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported save-state version {version}")
            }
            Self::RomMismatch { expected, actual } => {
                write!(
                    formatter,
                    "save state is for ROM {actual}, but the loaded ROM is {expected}"
                )
            }
            Self::InvalidFieldLength {
                field,
                expected,
                actual,
            } => {
                write!(
                    formatter,
                    "save-state field {field} has {actual} values, expected {expected}"
                )
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
struct EmulatorSaveState {
    version: u32,
    runtime: RuntimeSnapshot,
    cpu_registers: CpuRegistersSnapshot,
    cpu_timing: CpuTimingSnapshot,
    cpu_mode: CpuModeSnapshot,
    serial: SerialSnapshot,
    #[serde(default)]
    dma: DmaSnapshot,
    gpu_timing: GpuTimingSnapshot,
    gpu_mode: GpuModeSnapshot,
    line_renderer: LineRendererSnapshot,
    memory_access: MemoryAccessSnapshot,
    memory: MemorySnapshot,
    palettes: PaletteSnapshot,
    cgb_palette_registers: CgbPaletteRegistersSnapshot,
    rom: RomSnapshot,
    sram: SramSnapshot,
    sgb: SgbSnapshot,
}

impl EmulatorSaveState {
    fn from_core(core: &GameBoyCore) -> Self {
        Self {
            version: SAVE_STATE_VERSION,
            runtime: RuntimeSnapshot::from(core.runtime),
            cpu_registers: CpuRegistersSnapshot::from(core.cpu_registers),
            cpu_timing: CpuTimingSnapshot::from(core.cpu_timing),
            cpu_mode: CpuModeSnapshot::from(core.cpu_mode),
            serial: SerialSnapshot::from(core.serial),
            dma: DmaSnapshot::from(core.dma),
            gpu_timing: GpuTimingSnapshot::from(core.gpu_timing),
            gpu_mode: GpuModeSnapshot::from(core.gpu_mode),
            line_renderer: LineRendererSnapshot::from(core.line_renderer),
            memory_access: MemoryAccessSnapshot::from(core.memory_access),
            memory: MemorySnapshot::from(&core.memory),
            palettes: PaletteSnapshot::from(core.palettes),
            cgb_palette_registers: CgbPaletteRegistersSnapshot::from(core.cgb_palette_registers),
            rom: RomSnapshot::from(&core.rom),
            sram: SramSnapshot::from(&core.sram),
            sgb: SgbSnapshot::from(&core.sgb),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct RuntimeSnapshot {
    is_running: bool,
    is_paused: bool,
    joypad: JoypadSnapshot,
    joypad_state_changed: bool,
    clock_multiply: i64,
    clock_divide: i64,
    current_clock_multiplier_combo: i32,
}

impl From<RuntimeControl> for RuntimeSnapshot {
    fn from(runtime: RuntimeControl) -> Self {
        Self {
            is_running: runtime.is_running,
            is_paused: runtime.is_paused,
            joypad: JoypadSnapshot::from(runtime.joypad),
            joypad_state_changed: runtime.joypad_state_changed,
            clock_multiply: runtime.clock_multiply,
            clock_divide: runtime.clock_divide,
            current_clock_multiplier_combo: runtime.current_clock_multiplier_combo,
        }
    }
}

impl From<RuntimeSnapshot> for RuntimeControl {
    fn from(snapshot: RuntimeSnapshot) -> Self {
        Self {
            is_running: snapshot.is_running,
            is_paused: snapshot.is_paused,
            joypad: snapshot.joypad.into(),
            joypad_state_changed: snapshot.joypad_state_changed,
            clock_multiply: snapshot.clock_multiply,
            clock_divide: snapshot.clock_divide,
            current_clock_multiplier_combo: snapshot.current_clock_multiplier_combo,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct JoypadSnapshot {
    button: u8,
    direction: u8,
}

impl From<JoypadInputNibbles> for JoypadSnapshot {
    fn from(joypad: JoypadInputNibbles) -> Self {
        Self {
            button: joypad.button,
            direction: joypad.direction,
        }
    }
}

impl From<JoypadSnapshot> for JoypadInputNibbles {
    fn from(snapshot: JoypadSnapshot) -> Self {
        Self {
            button: snapshot.button,
            direction: snapshot.direction,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct CpuRegistersSnapshot {
    pc: u32,
    sp: u32,
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    ime: bool,
    #[serde(default)]
    ime_enable_delay: u8,
    #[serde(default)]
    halt_bug: bool,
}

impl From<CpuRegisters> for CpuRegistersSnapshot {
    fn from(registers: CpuRegisters) -> Self {
        Self {
            pc: registers.pc,
            sp: registers.sp,
            a: registers.a,
            f: registers.f,
            b: registers.b,
            c: registers.c,
            d: registers.d,
            e: registers.e,
            h: registers.h,
            l: registers.l,
            ime: registers.ime,
            ime_enable_delay: registers.ime_enable_delay,
            halt_bug: registers.halt_bug,
        }
    }
}

impl From<CpuRegistersSnapshot> for CpuRegisters {
    fn from(snapshot: CpuRegistersSnapshot) -> Self {
        Self {
            pc: snapshot.pc,
            sp: snapshot.sp,
            a: snapshot.a,
            f: snapshot.f,
            b: snapshot.b,
            c: snapshot.c,
            d: snapshot.d,
            e: snapshot.e,
            h: snapshot.h,
            l: snapshot.l,
            ime: snapshot.ime,
            ime_enable_delay: snapshot.ime_enable_delay,
            halt_bug: snapshot.halt_bug,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct CpuTimingSnapshot {
    clocks_acc: i32,
    clock_frequency_hz: i64,
    #[serde(default)]
    system_counter: u16,
    #[serde(default)]
    tima_reload_delay: u8,
}

impl From<CpuTiming> for CpuTimingSnapshot {
    fn from(timing: CpuTiming) -> Self {
        Self {
            clocks_acc: timing.clocks_acc,
            clock_frequency_hz: timing.clock_frequency_hz,
            system_counter: timing.system_counter,
            tima_reload_delay: timing.tima_reload_delay,
        }
    }
}

impl From<CpuTimingSnapshot> for CpuTiming {
    fn from(snapshot: CpuTimingSnapshot) -> Self {
        Self {
            clocks_acc: snapshot.clocks_acc,
            clock_frequency_hz: snapshot.clock_frequency_hz,
            system_counter: snapshot.system_counter,
            tima_reload_delay: snapshot.tima_reload_delay,
        }
    }
}

#[derive(Deserialize, Serialize)]
enum CpuModeSnapshot {
    Running,
    Halted,
    Stopped,
}

impl From<CpuMode> for CpuModeSnapshot {
    fn from(mode: CpuMode) -> Self {
        match mode {
            CpuMode::Running => Self::Running,
            CpuMode::Halted => Self::Halted,
            CpuMode::Stopped => Self::Stopped,
        }
    }
}

impl From<CpuModeSnapshot> for CpuMode {
    fn from(snapshot: CpuModeSnapshot) -> Self {
        match snapshot {
            CpuModeSnapshot::Running => Self::Running,
            CpuModeSnapshot::Halted => Self::Halted,
            CpuModeSnapshot::Stopped => Self::Stopped,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct SerialSnapshot {
    request: bool,
    is_transferring: bool,
    clock_is_external: bool,
    timer: i32,
}

impl From<SerialState> for SerialSnapshot {
    fn from(serial: SerialState) -> Self {
        Self {
            request: serial.request,
            is_transferring: serial.is_transferring,
            clock_is_external: serial.clock_is_external,
            timer: serial.timer,
        }
    }
}

impl From<SerialSnapshot> for SerialState {
    fn from(snapshot: SerialSnapshot) -> Self {
        Self {
            request: snapshot.request,
            is_transferring: snapshot.is_transferring,
            clock_is_external: snapshot.clock_is_external,
            timer: snapshot.timer,
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
struct DmaSnapshot {
    oam: OamDmaSnapshot,
    vram: VramDmaSnapshot,
}

impl From<DmaState> for DmaSnapshot {
    fn from(dma: DmaState) -> Self {
        Self {
            oam: OamDmaSnapshot::from(dma.oam),
            vram: VramDmaSnapshot::from(dma.vram),
        }
    }
}

impl From<DmaSnapshot> for DmaState {
    fn from(snapshot: DmaSnapshot) -> Self {
        Self {
            oam: snapshot.oam.into(),
            vram: snapshot.vram.into(),
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
struct OamDmaSnapshot {
    pending_source_high: Option<u8>,
    source_high: u8,
    next_index: u8,
    active: bool,
}

impl From<OamDmaState> for OamDmaSnapshot {
    fn from(oam: OamDmaState) -> Self {
        Self {
            pending_source_high: oam.pending_source_high,
            source_high: oam.source_high,
            next_index: oam.next_index,
            active: oam.active,
        }
    }
}

impl From<OamDmaSnapshot> for OamDmaState {
    fn from(snapshot: OamDmaSnapshot) -> Self {
        Self {
            pending_source_high: snapshot.pending_source_high,
            source_high: snapshot.source_high,
            next_index: snapshot.next_index,
            active: snapshot.active,
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
struct VramDmaSnapshot {
    cpu_halt_m_cycles: i32,
}

impl From<VramDmaState> for VramDmaSnapshot {
    fn from(vram: VramDmaState) -> Self {
        Self {
            cpu_halt_m_cycles: vram.cpu_halt_m_cycles,
        }
    }
}

impl From<VramDmaSnapshot> for VramDmaState {
    fn from(snapshot: VramDmaSnapshot) -> Self {
        Self {
            cpu_halt_m_cycles: snapshot.cpu_halt_m_cycles,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct GpuTimingSnapshot {
    clock_factor: i32,
    time_in_mode: i32,
    #[serde(default = "default_line_scan_vram_clocks")]
    line_scan_vram_clocks: i32,
    last_ly_compare: u32,
    blanked_screen: bool,
    need_clear: bool,
}

fn default_line_scan_vram_clocks() -> i32 {
    172
}

impl From<GpuTiming> for GpuTimingSnapshot {
    fn from(timing: GpuTiming) -> Self {
        Self {
            clock_factor: timing.clock_factor,
            time_in_mode: timing.time_in_mode,
            line_scan_vram_clocks: timing.line_scan_vram_clocks,
            last_ly_compare: timing.last_ly_compare,
            blanked_screen: timing.blanked_screen,
            need_clear: timing.need_clear,
        }
    }
}

impl From<GpuTimingSnapshot> for GpuTiming {
    fn from(snapshot: GpuTimingSnapshot) -> Self {
        Self {
            clock_factor: snapshot.clock_factor,
            time_in_mode: snapshot.time_in_mode,
            line_scan_vram_clocks: snapshot.line_scan_vram_clocks,
            last_ly_compare: snapshot.last_ly_compare,
            blanked_screen: snapshot.blanked_screen,
            need_clear: snapshot.need_clear,
        }
    }
}

#[derive(Deserialize, Serialize)]
enum GpuModeSnapshot {
    HBlank,
    VBlank,
    ScanOam,
    ScanVram,
}

impl From<GpuMode> for GpuModeSnapshot {
    fn from(mode: GpuMode) -> Self {
        match mode {
            GpuMode::HBlank => Self::HBlank,
            GpuMode::VBlank => Self::VBlank,
            GpuMode::ScanOam => Self::ScanOam,
            GpuMode::ScanVram => Self::ScanVram,
        }
    }
}

impl From<GpuModeSnapshot> for GpuMode {
    fn from(snapshot: GpuModeSnapshot) -> Self {
        match snapshot {
            GpuModeSnapshot::HBlank => Self::HBlank,
            GpuModeSnapshot::VBlank => Self::VBlank,
            GpuModeSnapshot::ScanOam => Self::ScanOam,
            GpuModeSnapshot::ScanVram => Self::ScanVram,
        }
    }
}

#[derive(Deserialize, Serialize)]
enum LineRendererSnapshot {
    Gb,
    Sgb,
    Cgb,
}

impl From<LineRenderer> for LineRendererSnapshot {
    fn from(renderer: LineRenderer) -> Self {
        match renderer {
            LineRenderer::Gb => Self::Gb,
            LineRenderer::Sgb => Self::Sgb,
            LineRenderer::Cgb => Self::Cgb,
        }
    }
}

impl From<LineRendererSnapshot> for LineRenderer {
    fn from(snapshot: LineRendererSnapshot) -> Self {
        match snapshot {
            LineRendererSnapshot::Gb => Self::Gb,
            LineRendererSnapshot::Sgb => Self::Sgb,
            LineRendererSnapshot::Cgb => Self::Cgb,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct MemoryAccessSnapshot {
    oam: bool,
    vram: bool,
    wram_bank_offset: u32,
    vram_bank_offset: u32,
}

impl From<MemoryAccess> for MemoryAccessSnapshot {
    fn from(access: MemoryAccess) -> Self {
        Self {
            oam: access.oam,
            vram: access.vram,
            wram_bank_offset: access.wram_bank_offset,
            vram_bank_offset: access.vram_bank_offset,
        }
    }
}

impl From<MemoryAccessSnapshot> for MemoryAccess {
    fn from(snapshot: MemoryAccessSnapshot) -> Self {
        Self {
            oam: snapshot.oam,
            vram: snapshot.vram,
            wram_bank_offset: snapshot.wram_bank_offset,
            vram_bank_offset: snapshot.vram_bank_offset,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct MemorySnapshot {
    wram: Vec<u8>,
    vram: Vec<u8>,
    io_ports: Vec<u8>,
    oam: Vec<u8>,
    tile_set: Vec<u32>,
}

impl From<&GameBoyMemory> for MemorySnapshot {
    fn from(memory: &GameBoyMemory) -> Self {
        Self {
            wram: memory.wram.to_vec(),
            vram: memory.vram.to_vec(),
            io_ports: memory.io_ports.to_vec(),
            oam: memory.oam.to_vec(),
            tile_set: memory.tile_set.to_vec(),
        }
    }
}

impl MemorySnapshot {
    fn apply(self, memory: &mut GameBoyMemory) -> Result<(), SaveStateError> {
        copy_u8_slice("memory.wram", &self.wram, &mut memory.wram)?;
        copy_u8_slice("memory.vram", &self.vram, &mut memory.vram)?;
        copy_u8_slice("memory.io_ports", &self.io_ports, &mut memory.io_ports)?;
        copy_u8_slice("memory.oam", &self.oam, &mut memory.oam)?;
        copy_u32_slice("memory.tile_set", &self.tile_set, &mut memory.tile_set)?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
struct PaletteSnapshot {
    translated_bg: Vec<u32>,
    translated_obj: Vec<u32>,
    sgb_translation_bg: Vec<u32>,
    sgb_translation_obj: Vec<u32>,
    cgb_bg: Vec<u32>,
    cgb_obj: Vec<u32>,
}

impl From<PaletteState> for PaletteSnapshot {
    fn from(palettes: PaletteState) -> Self {
        Self {
            translated_bg: palettes.translated_bg.to_vec(),
            translated_obj: palettes.translated_obj.to_vec(),
            sgb_translation_bg: palettes.sgb_translation_bg.to_vec(),
            sgb_translation_obj: palettes.sgb_translation_obj.to_vec(),
            cgb_bg: palettes.cgb_bg.to_vec(),
            cgb_obj: palettes.cgb_obj.to_vec(),
        }
    }
}

impl PaletteSnapshot {
    fn apply(self, palettes: &mut PaletteState) -> Result<(), SaveStateError> {
        copy_u32_slice(
            "palettes.translated_bg",
            &self.translated_bg,
            &mut palettes.translated_bg,
        )?;
        copy_u32_slice(
            "palettes.translated_obj",
            &self.translated_obj,
            &mut palettes.translated_obj,
        )?;
        copy_u32_slice(
            "palettes.sgb_translation_bg",
            &self.sgb_translation_bg,
            &mut palettes.sgb_translation_bg,
        )?;
        copy_u32_slice(
            "palettes.sgb_translation_obj",
            &self.sgb_translation_obj,
            &mut palettes.sgb_translation_obj,
        )?;
        copy_u32_slice("palettes.cgb_bg", &self.cgb_bg, &mut palettes.cgb_bg)?;
        copy_u32_slice("palettes.cgb_obj", &self.cgb_obj, &mut palettes.cgb_obj)?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
struct CgbPaletteRegistersSnapshot {
    bg_data: Vec<u8>,
    bg_index: u32,
    bg_increment: u32,
    obj_data: Vec<u8>,
    obj_index: u32,
    obj_increment: u32,
}

impl From<CgbPaletteRegisters> for CgbPaletteRegistersSnapshot {
    fn from(registers: CgbPaletteRegisters) -> Self {
        Self {
            bg_data: registers.bg_data.to_vec(),
            bg_index: registers.bg_index,
            bg_increment: registers.bg_increment,
            obj_data: registers.obj_data.to_vec(),
            obj_index: registers.obj_index,
            obj_increment: registers.obj_increment,
        }
    }
}

impl CgbPaletteRegistersSnapshot {
    fn apply(self, registers: &mut CgbPaletteRegisters) -> Result<(), SaveStateError> {
        copy_u8_slice(
            "cgb_palette_registers.bg_data",
            &self.bg_data,
            &mut registers.bg_data,
        )?;
        copy_u8_slice(
            "cgb_palette_registers.obj_data",
            &self.obj_data,
            &mut registers.obj_data,
        )?;
        registers.bg_index = self.bg_index;
        registers.bg_increment = self.bg_increment;
        registers.obj_index = self.obj_index;
        registers.obj_increment = self.obj_increment;
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
struct RomSnapshot {
    properties: RomPropertiesSnapshot,
    #[serde(default)]
    fixed_bank_offset: u32,
    bank_offset: u32,
    #[serde(default = "default_mbc1_lower_bank")]
    mbc1_lower_bank: u8,
    #[serde(default)]
    mbc1_upper_bank: u8,
    #[serde(default)]
    suspicious_mbc_warning_logged: bool,
    current_rom_id: String,
    current_opened_file: String,
}

fn default_mbc1_lower_bank() -> u8 {
    1
}

impl From<&RomState> for RomSnapshot {
    fn from(rom: &RomState) -> Self {
        Self {
            properties: RomPropertiesSnapshot::from(rom.properties),
            fixed_bank_offset: rom.fixed_bank_offset,
            bank_offset: rom.bank_offset,
            mbc1_lower_bank: rom.mbc1_lower_bank,
            mbc1_upper_bank: rom.mbc1_upper_bank,
            suspicious_mbc_warning_logged: rom.suspicious_mbc_warning_logged,
            current_rom_id: rom.current_rom_id.clone(),
            current_opened_file: rom.current_opened_file.clone(),
        }
    }
}

impl From<RomSnapshot> for RomState {
    fn from(snapshot: RomSnapshot) -> Self {
        Self {
            properties: snapshot.properties.into(),
            fixed_bank_offset: snapshot.fixed_bank_offset,
            bank_offset: snapshot.bank_offset,
            mbc1_lower_bank: snapshot.mbc1_lower_bank,
            mbc1_upper_bank: snapshot.mbc1_upper_bank,
            suspicious_mbc_warning_logged: snapshot.suspicious_mbc_warning_logged,
            current_rom_id: snapshot.current_rom_id,
            current_opened_file: snapshot.current_opened_file,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct RomPropertiesSnapshot {
    valid: bool,
    title: Vec<u8>,
    mbc: MemoryBankControllerSnapshot,
    cgb_flag: bool,
    sgb_flag: bool,
    has_sram: bool,
    has_rumble: bool,
    size_bytes: i32,
    bank_select_mask: u32,
    mbc_mode: u32,
    cart_type: u32,
    check_sum: u32,
    size_enum: u32,
}

impl From<RomProperties> for RomPropertiesSnapshot {
    fn from(properties: RomProperties) -> Self {
        Self {
            valid: properties.valid,
            title: properties.title.to_vec(),
            mbc: MemoryBankControllerSnapshot::from(properties.mbc),
            cgb_flag: properties.cgb_flag,
            sgb_flag: properties.sgb_flag,
            has_sram: properties.has_sram,
            has_rumble: properties.has_rumble,
            size_bytes: properties.size_bytes,
            bank_select_mask: properties.bank_select_mask,
            mbc_mode: properties.mbc_mode,
            cart_type: properties.cart_type,
            check_sum: properties.check_sum,
            size_enum: properties.size_enum,
        }
    }
}

impl From<RomPropertiesSnapshot> for RomProperties {
    fn from(snapshot: RomPropertiesSnapshot) -> Self {
        let mut title = [0; 17];
        let copy_len = snapshot.title.len().min(title.len());
        title[..copy_len].copy_from_slice(&snapshot.title[..copy_len]);
        Self {
            valid: snapshot.valid,
            title,
            mbc: snapshot.mbc.into(),
            cgb_flag: snapshot.cgb_flag,
            sgb_flag: snapshot.sgb_flag,
            has_sram: snapshot.has_sram,
            has_rumble: snapshot.has_rumble,
            size_bytes: snapshot.size_bytes,
            bank_select_mask: snapshot.bank_select_mask,
            mbc_mode: snapshot.mbc_mode,
            cart_type: snapshot.cart_type,
            check_sum: snapshot.check_sum,
            size_enum: snapshot.size_enum,
        }
    }
}

#[derive(Deserialize, Serialize)]
enum MemoryBankControllerSnapshot {
    None,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc4,
    Mbc5,
    Mmm01,
}

impl From<MemoryBankController> for MemoryBankControllerSnapshot {
    fn from(controller: MemoryBankController) -> Self {
        match controller {
            MemoryBankController::None => Self::None,
            MemoryBankController::Mbc1 => Self::Mbc1,
            MemoryBankController::Mbc2 => Self::Mbc2,
            MemoryBankController::Mbc3 => Self::Mbc3,
            MemoryBankController::Mbc4 => Self::Mbc4,
            MemoryBankController::Mbc5 => Self::Mbc5,
            MemoryBankController::Mmm01 => Self::Mmm01,
        }
    }
}

impl From<MemoryBankControllerSnapshot> for MemoryBankController {
    fn from(snapshot: MemoryBankControllerSnapshot) -> Self {
        match snapshot {
            MemoryBankControllerSnapshot::None => Self::None,
            MemoryBankControllerSnapshot::Mbc1 => Self::Mbc1,
            MemoryBankControllerSnapshot::Mbc2 => Self::Mbc2,
            MemoryBankControllerSnapshot::Mbc3 => Self::Mbc3,
            MemoryBankControllerSnapshot::Mbc4 => Self::Mbc4,
            MemoryBankControllerSnapshot::Mbc5 => Self::Mbc5,
            MemoryBankControllerSnapshot::Mmm01 => Self::Mmm01,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct SramSnapshot {
    data: Vec<u8>,
    has_battery: bool,
    has_timer: bool,
    timer_data: Vec<u8>,
    timer_mode: u32,
    timer_latch: u32,
    bank_offset: u32,
    size_enum: u8,
    size_bytes: u32,
    bank_select_mask: u8,
    enable_flag: bool,
    dirty: bool,
}

impl From<&SramState> for SramSnapshot {
    fn from(sram: &SramState) -> Self {
        Self {
            data: sram.data.to_vec(),
            has_battery: sram.has_battery,
            has_timer: sram.has_timer,
            timer_data: sram.timer_data.to_vec(),
            timer_mode: sram.timer_mode,
            timer_latch: sram.timer_latch,
            bank_offset: sram.bank_offset,
            size_enum: sram.size_enum,
            size_bytes: sram.size_bytes,
            bank_select_mask: sram.bank_select_mask,
            enable_flag: sram.enable_flag,
            dirty: sram.is_dirty(),
        }
    }
}

impl SramSnapshot {
    fn apply(self, sram: &mut SramState) -> Result<(), SaveStateError> {
        copy_u8_slice("sram.data", &self.data, &mut sram.data)?;
        copy_u8_slice("sram.timer_data", &self.timer_data, &mut sram.timer_data)?;
        sram.has_battery = self.has_battery;
        sram.has_timer = self.has_timer;
        sram.timer_mode = self.timer_mode;
        sram.timer_latch = self.timer_latch;
        sram.bank_offset = self.bank_offset;
        sram.size_enum = self.size_enum;
        sram.size_bytes = self.size_bytes;
        sram.bank_select_mask = self.bank_select_mask;
        sram.enable_flag = self.enable_flag;
        sram.set_dirty(self.dirty || self.has_battery);
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
struct SgbSnapshot {
    reading_command: bool,
    #[serde(default)]
    awaiting_stop_bit: bool,
    #[serde(default)]
    stop_bit_received: bool,
    command_bytes: Vec<u32>,
    command_bits: Vec<u8>,
    command: u32,
    read_command_bits: i32,
    read_command_bytes: i32,
    freeze_screen: bool,
    freeze_mode: u32,
    mult_enabled: bool,
    player_count: u32,
    packets_sent: u32,
    packets_to_send: u32,
    read_joypad_id: u32,
    mono_data: Vec<u32>,
    transfer_vram: Vec<u8>,
    palettes: Vec<u32>,
    system_palettes: Vec<u32>,
    character_palettes: Vec<u32>,
    #[serde(default)]
    attribute_files: Vec<u32>,
}

impl From<&SgbState> for SgbSnapshot {
    fn from(sgb: &SgbState) -> Self {
        Self {
            reading_command: sgb.reading_command,
            awaiting_stop_bit: sgb.awaiting_stop_bit,
            stop_bit_received: sgb.stop_bit_received,
            command_bytes: sgb
                .command_bytes
                .iter()
                .flat_map(|packet| packet.iter().copied())
                .collect(),
            command_bits: sgb.command_bits.to_vec(),
            command: sgb.command,
            read_command_bits: sgb.read_command_bits,
            read_command_bytes: sgb.read_command_bytes,
            freeze_screen: sgb.freeze_screen,
            freeze_mode: sgb.freeze_mode,
            mult_enabled: sgb.mult_enabled,
            player_count: sgb.player_count,
            packets_sent: sgb.packets_sent,
            packets_to_send: sgb.packets_to_send,
            read_joypad_id: sgb.read_joypad_id,
            mono_data: sgb.mono_data.to_vec(),
            transfer_vram: sgb.transfer_vram.to_vec(),
            palettes: sgb.palettes.to_vec(),
            system_palettes: sgb.system_palettes.to_vec(),
            character_palettes: sgb.character_palettes.to_vec(),
            attribute_files: sgb.attribute_files.to_vec(),
        }
    }
}

impl SgbSnapshot {
    fn apply(self, sgb: &mut SgbState) -> Result<(), SaveStateError> {
        if self.command_bytes.len() != 7 * 16 {
            return Err(SaveStateError::InvalidFieldLength {
                field: "sgb.command_bytes",
                expected: 7 * 16,
                actual: self.command_bytes.len(),
            });
        }
        for (index, value) in self.command_bytes.into_iter().enumerate() {
            let packet = index / 16;
            let byte = index % 16;
            sgb.command_bytes[packet][byte] = value;
        }
        copy_u8_slice(
            "sgb.command_bits",
            &self.command_bits,
            &mut sgb.command_bits,
        )?;
        copy_u32_slice("sgb.mono_data", &self.mono_data, &mut sgb.mono_data)?;
        copy_u8_slice(
            "sgb.transfer_vram",
            &self.transfer_vram,
            &mut sgb.transfer_vram,
        )?;
        copy_u32_slice("sgb.palettes", &self.palettes, &mut sgb.palettes)?;
        copy_u32_slice(
            "sgb.system_palettes",
            &self.system_palettes,
            &mut sgb.system_palettes,
        )?;
        copy_u32_slice(
            "sgb.character_palettes",
            &self.character_palettes,
            &mut sgb.character_palettes,
        )?;
        if self.attribute_files.is_empty() {
            sgb.attribute_files.fill(0);
        } else {
            copy_u32_slice(
                "sgb.attribute_files",
                &self.attribute_files,
                &mut sgb.attribute_files,
            )?;
        }

        sgb.reading_command = self.reading_command;
        sgb.awaiting_stop_bit = self.awaiting_stop_bit;
        sgb.stop_bit_received = self.stop_bit_received;
        sgb.command = self.command;
        sgb.read_command_bits = self.read_command_bits;
        sgb.read_command_bytes = self.read_command_bytes;
        sgb.freeze_screen = self.freeze_screen;
        sgb.freeze_mode = self.freeze_mode;
        sgb.mult_enabled = self.mult_enabled;
        sgb.player_count = self.player_count;
        sgb.packets_sent = self.packets_sent;
        sgb.packets_to_send = self.packets_to_send;
        sgb.read_joypad_id = self.read_joypad_id;
        Ok(())
    }
}

fn copy_u8_slice(
    field: &'static str,
    source: &[u8],
    destination: &mut [u8],
) -> Result<(), SaveStateError> {
    if source.len() != destination.len() {
        return Err(SaveStateError::InvalidFieldLength {
            field,
            expected: destination.len(),
            actual: source.len(),
        });
    }
    destination.copy_from_slice(source);
    Ok(())
}

fn copy_u32_slice(
    field: &'static str,
    source: &[u32],
    destination: &mut [u32],
) -> Result<(), SaveStateError> {
    if source.len() != destination.len() {
        return Err(SaveStateError::InvalidFieldLength {
            field,
            expected: destination.len(),
            actual: source.len(),
        });
    }
    destination.copy_from_slice(source);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_state_round_trips_representative_core_state() {
        let mut core = GameBoyCore::default();
        core.rom.current_rom_id = "rom-id".to_string();
        core.runtime.is_running = true;
        core.cpu_registers.pc = 0x1234;
        core.cpu_registers.a = 0xab;
        core.cpu_registers.ime_enable_delay = 2;
        core.cpu_registers.halt_bug = true;
        core.cpu_timing.system_counter = 0x3456;
        core.cpu_timing.tima_reload_delay = 1;
        core.dma.oam.pending_source_high = Some(0xc0);
        core.dma.oam.source_high = 0xd0;
        core.dma.oam.next_index = 0x20;
        core.dma.oam.active = true;
        core.dma.vram.cpu_halt_m_cycles = 3;
        core.gpu_timing.line_scan_vram_clocks = 180;
        core.rom.fixed_bank_offset = 0x8000;
        core.rom.bank_offset = 0xc000;
        core.rom.mbc1_lower_bank = 3;
        core.rom.mbc1_upper_bank = 2;
        core.rom.suspicious_mbc_warning_logged = true;
        core.memory.wram[0x10] = 0x42;
        core.memory.vram[0x20] = 0x24;
        core.sram.data[0x30] = 0x99;
        core.sram.has_battery = true;
        core.sgb.attribute_files[0x40] = 3;

        let bytes = encode_save_state(&core).expect("save state should encode");
        core.cpu_registers.pc = 0;
        core.cpu_registers.a = 0;
        core.cpu_registers.ime_enable_delay = 0;
        core.cpu_registers.halt_bug = false;
        core.cpu_timing.system_counter = 0;
        core.cpu_timing.tima_reload_delay = 0;
        core.dma.reset_for_rom_load();
        core.gpu_timing.line_scan_vram_clocks = 0;
        core.rom.fixed_bank_offset = 0;
        core.rom.bank_offset = 0;
        core.rom.mbc1_lower_bank = 1;
        core.rom.mbc1_upper_bank = 0;
        core.rom.suspicious_mbc_warning_logged = false;
        core.memory.wram[0x10] = 0;
        core.memory.vram[0x20] = 0;
        core.sram.data[0x30] = 0;
        core.sgb.attribute_files[0x40] = 0;

        apply_save_state(&mut core, &bytes).expect("save state should restore");

        assert_eq!(core.cpu_registers.pc, 0x1234);
        assert_eq!(core.cpu_registers.a, 0xab);
        assert_eq!(core.cpu_registers.ime_enable_delay, 2);
        assert!(core.cpu_registers.halt_bug);
        assert_eq!(core.cpu_timing.system_counter, 0x3456);
        assert_eq!(core.cpu_timing.tima_reload_delay, 1);
        assert_eq!(core.dma.oam.pending_source_high, Some(0xc0));
        assert_eq!(core.dma.oam.source_high, 0xd0);
        assert_eq!(core.dma.oam.next_index, 0x20);
        assert!(core.dma.oam.active);
        assert_eq!(core.dma.vram.cpu_halt_m_cycles, 3);
        assert_eq!(core.gpu_timing.line_scan_vram_clocks, 180);
        assert_eq!(core.rom.fixed_bank_offset, 0x8000);
        assert_eq!(core.rom.bank_offset, 0xc000);
        assert_eq!(core.rom.mbc1_lower_bank, 3);
        assert_eq!(core.rom.mbc1_upper_bank, 2);
        assert!(core.rom.suspicious_mbc_warning_logged);
        assert_eq!(core.memory.wram[0x10], 0x42);
        assert_eq!(core.memory.vram[0x20], 0x24);
        assert_eq!(core.sram.data[0x30], 0x99);
        assert!(core.sram.is_dirty());
        assert_eq!(core.sgb.attribute_files[0x40], 3);
    }
}
