use bevy::prelude::warn;

use crate::game_boy::emulator::GameBoyCore;
use crate::game_boy::emulator::constants::ROM_BANK_SIZE;
use crate::game_boy::emulator::cpu::CpuMode;
use crate::game_boy::emulator::gpu::GpuMode;
use crate::game_boy::emulator::input::{
    JOYP_LOW_NIBBLE_MASK, JOYP_SELECT_BUTTONS, JOYP_SELECT_DIRECTIONS, JOYP_SELECT_MASK,
    JOYP_SELECT_NONE, joypad_low_nibble_falling_edge,
};
use crate::game_boy::emulator::rom::MemoryBankController;

const JOYP_IO_INDEX: usize = 0x00;
const SB_IO_INDEX: usize = 0x01;
const SC_IO_INDEX: usize = 0x02;
const DIV_IO_INDEX: usize = 0x04;
const TIMA_IO_INDEX: usize = 0x05;
const TMA_IO_INDEX: usize = 0x06;
const TAC_IO_INDEX: usize = 0x07;
const IF_IO_INDEX: usize = 0x0f;
const LCDC_IO_INDEX: usize = 0x40;
const STAT_IO_INDEX: usize = 0x41;
const LY_IO_INDEX: usize = 0x44;
const DMA_IO_INDEX: usize = 0x46;
const BGP_IO_INDEX: usize = 0x47;
const OBP0_IO_INDEX: usize = 0x48;
const OBP1_IO_INDEX: usize = 0x49;
const KEY1_IO_INDEX: usize = 0x4d;
const VBK_IO_INDEX: usize = 0x4f;
const BOOT_IO_INDEX: usize = 0x50;
const HDMA1_IO_INDEX: usize = 0x51;
const HDMA2_IO_INDEX: usize = 0x52;
const HDMA3_IO_INDEX: usize = 0x53;
const HDMA4_IO_INDEX: usize = 0x54;
const HDMA5_IO_INDEX: usize = 0x55;
const BCPS_IO_INDEX: usize = 0x68;
const BCPD_IO_INDEX: usize = 0x69;
const OCPS_IO_INDEX: usize = 0x6a;
const OCPD_IO_INDEX: usize = 0x6b;
const SVBK_IO_INDEX: usize = 0x70;
const NR52_IO_INDEX: usize = 0x26;

const INTERRUPT_JOYPAD: u8 = 0x10;
const INTERRUPT_TIMER: u8 = 0x04;
const TIMER_RELOAD_DELAY_M_CYCLES: u8 = 1;
const OAM_DMA_BYTES: u8 = 160;
const HDMA_BLOCK_BYTES: usize = 16;
const HDMA_BLOCK_M_CYCLES: i32 = 8;

pub(crate) fn step_system_counter(core: &mut GameBoyCore, clocks: u16) {
    let machine_cycles = clocks / 4;
    let remainder = clocks % 4;

    for _ in 0..machine_cycles {
        finish_pending_timer_reload(core);
        advance_system_counter(core, 4);
    }
    if remainder != 0 {
        advance_system_counter(core, remainder);
    }
}

pub(crate) fn read8(core: &GameBoyCore, address: u16) -> u8 {
    if oam_dma_blocks_cpu_access(core, address) {
        return 0xff;
    }
    let value = read8_unrestricted(core, address);
    core.cheats.read_patch(address, value).unwrap_or(value)
}

fn read8_unrestricted(core: &GameBoyCore, address: u16) -> u8 {
    let address = usize::from(address);
    if address < 0x4000 {
        let bank_offset = usize::try_from(core.rom.fixed_bank_offset).unwrap_or_default();
        return core
            .memory
            .rom
            .get(bank_offset + address)
            .copied()
            .unwrap_or(0xff);
    }
    if address < 0x8000 {
        let bank_offset = usize::try_from(core.rom.bank_offset).unwrap_or_default();
        return core
            .memory
            .rom
            .get(bank_offset + (address & 0x3fff))
            .copied()
            .unwrap_or(0xff);
    }
    if address < 0xa000 {
        if core.memory_access.vram {
            let bank_offset =
                usize::try_from(core.memory_access.vram_bank_offset).unwrap_or_default();
            return core
                .memory
                .vram
                .get(bank_offset + (address & 0x1fff))
                .copied()
                .unwrap_or(0xff);
        }
        return 0xff;
    }
    if address < 0xc000 {
        return read_sram(core, address);
    }
    if address < 0xd000 {
        return core
            .memory
            .wram
            .get(address & 0x0fff)
            .copied()
            .unwrap_or(0xff);
    }
    if address < 0xe000 {
        let offset = usize::try_from(core.memory_access.wram_bank_offset).unwrap_or(0x1000);
        return core
            .memory
            .wram
            .get(offset + (address & 0x0fff))
            .copied()
            .unwrap_or(0xff);
    }
    if address < 0xf000 {
        return core
            .memory
            .wram
            .get(address & 0x0fff)
            .copied()
            .unwrap_or(0xff);
    }
    if address < 0xfe00 {
        let offset = usize::try_from(core.memory_access.wram_bank_offset).unwrap_or(0x1000);
        return core
            .memory
            .wram
            .get(offset + (address & 0x0fff))
            .copied()
            .unwrap_or(0xff);
    }
    if address < 0xfea0 {
        if core.memory_access.oam {
            return core.memory.oam[(address & 0x00ff) % core.memory.oam.len()];
        }
        return 0xff;
    }
    if address < 0xff00 {
        return 0xff;
    }
    if address < 0xff80 {
        return read_io(core, address & 0x7f);
    }
    core.memory
        .io_ports
        .get(address & 0xff)
        .copied()
        .unwrap_or(0xff)
}

pub(crate) fn read16(core: &GameBoyCore, address: u16) -> u16 {
    u16::from(read8(core, address)) | (u16::from(read8(core, address.wrapping_add(1))) << 8)
}

pub(crate) fn write8(core: &mut GameBoyCore, address: u16, value: u8) {
    if oam_dma_blocks_cpu_access(core, address) {
        return;
    }
    write8_unrestricted(core, address, value);
}

fn write8_unrestricted(core: &mut GameBoyCore, address: u16, value: u8) {
    let address_usize = usize::from(address);
    if address_usize < 0x8000 {
        write_mbc(core, address, value);
    } else if address_usize < 0xa000 {
        if core.memory_access.vram {
            core.memory
                .write_vram(core.memory_access.vram_bank_offset, address, value);
        }
    } else if address_usize < 0xc000 {
        write_sram(core, address_usize, value);
    } else if address_usize < 0xd000 {
        if let Some(slot) = core.memory.wram.get_mut(address_usize & 0x0fff) {
            *slot = value;
        }
    } else if address_usize < 0xe000 {
        let offset = usize::try_from(core.memory_access.wram_bank_offset).unwrap_or(0x1000);
        if let Some(slot) = core.memory.wram.get_mut(offset + (address_usize & 0x0fff)) {
            *slot = value;
        }
    } else if address_usize < 0xf000 {
        if let Some(slot) = core.memory.wram.get_mut(address_usize & 0x0fff) {
            *slot = value;
        }
    } else if address_usize < 0xfe00 {
        let offset = usize::try_from(core.memory_access.wram_bank_offset).unwrap_or(0x1000);
        if let Some(slot) = core.memory.wram.get_mut(offset + (address_usize & 0x0fff)) {
            *slot = value;
        }
    } else if address_usize < 0xfea0 {
        if core.memory_access.oam {
            core.memory.oam[(address_usize & 0x00ff) % core.memory.oam.len()] = value;
        }
    } else if address_usize < 0xff00 {
    } else if address_usize < 0xff80 {
        write_io(core, address_usize & 0x7f, value);
    } else {
        core.memory.write_io_port(address_usize & 0xff, value);
    }
}

