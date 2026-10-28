use super::*;

fn program(bytes: &[u8]) -> GameBoyCore {
    let mut core = GameBoyCore::default();
    core.cpu_registers.pc = 0x0100;
    core.cpu_registers.sp = 0xd000;
    set_hl(&mut core, 0xc000);
    core.memory.rom[0x0100..0x0100 + bytes.len()].copy_from_slice(bytes);
    core
}

fn execute(core: &mut GameBoyCore) -> i32 {
    perform_op(core, &mut GameBoyFrameRing::default())
}

#[test]
fn timer_read_observes_cycles_spent_fetching_opcode_and_operand() {
    let mut core = program(&[0xf0, 0x04]); // LDH A,(DIV)
    core.cpu_timing.system_counter = 0x00f8;
    assert_eq!(execute(&mut core), 12);
    assert_eq!(core.cpu_registers.a, 1);
    assert_eq!(core.cpu_timing.system_counter, 0x0104);
}

#[test]
fn timer_write_occurs_in_its_own_bus_cycle() {
    let mut core = program(&[0xe0, 0x04]); // LDH (DIV),A
    assert_eq!(execute(&mut core), 12);
    assert_eq!(core.cpu_timing.system_counter, 4);
}

#[test]
fn read_modify_write_can_cancel_an_overflow_between_its_bus_accesses() {
    let mut core = program(&[0x34]); // INC (HL), targeting TIMA
    set_hl(&mut core, 0xff05);
    core.cpu_timing.system_counter = 8;
    core.memory.io_ports[0x05] = 0xff;
    core.memory.io_ports[0x07] = 5;
    assert_eq!(execute(&mut core), 12);
    assert_eq!(core.memory.io_ports[0x05], 0);
    assert_eq!(core.cpu_timing.tima_reload_delay, 0);
    assert_eq!(core.memory.io_ports[IF_IO_INDEX] & 4, 0);
}

#[test]
fn sixteen_bit_store_uses_distinct_low_and_high_byte_cycles() {
    let mut core = program(&[0x08, 0x04, 0xff]); // LD (DIV),SP
    core.cpu_registers.sp = 0x1234;
    assert_eq!(execute(&mut core), 20);
    assert_eq!(core.cpu_timing.system_counter, 8);
    assert_eq!(core.memory.io_ports[0x05], 0x12);
}

#[test]
fn call_waits_before_pushing_high_then_low_stack_bytes() {
    let mut core = program(&[0xcd, 0x00, 0x02]);
    core.cpu_registers.sp = 0xff06; // High -> TIMA, low -> DIV
    core.memory.io_ports[0x07] = 5;
    assert_eq!(execute(&mut core), 24);
    assert_eq!(core.cpu_registers.pc, 0x0200);
    assert_eq!(core.cpu_timing.system_counter, 4);
    assert_eq!(core.memory.io_ports[0x05], 1);
}

#[test]
fn conditional_return_waits_before_reading_the_stack() {
    let mut core = program(&[0xc0]); // RET NZ, taken
    core.cpu_registers.sp = 0xff04; // Low <- DIV, high <- TIMA
    core.cpu_timing.system_counter = 0x00f8;
    core.memory.io_ports[0x05] = 0x12;
    assert_eq!(execute(&mut core), 20);
    assert_eq!(core.cpu_registers.pc, 0x1201);
}

#[test]
fn dma_can_release_the_bus_between_operand_fetch_and_data_read() {
    let mut core = program(&[]);
    core.cpu_registers.pc = 0xff80;
    core.memory.io_ports[0x80..0x83].copy_from_slice(&[0xfa, 0x00, 0xc0]);
    core.memory.wram[0] = 0x77;
    core.dma.oam.active = true;
    core.dma.oam.next_index = 157;
    assert_eq!(execute(&mut core), 16);
    assert_eq!(core.cpu_registers.a, 0x77);
    assert!(!core.dma.oam.active);
}

#[test]
fn vram_dma_requested_mid_instruction_stalls_the_next_bus_access() {
    let mut core = program(&[0x08, 0x55, 0xff]); // LD (HDMA5),SP
    core.rom.properties.cgb_flag = true;
    core.memory.io_ports[0x55] = 0xff;
    core.cpu_registers.sp = 0x1200; // Low byte requests one GDMA block.
    assert_eq!(execute(&mut core), 20);
    assert_eq!(core.dma.vram.cpu_halt_m_cycles, 0);
    assert_eq!(core.cpu_timing.system_counter, 52);
}

#[test]
fn every_opcode_path_accounts_for_its_bus_and_internal_cycles() {
    // Exercise both outcomes of every conditional instruction, and all CB
    // register/memory variants. The decoder checks its declared clock total
    // against the cycles actually consumed, without padding missing accesses.
    for flags in [0, FLAG_Z | FLAG_C] {
        for opcode in 0..=u8::MAX {
            let mut core = program(&[opcode, 0, 0]);
            core.cpu_registers.f = flags;
            execute(&mut core);
            let mut core = program(&[0xcb, opcode]);
            core.cpu_registers.f = flags;
            execute(&mut core);
        }
    }
}
