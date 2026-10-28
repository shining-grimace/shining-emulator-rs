use super::*;
use crate::game_boy::emulator::bus::{read8, write8};

fn core(cgb: bool) -> GameBoyCore {
    let mut core = GameBoyCore::default();
    core.rom.properties.cgb_flag = cgb;
    core.memory_access.reset_for_rom_load();
    core
}

fn request(core: &mut GameBoyCore, source: u8) {
    write8(core, 0xff46, source);
    step_oam_dma(core); // The register-write M-cycle still belongs to old DMA.
    begin_deferred_oam_dma(core);
}

#[test]
fn fresh_dma_leaves_oam_accessible_during_the_startup_cycle() {
    let mut core = core(false);
    core.memory.oam[0] = 0x12;
    core.memory.rom[0x0200] = 0x34;
    request(&mut core, 2);
    assert_eq!(read8(&core, 0xfe00), 0x12);
    step_oam_dma(&mut core);
    assert_eq!(read8(&core, 0xfe00), 0xff);
    assert_eq!(core.memory.oam[0], 0x12);
    step_oam_dma(&mut core);
    assert_eq!(core.memory.oam[0], 0x34);
    for _ in 1..160 {
        step_oam_dma(&mut core);
    }
    assert!(!core.dma.oam.active);
    assert_eq!(read8(&core, 0xfe00), 0x34);
}

#[test]
fn restart_keeps_old_dma_running_until_the_new_transfer_starts() {
    let mut core = core(false);
    core.memory.rom[0x0200..0x02a0].fill(0x22);
    core.memory.rom[0x0300..0x03a0].fill(0x33);
    request(&mut core, 2);
    step_oam_dma(&mut core);
    step_oam_dma(&mut core);
    request(&mut core, 3);
    assert_eq!(core.memory.oam[1], 0x22);
    assert_eq!(core.dma.oam.source_high, 2);
    assert_eq!(read8(&core, 0xff46), 3);
    assert_eq!(read8(&core, 0xfe00), 0xff);
    step_oam_dma(&mut core);
    assert_eq!(core.memory.oam[2], 0x22);
    assert_eq!(core.dma.oam.source_high, 3);
    assert_eq!(core.dma.oam.next_index, 0);
    step_oam_dma(&mut core);
    assert_eq!(core.memory.oam[0], 0x33);
}

#[test]
fn cgb_conflicts_follow_the_source_bus_and_leave_internal_io_accessible() {
    let mut core = core(true);
    core.memory.rom[0x0100] = 0x12;
    core.memory.wram[0] = 0x34;
    core.dma.oam.active = true;
    core.dma.oam.bus_value = 0x56;
    core.dma.oam.source_high = 0xc0;
    assert_eq!(read8(&core, 0x0100), 0x12);
    assert_eq!(read8(&core, 0xc000), 0x56);
    assert_eq!(read8(&core, 0xe000), 0x56);
    write8(&mut core, 0xc000, 0x99);
    assert_eq!(core.memory.wram[0], 0x34);
    core.dma.oam.source_high = 2;
    assert_eq!(read8(&core, 0x0100), 0x56);
    assert_eq!(read8(&core, 0xc000), 0x34);
    write8(&mut core, 0xc000, 0x78);
    assert_eq!(core.memory.wram[0], 0x78);
    assert_eq!(read8(&core, 0xfe00), 0xff);
    write8(&mut core, 0xff80, 0xab);
    write8(&mut core, 0xffff, 0x1f);
    assert_eq!(read8(&core, 0xff80), 0xab);
    assert_eq!(read8(&core, 0xffff), 0x1f);
    write8(&mut core, 0xff46, 0x80);
    assert_eq!(read8(&core, 0xff46), 0x80);
}

#[test]
fn dma_source_decoding_never_reads_oam_io_or_hram() {
    for (source, offset) in [(0xe0, 0), (0xfe, 0x1e00), (0xff, 0x1f00)] {
        for cgb in [false, true] {
            let mut core = core(cgb);
            core.memory.wram[offset] = 0x42;
            core.memory.oam[0] = 0x21;
            core.memory.io_ports[0] = 0x34;
            request(&mut core, source);
            step_oam_dma(&mut core);
            step_oam_dma(&mut core);
            assert_eq!(core.memory.oam[0], if cgb { 0xff } else { 0x42 });
        }
    }
}

#[test]
fn dma_vram_reads_bypass_cpu_ppu_access_locks() {
    let mut core = core(true);
    core.memory_access.vram = false;
    core.memory_access.vram_bank_offset = 0x2000;
    core.memory.vram[0x2000] = 0x56;
    request(&mut core, 0x80);
    step_oam_dma(&mut core);
    step_oam_dma(&mut core);
    assert_eq!(core.memory.oam[0], 0x56);
    assert_eq!(core.dma.oam.bus_value, 0x56);
}

#[test]
fn restart_request_is_latched_at_the_instructions_actual_write_cycle() {
    let mut core = core(false);
    core.cpu_registers.pc = 0xff80;
    core.cpu_registers.a = 3;
    core.memory.io_ports[0x80..0x82].copy_from_slice(&[0xe0, 0x46]);
    core.memory.rom[0x0200..0x02a0].fill(0x22);
    core.memory.rom[0x0300..0x03a0].fill(0x33);
    core.dma.oam.source_high = 2;
    core.dma.oam.active = true;
    let mut frames = crate::game_boy::frame_buffer::GameBoyFrameRing::default();
    assert_eq!(
        crate::game_boy::emulator::execution::perform_op(&mut core, &mut frames),
        12
    );
    assert_eq!(core.dma.oam.source_high, 2);
    assert_eq!(core.dma.oam.next_index, 3);
    assert_eq!(core.dma.oam.pending_source_high, Some(3));
    assert!(core.dma.oam.startup_pending);
    step_oam_dma(&mut core);
    assert_eq!(core.memory.oam[3], 0x22);
    assert_eq!(core.dma.oam.source_high, 3);
    step_oam_dma(&mut core);
    assert_eq!(core.memory.oam[0], 0x33);
}

#[test]
fn cgb_vram_source_conflicts_do_not_block_cartridge_or_wram() {
    let mut core = core(true);
    core.dma.oam.source_high = 0x80;
    core.dma.oam.active = true;
    core.dma.oam.bus_value = 0x56;
    core.memory.rom[0x0100] = 0x12;
    core.memory.wram[0] = 0x34;
    core.memory.vram[0] = 0x78;
    assert_eq!(read8(&core, 0x0100), 0x12);
    assert_eq!(read8(&core, 0xc000), 0x34);
    assert_eq!(read8(&core, 0x8000), 0x56);
    write8(&mut core, 0x8000, 0x99);
    assert_eq!(core.memory.vram[0], 0x78);
}