pub(crate) fn write16(core: &mut GameBoyCore, address: u16, value: u16) {
    write8(core, address, (value & 0x00ff) as u8);
    write8(core, address.wrapping_add(1), (value >> 8) as u8);
}

pub(crate) fn run_hblank_dma(core: &mut GameBoyCore) {
    if core.cpu_mode == CpuMode::Halted {
        return;
    }
    if !hblank_dma_active(core) {
        return;
    }

    let remaining = core.memory.io_ports[HDMA5_IO_INDEX] & 0x7f;
    transfer_hdma_block(core, HDMA_BLOCK_BYTES);
    add_vram_dma_cpu_halt(core, 1);
    if remaining == 0 {
        core.memory.write_io_port(HDMA5_IO_INDEX, 0xff);
    } else {
        core.memory.write_io_port(HDMA5_IO_INDEX, remaining - 1);
    }
}

pub(crate) fn begin_deferred_oam_dma(core: &mut GameBoyCore) {
    let Some(source_high) = core.dma.oam.pending_source_high.take() else {
        return;
    };

    core.dma.oam.source_high = source_high;
    core.dma.oam.next_index = 0;
    core.dma.oam.active = true;
}

pub(crate) fn step_oam_dma(core: &mut GameBoyCore) {
    if !core.dma.oam.active {
        return;
    }

    let index = core.dma.oam.next_index;
    let source = (u16::from(core.dma.oam.source_high) << 8) | u16::from(index);
    let byte = read8_unrestricted(core, source);
    if let Some(slot) = core.memory.oam.get_mut(usize::from(index)) {
        *slot = byte;
    }

    core.dma.oam.next_index = core.dma.oam.next_index.saturating_add(1);
    if core.dma.oam.next_index >= OAM_DMA_BYTES {
        core.dma.oam.active = false;
    }
}

fn read_sram(core: &GameBoyCore, address: usize) -> u8 {
    if !core.sram.enable_flag {
        return 0x00;
    }
    if core.sram.has_timer && core.sram.timer_mode > 0 {
        let index = usize::try_from(core.sram.timer_mode.saturating_sub(0x08)).unwrap_or_default();
        return core.sram.read_timer_data(index);
    }
    core.sram.read_data(address)
}

fn write_sram(core: &mut GameBoyCore, address: usize, mut value: u8) {
    if !core.sram.enable_flag {
        return;
    }
    if core.sram.has_timer && core.sram.timer_mode > 0 {
        let index = usize::try_from(core.sram.timer_mode.saturating_sub(0x08)).unwrap_or_default();
        core.sram.write_timer_data(index, value);
        return;
    }
    if core.rom.properties.mbc == MemoryBankController::Mbc2 {
        value &= 0x0f;
    }
    core.sram.write_data(address, value);
}

fn write_mbc(core: &mut GameBoyCore, address: u16, mut value: u8) {
    let address = u32::from(address);
    match core.rom.properties.mbc {
        MemoryBankController::None => {}
        MemoryBankController::Mbc1 => write_mbc1(core, address, value),
        MemoryBankController::Mbc2 => write_mbc2(core, address, value),
        MemoryBankController::Mbc3 => write_mbc3(core, address, value),
        MemoryBankController::Mbc5 => write_mbc5(core, address, value),
        MemoryBankController::Mbc4 | MemoryBankController::Mmm01 => {
            value &= 0x0f;
            if address < 0x2000 {
                core.sram.enable_flag = value == 0x0a;
            }
        }
    }
}

fn write_mbc1(core: &mut GameBoyCore, address: u32, mut value: u8) {
    if address < 0x2000 {
        value &= 0x0f;
        core.sram.enable_flag = value == 0x0a;
    } else if address < 0x4000 {
        warn_if_mbc1_write_looks_like_full_width_bank_select(core, address, value);
        value &= 0x1f;
        if value == 0 {
            value = 1;
        }
        core.rom.mbc1_lower_bank = value;
        update_mbc1_bank_offsets(core);
    } else if address < 0x6000 {
        value &= 0x03;
        core.rom.mbc1_upper_bank = value;
        update_mbc1_bank_offsets(core);
    } else {
        core.rom.properties.mbc_mode = u32::from(value & 0x01);
        update_mbc1_bank_offsets(core);
    }
}

fn warn_if_mbc1_write_looks_like_full_width_bank_select(
    core: &mut GameBoyCore,
    address: u32,
    value: u8,
) {
    if core.rom.suspicious_mbc_warning_logged {
        return;
    }
    if core.rom.properties.bank_select_mask <= 0x1f || value & 0xe0 == 0 {
        return;
    }

    core.rom.suspicious_mbc_warning_logged = true;
    warn!(
        "suspicious Game Boy MBC access: ROM header selects MBC1, but write {value:02x} to {address:04x} includes bank bits ignored by MBC1; this ROM may require a different mapper"
    );
}

fn update_mbc1_bank_offsets(core: &mut GameBoyCore) {
    let lower_bank = u32::from(core.rom.mbc1_lower_bank.max(1));
    let upper_bank = u32::from(core.rom.mbc1_upper_bank);
    let bank_mask = core.rom.properties.bank_select_mask;

    let switchable_bank = (upper_bank << 5) | lower_bank;
    core.rom.bank_offset = (switchable_bank & bank_mask) * ROM_BANK_SIZE;

    let fixed_bank = if core.rom.properties.mbc_mode == 0 {
        0
    } else {
        upper_bank << 5
    };
    core.rom.fixed_bank_offset = (fixed_bank & bank_mask) * ROM_BANK_SIZE;

    core.sram.bank_offset = if core.rom.properties.mbc_mode == 0 {
        0
    } else {
        upper_bank * 0x2000
    };
}

fn write_mbc2(core: &mut GameBoyCore, address: u32, mut value: u8) {
    if address >= 0x4000 {
        return;
    }
    if address & 0x0100 == 0 {
        value &= 0x0f;
        core.sram.enable_flag = value == 0x0a;
    } else {
        value &= 0x0f;
        value &= core.rom.properties.bank_select_mask as u8;
        if value == 0 {
            value = 1;
        }
        core.rom.bank_offset = u32::from(value) * ROM_BANK_SIZE;
    }
}

fn write_mbc3(core: &mut GameBoyCore, address: u32, mut value: u8) {
    if address < 0x2000 {
        value &= 0x0f;
        core.sram.enable_flag = value == 0x0a;
    } else if address < 0x4000 {
        value &= core.rom.properties.bank_select_mask as u8;
        if value == 0 {
            value = 1;
        }
        core.rom.bank_offset = u32::from(value) * ROM_BANK_SIZE;
    } else if address < 0x6000 {
        value &= 0x0f;
        if value < 0x08 {
            core.sram.bank_offset = u32::from(value) * 0x2000;
            core.sram.timer_mode = 0;
        } else if (0x08..0x0d).contains(&value) {
            core.sram.timer_mode = u32::from(value);
        } else {
            core.sram.timer_mode = 0;
        }
    } else {
        core.sram.latch_timer_data(value);
    }
}

fn write_mbc5(core: &mut GameBoyCore, address: u32, mut value: u8) {
    if address < 0x2000 {
        value &= 0x0f;
        core.sram.enable_flag = value == 0x0a;
    } else if address < 0x3000 {
        let masked = u32::from(value) & core.rom.properties.bank_select_mask;
        core.rom.bank_offset &= 0x0040_0000;
        core.rom.bank_offset |= masked * ROM_BANK_SIZE;
    } else if address < 0x4000 {
        value &= 0x01;
        core.rom.bank_offset &= 0x003f_c000;
        if value != 0 {
            core.rom.bank_offset |= 0x0040_0000;
        }
        mask_bank_offset(core);
    } else if address < 0x6000 {
        value &= 0x0f;
        core.sram.bank_offset = u32::from(value) * 0x2000;
    }
}

