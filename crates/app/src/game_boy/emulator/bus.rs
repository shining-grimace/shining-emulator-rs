use crate::game_boy::emulator::GameBoyCore;
use crate::game_boy::emulator::constants::ROM_BANK_SIZE;
use crate::game_boy::emulator::gpu::GpuMode;
use crate::game_boy::emulator::rom::MemoryBankController;

const JOYP_IO_INDEX: usize = 0x00;
const SB_IO_INDEX: usize = 0x01;
const SC_IO_INDEX: usize = 0x02;
const DIV_IO_INDEX: usize = 0x04;
const TAC_IO_INDEX: usize = 0x07;
const LCDC_IO_INDEX: usize = 0x40;
const STAT_IO_INDEX: usize = 0x41;
const LY_IO_INDEX: usize = 0x44;
const DMA_IO_INDEX: usize = 0x46;
const BGP_IO_INDEX: usize = 0x47;
const OBP0_IO_INDEX: usize = 0x48;
const OBP1_IO_INDEX: usize = 0x49;
const KEY1_IO_INDEX: usize = 0x4d;
const VBK_IO_INDEX: usize = 0x4f;
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

pub(crate) fn read8(core: &GameBoyCore, address: u16) -> u8 {
    let address = usize::from(address);
    if address < 0x4000 {
        return core.memory.rom.get(address).copied().unwrap_or(0xff);
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

fn read_sram(core: &GameBoyCore, address: usize) -> u8 {
    if !core.sram.enable_flag {
        return 0x00;
    }
    if core.sram.has_timer && core.sram.timer_mode > 0 {
        let index = usize::try_from(core.sram.timer_mode.saturating_sub(0x08)).unwrap_or_default();
        return core.sram.timer_data.get(index).copied().unwrap_or(0);
    }
    let bank_offset = usize::try_from(core.sram.bank_offset).unwrap_or_default();
    core.sram
        .data
        .get(bank_offset + (address & 0x1fff))
        .copied()
        .unwrap_or(0xff)
}

fn write_sram(core: &mut GameBoyCore, address: usize, mut value: u8) {
    if !core.sram.enable_flag {
        return;
    }
    if core.sram.has_timer && core.sram.timer_mode > 0 {
        let index = usize::try_from(core.sram.timer_mode.saturating_sub(0x08)).unwrap_or_default();
        if let Some(slot) = core.sram.timer_data.get_mut(index) {
            *slot = value;
        }
        return;
    }
    if core.rom.properties.mbc == MemoryBankController::Mbc2 {
        value &= 0x0f;
    }
    let bank_offset = usize::try_from(core.sram.bank_offset).unwrap_or_default();
    if let Some(slot) = core.sram.data.get_mut(bank_offset + (address & 0x1fff)) {
        *slot = value;
    }
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
        core.rom.bank_offset &= 0xfff8_0000;
        value &= 0x1f;
        if value == 0 {
            value = 1;
        }
        core.rom.bank_offset |= u32::from(value) * ROM_BANK_SIZE;
        mask_bank_offset(core);
    } else if address < 0x6000 {
        value &= 0x03;
        if core.rom.properties.mbc_mode != 0 {
            core.sram.bank_offset = u32::from(value) * 0x2000;
        } else {
            core.rom.bank_offset &= 0xffe7_c000;
            core.rom.bank_offset |= u32::from(value) * 0x80000;
            mask_bank_offset(core);
        }
    } else if core.sram.size_bytes > 8192 {
        core.rom.properties.mbc_mode = u32::from(value & 0x01);
    } else {
        core.rom.properties.mbc_mode = 0;
    }
}

fn write_mbc2(core: &mut GameBoyCore, address: u32, mut value: u8) {
    if address < 0x1000 {
        value &= 0x0f;
        core.sram.enable_flag = value == 0x0a;
    } else if address < 0x2100 {
    } else if address < 0x21ff {
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
        if value < 0x04 {
            core.sram.bank_offset = u32::from(value) * 0x2000;
            core.sram.timer_mode = 0;
        } else if (0x08..0x0d).contains(&value) {
            core.sram.timer_mode = u32::from(value);
        } else {
            core.sram.timer_mode = 0;
        }
    } else {
        value &= 0x01;
        core.sram.timer_latch = u32::from(value);
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
        JOYP_IO_INDEX => {
            let select = core.memory.io_ports[JOYP_IO_INDEX] & 0x30;
            if select == 0x20 {
                core.runtime.joypad.direction
            } else if select == 0x10 {
                core.runtime.joypad.button
            } else if core.sgb.mult_enabled && select == 0x30 {
                core.sgb.read_joypad_id as u8
            } else {
                0x0f
            }
        }
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
        DIV_IO_INDEX => core.memory.write_io_port(DIV_IO_INDEX, 0),
        TAC_IO_INDEX => write_timer_control(core, value),
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
        HDMA5_IO_INDEX => write_hdma5(core, value),
        BCPS_IO_INDEX => write_cgb_bg_palette_index(core, value),
        BCPD_IO_INDEX => write_cgb_bg_palette_data(core, value),
        OCPS_IO_INDEX => write_cgb_obj_palette_index(core, value),
        OCPD_IO_INDEX => write_cgb_obj_palette_data(core, value),
        SVBK_IO_INDEX => write_wram_bank(core, value),
        _ => core.memory.write_io_port(index, value),
    }
}

fn write_joyp(core: &mut GameBoyCore, value: u8) {
    let select = value & 0x30;
    let mut joyp = select;
    if select == 0x20 {
        joyp |= core.runtime.joypad.direction;
    } else if select == 0x10 {
        joyp |= core.runtime.joypad.button;
    }
    core.memory.write_io_port(JOYP_IO_INDEX, joyp);
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

fn write_timer_control(core: &mut GameBoyCore, value: u8) {
    core.cpu_timing.timer_running = value & 0x04 != 0;
    core.cpu_timing.timer_inc_time = match value & 0x03 {
        0 => 1024,
        1 => 16,
        2 => 64,
        _ => 256,
    };
    core.memory.write_io_port(TAC_IO_INDEX, value & 0x07);
}

fn write_lcd_control(core: &mut GameBoyCore, value: u8) {
    let was_enabled = core.memory.io_ports[LCDC_IO_INDEX] & 0x80 != 0;
    let will_be_enabled = value & 0x80 != 0;

    if !will_be_enabled {
        core.memory_access.vram = true;
        core.memory_access.oam = true;
        core.gpu_timing.blanked_screen = false;
        core.gpu_timing.time_in_mode = 0;
        core.gpu_mode = GpuMode::ScanOam;
        core.memory
            .write_io_port(STAT_IO_INDEX, core.memory.io_ports[STAT_IO_INDEX] & 0xfc);
        core.memory.write_io_port(LY_IO_INDEX, 0);
    } else if !was_enabled {
        core.video_frame.begin_frame();
    }

    core.memory.write_io_port(LCDC_IO_INDEX, value);
}

fn start_oam_dma(core: &mut GameBoyCore, value: u8) {
    core.memory.write_io_port(DMA_IO_INDEX, value);
    if value < 0x80 {
        return;
    }
    let mut source = u16::from(value) << 8;
    for index in 0..core.memory.oam.len() {
        let byte = read8(core, source);
        core.memory.oam[index] = byte;
        source = source.wrapping_add(1);
    }
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
    core.memory.write_io_port(HDMA5_IO_INDEX, value);
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
    }
}

fn increment_cgb_obj_palette_index(core: &mut GameBoyCore) {
    if core.cgb_palette_registers.obj_increment != 0 {
        core.cgb_palette_registers.obj_index = (core.cgb_palette_registers.obj_index + 1) & 0x3f;
    }
}

fn remap_555_8888(lo: u8, hi: u8) -> u32 {
    let value = u16::from(lo) | (u16::from(hi) << 8);
    let red = u32::from(value & 0x1f) * 255 / 31;
    let green = u32::from((value >> 5) & 0x1f) * 255 / 31;
    let blue = u32::from((value >> 10) & 0x1f) * 255 / 31;
    0xff00_0000 | (red << 16) | (green << 8) | blue
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn timer_control_configures_timer_frequency() {
        let mut core = GameBoyCore::default();

        write8(&mut core, 0xff07, 0x05);

        assert!(core.cpu_timing.timer_running);
        assert_eq!(core.cpu_timing.timer_inc_time, 16);
    }
}
