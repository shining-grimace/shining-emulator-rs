use super::*;

fn dispatch(core: &mut GameBoyCore) {
    let mut frames = GameBoyFrameRing::default();
    // Exercise the same entry point as normal emulation, including IME/HALT.
    super::super::execute_next_step(core, &mut frames);
}

fn interrupt_core(pc: u32, sp: u32, pending: u8) -> GameBoyCore {
    let mut core = GameBoyCore::default();
    core.cpu_registers.pc = pc;
    core.cpu_registers.sp = sp;
    core.cpu_registers.ime = true;
    core.cpu_timing.clocks_acc = 20;
    core.memory.io_ports[IE_IO_INDEX] = pending;
    core.memory.io_ports[IF_IO_INDEX] = pending | 0xe0;
    core
}

#[test]
fn entry_detection_disables_ime_without_pushing_or_acknowledging() {
    let mut core = interrupt_core(0x1234, 0xfffe, 1);
    core.cpu_registers.ime_enable_delay = 1;
    assert_eq!(handle_interrupts(&mut core), InterruptAction::Service);
    assert!(!core.cpu_registers.ime);
    assert_eq!(core.cpu_registers.ime_enable_delay, 0);
    assert_eq!(core.cpu_registers.pc, 0x1234);
    assert_eq!(core.cpu_registers.sp, 0xfffe);
    assert_eq!(core.memory.io_ports[IF_IO_INDEX], 0xe1);
    assert_eq!(core.cpu_timing.system_counter, 0);
}

#[test]
fn low_stack_write_resets_div_after_three_elapsed_machine_cycles() {
    let mut core = interrupt_core(0x1234, 0xff06, 1);
    dispatch(&mut core);
    // High byte goes to TIMA in M3; low byte resets DIV in M4. Only M4-M5
    // advance the counter after that reset, rather than all five cycles.
    assert_eq!(core.memory.io_ports[0x05], 0x12);
    assert_eq!(core.cpu_timing.system_counter, 8);
    assert_eq!(core.cpu_timing.clocks_acc, 0);
    assert_eq!(core.cpu_registers.pc, 0x40);
}

#[test]
fn timer_reload_during_wait_cycles_precedes_stack_writes() {
    let mut core = interrupt_core(0x1234, 0xff07, 1);
    core.memory.io_ports[0x06] = 0x88;
    core.cpu_timing.tima_reload_delay = 1;
    dispatch(&mut core);
    assert_eq!(core.memory.io_ports[0x06], 0x12);
    assert_eq!(core.memory.io_ports[0x05], 0x34);
    // Acknowledge VBlank without losing the timer interrupt raised in M1.
    assert_eq!(core.memory.io_ports[IF_IO_INDEX], 0xe4);
}

#[test]
fn dma_restrictions_are_checked_separately_for_each_stack_byte() {
    let mut core = interrupt_core(0x1234, 0xc002, 1);
    core.memory.wram[0] = 0xaa;
    core.memory.wram[1] = 0xbb;
    core.dma.oam.active = true;
    core.dma.oam.next_index = 157;
    dispatch(&mut core);
    // DMA ends during M3: the high-byte write was blocked, but M4 succeeds.
    assert!(!core.dma.oam.active);
    assert_eq!(core.memory.wram[1], 0xbb);
    assert_eq!(core.memory.wram[0], 0x34);
}

#[test]
fn high_stack_byte_can_cancel_interrupt_by_overwriting_ie() {
    let mut core = interrupt_core(0x0234, 0x0000, 4);
    dispatch(&mut core);
    assert_eq!(core.cpu_registers.pc, 0);
    assert_eq!(core.cpu_registers.sp, 0xfffe);
    assert_eq!(core.memory.io_ports[IE_IO_INDEX], 2);
    assert_eq!(core.memory.io_ports[IF_IO_INDEX], 0xe4);
    assert_eq!(core.memory.io_ports[0xfe], 0x34);
    assert!(!core.cpu_registers.ime);
    assert_eq!(core.cpu_timing.system_counter, 20);
}

#[test]
fn high_stack_byte_can_change_interrupt_priority_by_overwriting_ie() {
    let mut core = interrupt_core(0x0234, 0, 3);
    dispatch(&mut core);
    assert_eq!(core.cpu_registers.pc, 0x48);
    assert_eq!(core.memory.io_ports[IF_IO_INDEX], 0xe1);
}

#[test]
fn low_stack_byte_overwriting_ie_is_too_late_to_cancel_interrupt() {
    let mut core = interrupt_core(0x1235, 1, 8);
    dispatch(&mut core);
    assert_eq!(core.cpu_registers.pc, 0x58);
    assert_eq!(core.memory.io_ports[IE_IO_INDEX], 0x35);
    assert_eq!(core.memory.io_ports[IF_IO_INDEX], 0xe0);
    assert_eq!(core.cpu_registers.sp, 0xffff);
}

#[test]
fn interrupt_raised_during_wait_cycles_can_take_priority() {
    let mut core = interrupt_core(0x1234, 0xfffe, 8);
    core.memory.io_ports[IE_IO_INDEX] |= 4;
    core.cpu_timing.tima_reload_delay = 1;
    dispatch(&mut core);
    assert_eq!(core.cpu_registers.pc, 0x50);
    assert_eq!(core.memory.io_ports[IF_IO_INDEX], 0xe8);
    assert_eq!(core.memory.io_ports[0xfd], 0x12);
    assert_eq!(core.memory.io_ports[0xfc], 0x34);
}

#[test]
fn interrupt_requested_again_after_acknowledgement_remains_pending() {
    let mut core = interrupt_core(0x1234, 0xfffe, 4);
    core.cpu_timing.tima_reload_delay = 4;
    dispatch(&mut core);
    assert_eq!(core.cpu_registers.pc, 0x50);
    assert_eq!(core.memory.io_ports[IF_IO_INDEX], 0xe4);
}

#[test]
fn stack_write_to_dma_register_starts_transfer_before_next_stack_write() {
    let mut core = interrupt_core(0xc034, 0xff47, 1);
    dispatch(&mut core);
    assert_eq!(core.memory.io_ports[0x46], 0xc0);
    // IO remains available during DMA, including M4's low-byte LYC write.
    assert_eq!(core.memory.io_ports[0x45], 0x34);
    assert!(core.dma.oam.active);
    assert_eq!(core.dma.oam.next_index, 1);
    assert_eq!(core.dma.oam.pending_source_high, None);
}