fn mask_bank_offset(core: &mut GameBoyCore) {
    core.rom.bank_offset &= core.rom.properties.bank_select_mask * ROM_BANK_SIZE;
}

fn read_io(core: &GameBoyCore, index: usize) -> u8 {
    match index {
        JOYP_IO_INDEX => read_joyp(core),
        0x11 | 0x16 => core.memory.io_ports[index] & 0xc0,
        0x13 | 0x18 | 0x1d => 0,
        0x14 | 0x19 | 0x1e | 0x23 => core.memory.io_ports[index] & 0x40,
        BCPD_IO_INDEX if core.rom.properties.cgb_flag => core
            .cgb_palette_registers
            .bg_data
            .get(core.cgb_palette_registers.bg_index as usize)
            .copied()
            .unwrap_or(0),
        OCPD_IO_INDEX if core.rom.properties.cgb_flag => core
            .cgb_palette_registers
            .obj_data
            .get(core.cgb_palette_registers.obj_index as usize)
            .copied()
            .unwrap_or(0),
        BCPD_IO_INDEX | OCPD_IO_INDEX => 0,
        BOOT_IO_INDEX => core.memory.io_ports[BOOT_IO_INDEX] & 0x01,
        _ => core.memory.io_ports.get(index).copied().unwrap_or(0xff),
    }
}

fn write_io(core: &mut GameBoyCore, index: usize, value: u8) {
    match index {
        JOYP_IO_INDEX => write_joyp(core, value),
        SB_IO_INDEX => {
            if !core.serial.is_transferring || core.serial.clock_is_external {
                core.memory.write_io_port(SB_IO_INDEX, value);
            }
        }
        SC_IO_INDEX => write_serial_control(core, value),
        DIV_IO_INDEX => write_divider(core),
        TIMA_IO_INDEX => write_timer_counter(core, value),
        TMA_IO_INDEX => write_timer_modulo(core, value),
        TAC_IO_INDEX => write_timer_control(core, value),
        0x10..=0x26 | 0x30..=0x3f => write_audio_register(core, index, value),
        LCDC_IO_INDEX => write_lcd_control(core, value),
        STAT_IO_INDEX => {
            let stat = core.memory.io_ports[STAT_IO_INDEX] & 0x07;
            core.memory
                .write_io_port(STAT_IO_INDEX, stat | (value & 0x78));
        }
        LY_IO_INDEX => {}
        DMA_IO_INDEX => start_oam_dma(core, value),
        BGP_IO_INDEX => {
            core.memory.write_io_port(BGP_IO_INDEX, value);
            core.palettes.translate_bg(value);
        }
        OBP0_IO_INDEX => {
            core.memory.write_io_port(OBP0_IO_INDEX, value);
            core.palettes.translate_obj1(value);
        }
        OBP1_IO_INDEX => {
            core.memory.write_io_port(OBP1_IO_INDEX, value);
            core.palettes.translate_obj2(value);
        }
        KEY1_IO_INDEX => write_key1(core, value),
        VBK_IO_INDEX => write_vram_bank(core, value),
        HDMA1_IO_INDEX => core.memory.write_io_port(HDMA1_IO_INDEX, value),
        HDMA2_IO_INDEX => core.memory.write_io_port(HDMA2_IO_INDEX, value & 0xf0),
        HDMA3_IO_INDEX => core.memory.write_io_port(HDMA3_IO_INDEX, value & 0x1f),
        HDMA4_IO_INDEX => core.memory.write_io_port(HDMA4_IO_INDEX, value & 0xf0),
        BOOT_IO_INDEX => write_boot_lock(core, value),
        HDMA5_IO_INDEX => write_hdma5(core, value),
        BCPS_IO_INDEX => write_cgb_bg_palette_index(core, value),
        BCPD_IO_INDEX => write_cgb_bg_palette_data(core, value),
        OCPS_IO_INDEX => write_cgb_obj_palette_index(core, value),
        OCPD_IO_INDEX => write_cgb_obj_palette_data(core, value),
        SVBK_IO_INDEX => write_wram_bank(core, value),
        _ => core.memory.write_io_port(index, value),
    }
}

fn write_audio_register(core: &mut GameBoyCore, index: usize, value: u8) {
    if index == NR52_IO_INDEX {
        core.memory.write_io_port(
            NR52_IO_INDEX,
            (value & 0x80) | 0x70 | core.audio_unit.channel_status_bits(),
        );
    } else {
        core.memory.write_io_port(index, value);
    }

    core.audio_unit
        .write_register(index, value, &core.memory.io_ports);
    core.memory.write_io_port(
        NR52_IO_INDEX,
        audio_status_register_value(&core.audio_unit, &core.memory.io_ports),
    );
}

fn audio_status_register_value(
    audio_unit: &crate::game_boy::emulator::audio_unit::AudioUnitState,
    io_ports: &[u8],
) -> u8 {
    let global_enable = io_ports.get(NR52_IO_INDEX).copied().unwrap_or_default() & 0x80;
    global_enable | 0x70 | audio_unit.channel_status_bits()
}

fn write_joyp(core: &mut GameBoyCore, value: u8) {
    let old_joyp = core.memory.io_ports[JOYP_IO_INDEX];
    let old_select = old_joyp & JOYP_SELECT_MASK;
    let select = value & JOYP_SELECT_MASK;
    if core.rom.properties.sgb_flag {
        if select == 0x00 {
            core.sgb.begin_command();
        } else if select == JOYP_SELECT_DIRECTIONS && old_select != select {
            core.sgb.receive_command_bit(0);
        } else if select == JOYP_SELECT_BUTTONS && old_select != select {
            core.sgb.receive_command_bit(1);
        } else if select == JOYP_SELECT_NONE {
            if core.sgb.reading_command {
                core.sgb.finish_packet_if_ready(&core.memory);
            } else if core.sgb.mult_enabled && old_select < JOYP_SELECT_NONE {
                core.sgb.read_joypad_id = core.sgb.read_joypad_id.saturating_sub(1);
                if core.sgb.read_joypad_id < 0x0c {
                    core.sgb.read_joypad_id = 0x0f;
                }
            }
        }
        let new_joyp = joyp_value(core, select);
        core.memory.write_io_port(JOYP_IO_INDEX, new_joyp);
        request_joypad_interrupt_on_falling_edge(core, old_joyp, new_joyp);
        return;
    }

    let new_joyp = joyp_value(core, select);
    core.memory.write_io_port(JOYP_IO_INDEX, new_joyp);
    request_joypad_interrupt_on_falling_edge(core, old_joyp, new_joyp);
}

fn read_joyp(core: &GameBoyCore) -> u8 {
    let select = core.memory.io_ports[JOYP_IO_INDEX] & JOYP_SELECT_MASK;
    if core.rom.properties.sgb_flag && core.sgb.mult_enabled && select == JOYP_SELECT_NONE {
        return (core.sgb.read_joypad_id as u8) & JOYP_LOW_NIBBLE_MASK;
    }

    joyp_value(core, select)
}

fn joyp_value(core: &GameBoyCore, select: u8) -> u8 {
    0xc0 | (select & JOYP_SELECT_MASK) | joyp_low_nibble(core, select)
}

fn joyp_low_nibble(core: &GameBoyCore, select: u8) -> u8 {
    core.runtime.joypad.low_nibble_for_select(select)
}

fn request_joypad_interrupt_on_falling_edge(core: &mut GameBoyCore, old_joyp: u8, new_joyp: u8) {
    if !joypad_low_nibble_falling_edge(old_joyp, new_joyp) {
        return;
    }

    let interrupt_flags = core.memory.io_ports[IF_IO_INDEX] | INTERRUPT_JOYPAD;
    core.memory.write_io_port(IF_IO_INDEX, interrupt_flags);
}

fn write_boot_lock(core: &mut GameBoyCore, value: u8) {
    let boot_off = core.memory.io_ports[BOOT_IO_INDEX] & 0x01 != 0;
    if !boot_off && value & 0x01 != 0 {
        core.memory.write_io_port(BOOT_IO_INDEX, 0x01);
    }
}

fn write_serial_control(core: &mut GameBoyCore, value: u8) {
    if value & 0x80 != 0 {
        core.memory.write_io_port(SC_IO_INDEX, value & 0x83);
        core.serial.is_transferring = true;
        if value & 0x01 != 0 {
            core.serial.clock_is_external = false;
            core.serial.timer = 512;
            if !core.rom.properties.cgb_flag {
                let control = core.memory.io_ports[SC_IO_INDEX] | 0x02;
                core.memory.write_io_port(SC_IO_INDEX, control);
            } else if value & 0x02 != 0 {
                core.serial.timer /= 32;
            }
        } else {
            core.serial.clock_is_external = true;
            core.serial.timer = 1;
        }
    } else {
        core.memory.write_io_port(SC_IO_INDEX, value & 0x83);
        core.serial.is_transferring = false;
        core.serial.request = false;
    }
}

fn write_divider(core: &mut GameBoyCore) {
    let old_signal = timer_edge_signal(core, core.memory.io_ports[TAC_IO_INDEX]);
    core.cpu_timing.system_counter = 0;
    core.memory.write_io_port(DIV_IO_INDEX, 0);
    tick_timer_on_falling_edge(core, old_signal);
}

fn write_timer_counter(core: &mut GameBoyCore, value: u8) {
    core.cpu_timing.tima_reload_delay = 0;
    core.memory.write_io_port(TIMA_IO_INDEX, value);
}

fn write_timer_modulo(core: &mut GameBoyCore, value: u8) {
    core.memory.write_io_port(TMA_IO_INDEX, value);
}

fn write_timer_control(core: &mut GameBoyCore, value: u8) {
    let old_tac = core.memory.io_ports[TAC_IO_INDEX];
    let should_tick = tac_write_triggers_timer_tick(core, old_tac, value & 0x07);
    core.memory.write_io_port(TAC_IO_INDEX, value & 0x07);
    if should_tick {
        increment_timer_counter(core);
    }
}

fn advance_system_counter(core: &mut GameBoyCore, clocks: u16) {
    let tac = core.memory.io_ports[TAC_IO_INDEX];
    let old_signal = timer_edge_signal(core, tac);
    core.cpu_timing.system_counter = core.cpu_timing.system_counter.wrapping_add(clocks);
    core.memory
        .write_io_port(DIV_IO_INDEX, (core.cpu_timing.system_counter >> 8) as u8);
    tick_timer_on_falling_edge(core, old_signal);
}

fn tac_write_triggers_timer_tick(core: &GameBoyCore, old_tac: u8, new_tac: u8) -> bool {
    if core.rom.properties.cgb_flag {
        return selected_timer_bit(core, old_tac)
            && !selected_timer_bit(core, new_tac)
            && new_tac & 0x04 != 0;
    }

    timer_edge_signal(core, old_tac) && !timer_edge_signal(core, new_tac)
}

fn timer_edge_signal(core: &GameBoyCore, tac: u8) -> bool {
    tac & 0x04 != 0 && selected_timer_bit(core, tac)
}

fn selected_timer_bit(core: &GameBoyCore, tac: u8) -> bool {
    let bit = match tac & 0x03 {
        0 => 9,
        1 => 3,
        2 => 5,
        _ => 7,
    };
    core.cpu_timing.system_counter & (1_u16 << bit) != 0
}

fn tick_timer_on_falling_edge(core: &mut GameBoyCore, old_signal: bool) {
    let new_signal = timer_edge_signal(core, core.memory.io_ports[TAC_IO_INDEX]);
    if old_signal && !new_signal {
        increment_timer_counter(core);
    }
}

fn increment_timer_counter(core: &mut GameBoyCore) {
    if core.cpu_timing.tima_reload_delay != 0 {
        return;
    }

    let timer = core.memory.io_ports[TIMA_IO_INDEX];
    let next_timer = timer.wrapping_add(1);
    core.memory.write_io_port(TIMA_IO_INDEX, next_timer);
    if next_timer == 0 {
        core.cpu_timing.tima_reload_delay = TIMER_RELOAD_DELAY_M_CYCLES;
    }
}

fn finish_pending_timer_reload(core: &mut GameBoyCore) {
    if core.cpu_timing.tima_reload_delay == 0 {
        return;
    }

    core.cpu_timing.tima_reload_delay -= 1;
    if core.cpu_timing.tima_reload_delay != 0 {
        return;
    }

    let modulo = core.memory.io_ports[TMA_IO_INDEX];
    core.memory.write_io_port(TIMA_IO_INDEX, modulo);
    let interrupt_flags = core.memory.io_ports[IF_IO_INDEX] | INTERRUPT_TIMER;
    core.memory.write_io_port(IF_IO_INDEX, interrupt_flags);
}

fn write_lcd_control(core: &mut GameBoyCore, value: u8) {
    let was_enabled = core.memory.io_ports[LCDC_IO_INDEX] & 0x80 != 0;
    let will_be_enabled = value & 0x80 != 0;

    if !will_be_enabled {
        core.memory_access.vram = true;
        core.memory_access.oam = true;
        core.gpu_timing.blanked_screen = false;
        core.gpu_timing.time_in_mode = 0;
        core.gpu_timing.line_scan_vram_clocks = 172;
        core.gpu_mode = GpuMode::HBlank;
        core.memory
            .write_io_port(STAT_IO_INDEX, core.memory.io_ports[STAT_IO_INDEX] & 0xfc);
        core.memory.write_io_port(LY_IO_INDEX, 0);
    } else if !was_enabled {
        core.memory_access.oam = false;
        core.memory_access.vram = true;
        core.gpu_timing.blanked_screen = false;
        core.gpu_timing.time_in_mode = 0;
        core.gpu_timing.line_scan_vram_clocks = 172;
        core.gpu_mode = GpuMode::ScanOam;
        core.memory.write_io_port(
            STAT_IO_INDEX,
            (core.memory.io_ports[STAT_IO_INDEX] & 0xfc) | 0x02,
        );
        core.memory.write_io_port(LY_IO_INDEX, 0);
        core.video_frame.begin_frame();
    }

    core.memory.write_io_port(LCDC_IO_INDEX, value);
}

fn start_oam_dma(core: &mut GameBoyCore, value: u8) {
    core.memory.write_io_port(DMA_IO_INDEX, value);
    core.dma.oam.pending_source_high = Some(value);
}

fn write_key1(core: &mut GameBoyCore, value: u8) {
    if !core.rom.properties.cgb_flag {
        return;
    }
    let current_speed = core.memory.io_ports[KEY1_IO_INDEX] & 0x80;
    let prepare = value & 0x01;
    core.memory
        .write_io_port(KEY1_IO_INDEX, current_speed | prepare);
}

fn write_vram_bank(core: &mut GameBoyCore, value: u8) {
    if !core.rom.properties.cgb_flag {
        return;
    }
    let bank = value & 0x01;
    core.memory.write_io_port(VBK_IO_INDEX, bank);
    core.memory_access.vram_bank_offset = u32::from(bank) * 0x2000;
}

fn write_wram_bank(core: &mut GameBoyCore, value: u8) {
    if !core.rom.properties.cgb_flag {
        return;
    }
    let bank = u32::from(value & 0x07).max(1);
    core.memory.write_io_port(SVBK_IO_INDEX, value & 0x07);
    core.memory_access.wram_bank_offset = bank * 0x1000;
}

fn write_hdma5(core: &mut GameBoyCore, value: u8) {
    if !core.rom.properties.cgb_flag {
        return;
    }
    if value & 0x80 == 0 {
        if hblank_dma_active(core) {
            core.memory
                .write_io_port(HDMA5_IO_INDEX, core.memory.io_ports[HDMA5_IO_INDEX] | 0x80);
            return;
        }
        let blocks = usize::from(value & 0x7f) + 1;
        transfer_hdma_block(core, blocks * HDMA_BLOCK_BYTES);
        add_vram_dma_cpu_halt(core, blocks);
        core.memory.write_io_port(HDMA5_IO_INDEX, 0xff);
    } else {
        core.memory.write_io_port(HDMA5_IO_INDEX, value & 0x7f);
    }
}

fn write_cgb_bg_palette_index(core: &mut GameBoyCore, value: u8) {
    if !core.rom.properties.cgb_flag {
        return;
    }
    core.memory.write_io_port(BCPS_IO_INDEX, value & 0xbf);
    core.cgb_palette_registers.bg_index = u32::from(value & 0x3f);
    core.cgb_palette_registers.bg_increment = u32::from(value & 0x80 != 0);
}

fn write_cgb_bg_palette_data(core: &mut GameBoyCore, value: u8) {
    if !core.rom.properties.cgb_flag {
        return;
    }
    let index = core.cgb_palette_registers.bg_index as usize;
    if let Some(slot) = core.cgb_palette_registers.bg_data.get_mut(index) {
        *slot = value;
    }
    let pair = index & 0xfe;
    if let (Some(&lo), Some(&hi)) = (
        core.cgb_palette_registers.bg_data.get(pair),
        core.cgb_palette_registers.bg_data.get(pair | 0x01),
    ) {
        if let Some(colour) = core.cgb_palette_registers.bg_index.checked_shr(1) {
            if let Some(slot) = core.palettes.cgb_bg.get_mut(colour as usize) {
                *slot = remap_555_8888(lo, hi);
            }
        }
    }
    increment_cgb_bg_palette_index(core);
}

fn write_cgb_obj_palette_index(core: &mut GameBoyCore, value: u8) {
    if !core.rom.properties.cgb_flag {
        return;
    }
    core.memory.write_io_port(OCPS_IO_INDEX, value & 0xbf);
    core.cgb_palette_registers.obj_index = u32::from(value & 0x3f);
    core.cgb_palette_registers.obj_increment = u32::from(value & 0x80 != 0);
}

fn write_cgb_obj_palette_data(core: &mut GameBoyCore, value: u8) {
    if !core.rom.properties.cgb_flag {
        return;
    }
    let index = core.cgb_palette_registers.obj_index as usize;
    if let Some(slot) = core.cgb_palette_registers.obj_data.get_mut(index) {
        *slot = value;
    }
    let pair = index & 0xfe;
    if let (Some(&lo), Some(&hi)) = (
        core.cgb_palette_registers.obj_data.get(pair),
        core.cgb_palette_registers.obj_data.get(pair | 0x01),
    ) {
        if let Some(colour) = core.cgb_palette_registers.obj_index.checked_shr(1) {
            if let Some(slot) = core.palettes.cgb_obj.get_mut(colour as usize) {
                *slot = remap_555_8888(lo, hi);
            }
        }
    }
    increment_cgb_obj_palette_index(core);
}

fn increment_cgb_bg_palette_index(core: &mut GameBoyCore) {
    if core.cgb_palette_registers.bg_increment != 0 {
        core.cgb_palette_registers.bg_index = (core.cgb_palette_registers.bg_index + 1) & 0x3f;
        core.memory.write_io_port(
            BCPS_IO_INDEX,
            (core.memory.io_ports[BCPS_IO_INDEX] & 0x80)
                | (core.cgb_palette_registers.bg_index as u8),
        );
    }
}

fn increment_cgb_obj_palette_index(core: &mut GameBoyCore) {
    if core.cgb_palette_registers.obj_increment != 0 {
        core.cgb_palette_registers.obj_index = (core.cgb_palette_registers.obj_index + 1) & 0x3f;
        core.memory.write_io_port(
            OCPS_IO_INDEX,
            (core.memory.io_ports[OCPS_IO_INDEX] & 0x80)
                | (core.cgb_palette_registers.obj_index as u8),
        );
    }
}

fn remap_555_8888(lo: u8, hi: u8) -> u32 {
    let value = u16::from(lo) | (u16::from(hi) << 8);
    let red = u32::from(value & 0x1f) * 255 / 31;
    let green = u32::from((value >> 5) & 0x1f) * 255 / 31;
    let blue = u32::from((value >> 10) & 0x1f) * 255 / 31;
    0xff00_0000 | (red << 16) | (green << 8) | blue
}

fn transfer_hdma_block(core: &mut GameBoyCore, byte_count: usize) {
    let mut source = hdma_source(core);
    if !valid_hdma_source(source) {
        return;
    }
    let mut destination = hdma_destination(core);
    for _ in 0..byte_count {
        let value = read8_unrestricted(core, source);
        write_hdma_vram_byte(core, destination, value);
        source = source.wrapping_add(1);
        destination = 0x8000 | (destination.wrapping_add(1) & 0x1fff);
    }
    set_hdma_source(core, source);
    set_hdma_destination(core, destination);
}

fn write_hdma_vram_byte(core: &mut GameBoyCore, destination: u16, value: u8) {
    core.memory
        .write_vram(core.memory_access.vram_bank_offset, destination, value);
}

fn hdma_source(core: &GameBoyCore) -> u16 {
    u16::from(core.memory.io_ports[HDMA1_IO_INDEX]) << 8
        | u16::from(core.memory.io_ports[HDMA2_IO_INDEX])
}

fn hdma_destination(core: &GameBoyCore) -> u16 {
    0x8000
        | ((u16::from(core.memory.io_ports[HDMA3_IO_INDEX]) << 8
            | u16::from(core.memory.io_ports[HDMA4_IO_INDEX]))
            & 0x1fff)
}

fn set_hdma_source(core: &mut GameBoyCore, source: u16) {
    core.memory
        .write_io_port(HDMA1_IO_INDEX, (source >> 8) as u8);
    core.memory
        .write_io_port(HDMA2_IO_INDEX, (source & 0x00f0) as u8);
}

fn set_hdma_destination(core: &mut GameBoyCore, destination: u16) {
    let relative = destination & 0x1ff0;
    core.memory
        .write_io_port(HDMA3_IO_INDEX, ((relative >> 8) as u8) & 0x1f);
    core.memory
        .write_io_port(HDMA4_IO_INDEX, (relative & 0x00f0) as u8);
}

fn valid_hdma_source(source: u16) -> bool {
    (source & 0xe000) != 0x8000 && source < 0xe000
}

fn hblank_dma_active(core: &GameBoyCore) -> bool {
    core.rom.properties.cgb_flag
        && core.memory.io_ports[HDMA5_IO_INDEX] != 0xff
        && core.memory.io_ports[HDMA5_IO_INDEX] & 0x80 == 0
}

fn add_vram_dma_cpu_halt(core: &mut GameBoyCore, blocks: usize) {
    let blocks = i32::try_from(blocks).unwrap_or(i32::MAX / HDMA_BLOCK_M_CYCLES);
    let speed_factor = core.gpu_timing.clock_factor.max(1);
    let cycles = blocks
        .saturating_mul(HDMA_BLOCK_M_CYCLES)
        .saturating_mul(speed_factor);
    core.dma.vram.cpu_halt_m_cycles = core.dma.vram.cpu_halt_m_cycles.saturating_add(cycles);
}

fn oam_dma_blocks_cpu_access(core: &GameBoyCore, address: u16) -> bool {
    core.dma.oam.active && !core.rom.properties.cgb_flag && !(0xff80..=0xfffe).contains(&address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_boy::emulator::input::JoypadInputNibbles;
    use crate::game_boy::emulator::rom::{MemoryBankController, RomProperties};

    #[test]
    fn vram_bus_writes_decode_tiles() {
        let mut core = GameBoyCore::default();
        core.memory_access.vram = true;

        write8(&mut core, 0x8000, 0x80);
        write8(&mut core, 0x8001, 0x40);

        assert_eq!(&core.memory.tile_set[0..2], &[1, 2]);
    }

    #[test]
    fn mbc1_rom_bank_writes_change_bank_offset() {
        let mut core = GameBoyCore::default();
        core.rom.properties = RomProperties {
            mbc: MemoryBankController::Mbc1,
            bank_select_mask: 0x1f,
            ..Default::default()
        };

        write8(&mut core, 0x2000, 0x03);

        assert_eq!(core.rom.bank_offset, 0x0000_c000);
    }

    #[test]
    fn mbc1_rom_banking_mode_combines_upper_and_lower_bank_bits() {
        let mut core = GameBoyCore::default();
        core.rom.properties = RomProperties {
            mbc: MemoryBankController::Mbc1,
            bank_select_mask: 0x3f,
            ..Default::default()
        };
        core.memory.rom[0x0000] = 0xa0;
        core.memory.rom[0x21 * 0x4000] = 0xc1;

        write8(&mut core, 0x2000, 0x01);
        write8(&mut core, 0x4000, 0x01);

        assert_eq!(core.rom.fixed_bank_offset, 0);
        assert_eq!(core.rom.bank_offset, 0x21 * ROM_BANK_SIZE);
        assert_eq!(read8(&core, 0x0000), 0xa0);
        assert_eq!(read8(&core, 0x4000), 0xc1);
    }

    #[test]
    fn mbc1_ram_banking_mode_maps_upper_bits_into_fixed_bank_area() {
        let mut core = GameBoyCore::default();
        core.rom.properties = RomProperties {
            mbc: MemoryBankController::Mbc1,
            bank_select_mask: 0x3f,
            ..Default::default()
        };
        core.memory.rom[0x0000] = 0xa0;
        core.memory.rom[0x0021 * 0x4000] = 0xc1;
        core.memory.rom[0x0020 * 0x4000] = 0xd0;

        write8(&mut core, 0x2000, 0x01);
        write8(&mut core, 0x4000, 0x01);
        write8(&mut core, 0x6000, 0x01);

        assert_eq!(core.rom.fixed_bank_offset, 0x20 * ROM_BANK_SIZE);
        assert_eq!(core.rom.bank_offset, 0x21 * ROM_BANK_SIZE);
        assert_eq!(core.sram.bank_offset, 0x2000);
        assert_eq!(read8(&core, 0x0000), 0xd0);
        assert_eq!(read8(&core, 0x4000), 0xc1);
    }

    #[test]
    fn mbc1_full_width_bank_write_logs_suspicious_mapper_warning_once() {
        let mut core = GameBoyCore::default();
        core.rom.properties = RomProperties {
            mbc: MemoryBankController::Mbc1,
            bank_select_mask: 0x3f,
            ..Default::default()
        };

        write8(&mut core, 0x2100, 0x21);

        assert_eq!(core.rom.properties.mbc, MemoryBankController::Mbc1);
        assert!(core.rom.suspicious_mbc_warning_logged);

        core.rom.suspicious_mbc_warning_logged = false;
        write8(&mut core, 0x2100, 0x01);

        assert!(!core.rom.suspicious_mbc_warning_logged);
    }

    #[test]
    fn mbc2_uses_address_bit_eight_for_ram_enable_and_rom_bank() {
        let mut core = GameBoyCore::default();
        core.rom.properties = RomProperties {
            mbc: MemoryBankController::Mbc2,
            bank_select_mask: 0x0f,
            ..Default::default()
        };

        write8(&mut core, 0x0000, 0x0a);
        assert!(core.sram.enable_flag);

        write8(&mut core, 0x0100, 0x03);
        assert_eq!(core.rom.bank_offset, 0x03 * ROM_BANK_SIZE);
        assert!(core.sram.enable_flag);

        write8(&mut core, 0x2000, 0x00);
        assert!(!core.sram.enable_flag);

        write8(&mut core, 0x2100, 0x00);
        assert_eq!(core.rom.bank_offset, ROM_BANK_SIZE);

        write8(&mut core, 0x4000, 0x0a);
        assert!(!core.sram.enable_flag);
    }

    #[test]
    fn mbc3_selects_ram_banks_zero_through_seven() {
        let mut core = GameBoyCore::default();
        core.rom.properties = RomProperties {
            mbc: MemoryBankController::Mbc3,
            ..Default::default()
        };
        core.sram.enable_flag = true;
        core.sram.size_bytes = 65_536;

        write8(&mut core, 0x4000, 0x07);
        write8(&mut core, 0xa000, 0x5a);

        assert_eq!(core.sram.bank_offset, 7 * 0x2000);
        assert_eq!(read8(&core, 0xa000), 0x5a);
    }

    #[test]
    fn mbc3_rtc_registers_latch_on_zero_to_one_write() {
        let mut core = GameBoyCore::default();
        core.rom.properties = RomProperties {
            mbc: MemoryBankController::Mbc3,
            ..Default::default()
        };
        core.sram.has_timer = true;

        write8(&mut core, 0x0000, 0x0a);
        write8(&mut core, 0x4000, 0x08);
        write8(&mut core, 0xa000, 0x22);
        write8(&mut core, 0x6000, 0x00);
        write8(&mut core, 0x6000, 0x01);
        write8(&mut core, 0xa000, 0x33);

        assert_eq!(read8(&core, 0xa000), 0x22);

        write8(&mut core, 0x6000, 0x00);
        write8(&mut core, 0x6000, 0x01);

        assert_eq!(read8(&core, 0xa000), 0x33);
    }

    #[test]
    fn system_counter_updates_visible_divider() {
        let mut core = GameBoyCore::default();

        step_system_counter(&mut core, 256);

        assert_eq!(core.cpu_timing.system_counter, 0x0100);
        assert_eq!(core.memory.io_ports[DIV_IO_INDEX], 0x01);
    }

    #[test]
    fn divider_reset_ticks_timer_on_selected_bit_falling_edge() {
        let mut core = GameBoyCore::default();
        core.cpu_timing.system_counter = 0x0008;
        core.memory.io_ports[DIV_IO_INDEX] = 0x22;
        core.memory.io_ports[TAC_IO_INDEX] = 0x05;

        write8(&mut core, 0xff04, 0xaa);

        assert_eq!(core.cpu_timing.system_counter, 0);
        assert_eq!(core.memory.io_ports[DIV_IO_INDEX], 0);
        assert_eq!(core.memory.io_ports[TIMA_IO_INDEX], 1);
    }

    #[test]
    fn timer_control_write_ticks_timer_on_dmg_falling_edge() {
        let mut core = GameBoyCore::default();
        core.cpu_timing.system_counter = 0x0008;
        core.memory.io_ports[TAC_IO_INDEX] = 0x05;

        write8(&mut core, 0xff07, 0x04);

        assert_eq!(core.memory.io_ports[TAC_IO_INDEX], 0x04);
        assert_eq!(core.memory.io_ports[TIMA_IO_INDEX], 1);
    }

    #[test]
    fn cgb_timer_control_disable_does_not_tick_timer() {
        let mut core = GameBoyCore::default();
        core.rom.properties.cgb_flag = true;
        core.cpu_timing.system_counter = 0x0008;
        core.memory.io_ports[TAC_IO_INDEX] = 0x05;

        write8(&mut core, 0xff07, 0x01);

        assert_eq!(core.memory.io_ports[TAC_IO_INDEX], 0x01);
        assert_eq!(core.memory.io_ports[TIMA_IO_INDEX], 0);
    }

    #[test]
    fn joyp_reads_include_select_bits_and_selected_low_nibble() {
        let mut core = GameBoyCore::default();
        core.runtime.joypad = JoypadInputNibbles {
            button: 0x0e,
            direction: 0x0d,
        };

        write8(&mut core, 0xff00, JOYP_SELECT_DIRECTIONS);
        assert_eq!(read8(&core, 0xff00), 0xed);

        write8(&mut core, 0xff00, JOYP_SELECT_BUTTONS);
        assert_eq!(read8(&core, 0xff00), 0xde);

        write8(&mut core, 0xff00, 0x00);
        assert_eq!(read8(&core, 0xff00), 0xcc);

        write8(&mut core, 0xff00, JOYP_SELECT_NONE);
        assert_eq!(read8(&core, 0xff00), 0xff);
    }

    #[test]
    fn joyp_write_requests_interrupt_when_selected_bits_fall() {
        let mut core = GameBoyCore::default();
        core.memory.io_ports[JOYP_IO_INDEX] = 0xc0 | JOYP_SELECT_NONE | 0x0f;
        core.runtime.joypad = JoypadInputNibbles {
            button: 0x0e,
            direction: 0x0f,
        };

        write8(&mut core, 0xff00, JOYP_SELECT_BUTTONS);

        assert_eq!(core.memory.io_ports[JOYP_IO_INDEX], 0xde);
        assert_eq!(
            core.memory.io_ports[IF_IO_INDEX] & INTERRUPT_JOYPAD,
            INTERRUPT_JOYPAD
        );
    }

    #[test]
    fn joyp_write_does_not_interrupt_without_low_nibble_falling_edge() {
        let mut core = GameBoyCore::default();
        core.memory.io_ports[JOYP_IO_INDEX] = 0xc0 | JOYP_SELECT_BUTTONS | 0x0e;
        core.runtime.joypad = JoypadInputNibbles {
            button: 0x0f,
            direction: 0x0f,
        };

        write8(&mut core, 0xff00, JOYP_SELECT_BUTTONS);

        assert_eq!(core.memory.io_ports[JOYP_IO_INDEX], 0xdf);
        assert_eq!(core.memory.io_ports[IF_IO_INDEX] & INTERRUPT_JOYPAD, 0);
    }

    #[test]
    fn boot_lock_only_transitions_from_mapped_to_unmapped() {
        let mut core = GameBoyCore::default();
        core.memory.io_ports[BOOT_IO_INDEX] = 0x00;

        write8(&mut core, 0xff50, 0x00);
        assert_eq!(read8(&core, 0xff50), 0x00);

        write8(&mut core, 0xff50, 0x01);
        assert_eq!(read8(&core, 0xff50), 0x01);

        write8(&mut core, 0xff50, 0x00);
        assert_eq!(read8(&core, 0xff50), 0x01);
    }

    #[test]
    fn lcd_enable_starts_line_zero_oam_scan() {
        let mut core = GameBoyCore::default();
        core.memory_access.oam = true;
        core.memory_access.vram = true;
        core.gpu_mode = GpuMode::HBlank;
        core.gpu_timing.time_in_mode = 37;
        core.memory.io_ports[LCDC_IO_INDEX] = 0x00;
        core.memory.io_ports[STAT_IO_INDEX] = 0x00;
        core.memory.io_ports[LY_IO_INDEX] = 42;

        write8(&mut core, 0xff40, 0x80);

        assert_eq!(core.gpu_mode, GpuMode::ScanOam);
        assert_eq!(core.gpu_timing.time_in_mode, 0);
        assert_eq!(core.memory.io_ports[STAT_IO_INDEX] & 0x03, 0x02);
        assert_eq!(core.memory.io_ports[LY_IO_INDEX], 0);
        assert!(!core.memory_access.oam);
        assert!(core.memory_access.vram);
    }

    #[test]
    fn lcd_disable_enters_hblank_stat_mode() {
        let mut core = GameBoyCore::default();
        core.memory_access.oam = false;
        core.memory_access.vram = false;
        core.gpu_mode = GpuMode::ScanVram;
        core.memory.io_ports[LCDC_IO_INDEX] = 0x80;
        core.memory.io_ports[STAT_IO_INDEX] = 0x83;
        core.memory.io_ports[LY_IO_INDEX] = 42;

        write8(&mut core, 0xff40, 0x00);

        assert_eq!(core.gpu_mode, GpuMode::HBlank);
        assert_eq!(core.memory.io_ports[STAT_IO_INDEX] & 0x03, 0x00);
        assert_eq!(core.memory.io_ports[LY_IO_INDEX], 0);
        assert!(core.memory_access.oam);
        assert!(core.memory_access.vram);
    }

    #[test]
    fn cgb_general_hdma_copies_to_vram() {
        let mut core = GameBoyCore::default();
        core.rom.properties.cgb_flag = true;
        core.memory.io_ports[HDMA5_IO_INDEX] = 0xff;
        core.memory.rom[0x0200] = 0x80;
        core.memory.rom[0x0201] = 0x40;
        write8(&mut core, 0xff51, 0x02);
        write8(&mut core, 0xff52, 0x00);
        write8(&mut core, 0xff53, 0x00);
        write8(&mut core, 0xff54, 0x00);

        write8(&mut core, 0xff55, 0x00);

        assert_eq!(core.memory.vram[0], 0x80);
        assert_eq!(core.memory.vram[1], 0x40);
        assert_eq!(&core.memory.tile_set[0..2], &[1, 2]);
        assert_eq!(core.memory.io_ports[HDMA5_IO_INDEX], 0xff);
        assert_eq!(core.dma.vram.cpu_halt_m_cycles, HDMA_BLOCK_M_CYCLES);
    }

    #[test]
    fn hblank_hdma_copies_one_block_and_completes() {
        let mut core = GameBoyCore::default();
        core.rom.properties.cgb_flag = true;
        core.memory_access.vram = true;
        core.memory.rom[0x0200] = 0x55;
        write8(&mut core, 0xff51, 0x02);
        write8(&mut core, 0xff52, 0x00);
        write8(&mut core, 0xff53, 0x00);
        write8(&mut core, 0xff54, 0x00);
        write8(&mut core, 0xff55, 0x80);

        run_hblank_dma(&mut core);

        assert_eq!(core.memory.vram[0], 0x55);
        assert_eq!(core.memory.io_ports[HDMA5_IO_INDEX], 0xff);
        assert_eq!(core.dma.vram.cpu_halt_m_cycles, HDMA_BLOCK_M_CYCLES);
    }

    #[test]
    fn hblank_hdma_tracks_active_remaining_blocks() {
        let mut core = GameBoyCore::default();
        core.rom.properties.cgb_flag = true;
        core.memory.rom[0x0200] = 0x11;
        core.memory.rom[0x0210] = 0x22;
        write8(&mut core, 0xff51, 0x02);
        write8(&mut core, 0xff52, 0x00);
        write8(&mut core, 0xff53, 0x00);
        write8(&mut core, 0xff54, 0x00);

        write8(&mut core, 0xff55, 0x81);

        assert_eq!(core.memory.io_ports[HDMA5_IO_INDEX], 0x01);

        run_hblank_dma(&mut core);

        assert_eq!(core.memory.vram[0], 0x11);
        assert_eq!(core.memory.io_ports[HDMA5_IO_INDEX], 0x00);
        assert_eq!(core.dma.vram.cpu_halt_m_cycles, HDMA_BLOCK_M_CYCLES);

        run_hblank_dma(&mut core);

        assert_eq!(core.memory.vram[0x10], 0x22);
        assert_eq!(core.memory.io_ports[HDMA5_IO_INDEX], 0xff);
        assert_eq!(core.dma.vram.cpu_halt_m_cycles, HDMA_BLOCK_M_CYCLES * 2);
    }

    #[test]
    fn hblank_hdma_does_not_run_while_cpu_is_halted() {
        let mut core = GameBoyCore::default();
        core.rom.properties.cgb_flag = true;
        core.cpu_mode = CpuMode::Halted;
        core.memory.rom[0x0200] = 0x55;
        write8(&mut core, 0xff51, 0x02);
        write8(&mut core, 0xff52, 0x00);
        write8(&mut core, 0xff53, 0x00);
        write8(&mut core, 0xff54, 0x00);
        write8(&mut core, 0xff55, 0x80);

        run_hblank_dma(&mut core);

        assert_eq!(core.memory.vram[0], 0x00);
        assert_eq!(core.memory.io_ports[HDMA5_IO_INDEX], 0x00);
        assert_eq!(core.dma.vram.cpu_halt_m_cycles, 0);
    }

    #[test]
    fn active_hblank_hdma_can_be_stopped_by_hdma5_write() {
        let mut core = GameBoyCore::default();
        core.rom.properties.cgb_flag = true;
        write8(&mut core, 0xff55, 0x82);

        write8(&mut core, 0xff55, 0x00);

        assert_eq!(core.memory.io_ports[HDMA5_IO_INDEX], 0x82);
    }

    #[test]
    fn oam_dma_starts_after_deferred_activation_and_steps_one_byte_per_mcycle() {
        let mut core = GameBoyCore::default();
        core.memory_access.oam = true;
        core.memory.rom[0x0200] = 0x12;
        core.memory.rom[0x0201] = 0x34;

        write8(&mut core, 0xff46, 0x02);

        assert_eq!(core.memory.oam[0], 0x00);
        assert_eq!(core.dma.oam.pending_source_high, Some(0x02));
        assert!(!core.dma.oam.active);

        begin_deferred_oam_dma(&mut core);
        step_oam_dma(&mut core);

        assert_eq!(core.memory.oam[0], 0x12);
        assert_eq!(core.memory.oam[1], 0x00);
        assert!(core.dma.oam.active);

        step_oam_dma(&mut core);

        assert_eq!(core.memory.oam[1], 0x34);
        assert_eq!(core.memory.io_ports[DMA_IO_INDEX], 0x02);
    }

    #[test]
    fn oam_dma_blocks_non_hram_cpu_access_on_dmg() {
        let mut core = GameBoyCore::default();
        core.memory.rom[0x0100] = 0x12;
        core.memory.io_ports[0x80] = 0x34;
        core.dma.oam.active = true;

        assert_eq!(read8(&core, 0x0100), 0xff);
        assert_eq!(read8(&core, 0xff80), 0x34);

        write8(&mut core, 0xc000, 0x56);
        write8(&mut core, 0xff80, 0x78);

        assert_eq!(core.memory.wram[0], 0x00);
        assert_eq!(core.memory.io_ports[0x80], 0x78);
    }

    #[test]
    fn oam_dma_does_not_block_cgb_cpu_bus() {
        let mut core = GameBoyCore::default();
        core.rom.properties.cgb_flag = true;
        core.memory.rom[0x0100] = 0x12;
        core.dma.oam.active = true;

        assert_eq!(read8(&core, 0x0100), 0x12);
    }

    #[test]
    fn sgb_joypad_packet_enables_multiplayer_mode() {
        let mut core = GameBoyCore::default();
        core.rom.properties.sgb_flag = true;
        let mut packet = [0_u8; 16];
        packet[0] = (0x11 << 3) | 0x01;
        packet[1] = 0x01;

        write_sgb_packet(&mut core, packet);

        assert!(core.sgb.mult_enabled);
        assert_eq!(core.sgb.player_count, 2);
        assert_eq!(core.sgb.read_joypad_id, 0x0f);
        assert_eq!(core.sgb.completed_commands, 1);
        assert_eq!(core.sgb.last_command, 0x11);
        assert!(!core.sgb.reading_command);
    }

    #[test]
    fn sgb_joypad_packet_ignores_repeated_select_writes() {
        let mut core = GameBoyCore::default();
        core.rom.properties.sgb_flag = true;
        let mut packet = [0_u8; 16];
        packet[0] = (0x11 << 3) | 0x01;
        packet[1] = 0x01;

        write8(&mut core, 0xff00, 0x00);
        for byte in packet {
            for bit in 0..8 {
                let select = if byte & (1 << bit) != 0 {
                    JOYP_SELECT_BUTTONS
                } else {
                    JOYP_SELECT_DIRECTIONS
                };
                write8(&mut core, 0xff00, select);
                write8(&mut core, 0xff00, select);
                write8(&mut core, 0xff00, JOYP_SELECT_NONE);
            }
        }
        write8(&mut core, 0xff00, JOYP_SELECT_DIRECTIONS);
        write8(&mut core, 0xff00, JOYP_SELECT_NONE);

        assert!(core.sgb.mult_enabled);
        assert_eq!(core.sgb.packet_errors, 0);
    }

    #[test]
    fn sgb_joypad_packet_stop_bit_does_not_shift_next_packet() {
        let mut core = GameBoyCore::default();
        core.rom.properties.sgb_flag = true;
        let mut first_packet = [0_u8; 16];
        first_packet[0] = (0x07 << 3) | 0x02;
        first_packet[3] = 44;
        first_packet[6..].fill(0xff);
        let mut second_packet = [0_u8; 16];
        second_packet[0] = 0xff;

        write_sgb_packet(&mut core, first_packet);
        write_sgb_packet(&mut core, second_packet);

        assert_eq!(core.sgb.command_bytes[1][0], 0xff);
        assert_eq!(core.sgb.character_palettes[2 * 20 + 3], 3);
        assert_eq!(core.sgb.packet_errors, 0);
    }

    #[test]
    fn sgb_multiplayer_probe_reads_and_cycles_controller_id() {
        let mut core = GameBoyCore::default();
        core.rom.properties.sgb_flag = true;
        let mut packet = [0_u8; 16];
        packet[0] = (0x11 << 3) | 0x01;
        packet[1] = 0x01;
        write_sgb_packet(&mut core, packet);

        assert_eq!(read8(&core, 0xff00), 0x0f);

        write8(&mut core, 0xff00, JOYP_SELECT_DIRECTIONS);
        write8(&mut core, 0xff00, JOYP_SELECT_NONE);

        assert_eq!(read8(&core, 0xff00), 0x0e);

        write8(&mut core, 0xff00, JOYP_SELECT_BUTTONS);
        write8(&mut core, 0xff00, JOYP_SELECT_NONE);

        assert_eq!(read8(&core, 0xff00), 0x0d);
    }

    #[test]
    fn sgb_multiplayer_probe_does_not_cycle_on_repeated_selected_row() {
        let mut core = GameBoyCore::default();
        core.rom.properties.sgb_flag = true;
        let mut packet = [0_u8; 16];
        packet[0] = (0x11 << 3) | 0x01;
        packet[1] = 0x01;
        write_sgb_packet(&mut core, packet);

        write8(&mut core, 0xff00, JOYP_SELECT_DIRECTIONS);
        write8(&mut core, 0xff00, JOYP_SELECT_DIRECTIONS);

        assert_eq!(core.sgb.read_joypad_id, 0x0f);

        write8(&mut core, 0xff00, JOYP_SELECT_NONE);

        assert_eq!(core.sgb.read_joypad_id, 0x0e);
    }

    fn write_sgb_packet(core: &mut GameBoyCore, packet: [u8; 16]) {
        write8(core, 0xff00, 0x00);
        for byte in packet {
            for bit in 0..8 {
                let select = if byte & (1 << bit) != 0 {
                    JOYP_SELECT_BUTTONS
                } else {
                    JOYP_SELECT_DIRECTIONS
                };
                write8(core, 0xff00, select);
                write8(core, 0xff00, JOYP_SELECT_NONE);
            }
        }
        write8(core, 0xff00, JOYP_SELECT_DIRECTIONS);
        write8(core, 0xff00, JOYP_SELECT_NONE);
    }
}
