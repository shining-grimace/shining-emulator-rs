use super::{IE_IO_INDEX, IF_IO_INDEX, io, step_machine_cycle, write_io};
use crate::game_boy::emulator::GameBoyCore;
use crate::game_boy::emulator::bus::{begin_deferred_oam_dma, write8};
use crate::game_boy::emulator::cpu::CpuMode;
use crate::game_boy::frame_buffer::GameBoyFrameRing;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InterruptAction {
    None,
    WakeHalt,
    Service,
}

pub(super) fn handle_interrupts(emulator: &mut GameBoyCore) -> InterruptAction {
    if pending_interrupts(emulator) == 0 {
        return InterruptAction::None;
    }

    if emulator.cpu_mode == CpuMode::Halted {
        emulator.cpu_mode = CpuMode::Running;
        if !emulator.cpu_registers.ime {
            return InterruptAction::WakeHalt;
        }
    } else if !emulator.cpu_registers.ime {
        return InterruptAction::None;
    }

    emulator.cpu_registers.ime = false;
    emulator.cpu_registers.ime_enable_delay = 0;
    InterruptAction::Service
}

/// Advances peripherals between each interrupt-entry operation.
/// See <https://gbdev.io/pandocs/Interrupts.html> and the hardware-verified
/// `acceptance/interrupts/ie_push` test in the Mooneye test suite.
pub(super) fn service_interrupt(emulator: &mut GameBoyCore, frames: &mut GameBoyFrameRing) {
    let [pc_low, pc_high, _, _] = emulator.cpu_registers.pc.to_le_bytes();

    // M1-M2: internal wait cycles; the stack and IF have not changed yet.
    step_machine_cycle(emulator, frames);
    step_machine_cycle(emulator, frames);

    // M3: push the high byte first, using the current CPU bus restrictions.
    push_byte(emulator, frames, pc_high);

    // Select and acknowledge after the high-byte write, before the low-byte
    // write. An IE write during M3 can redirect or cancel dispatch; M4 cannot.
    let pending = pending_interrupts(emulator);
    let target = if pending == 0 {
        0
    } else {
        let index = pending.trailing_zeros();
        let requested = io(&emulator.memory, IF_IO_INDEX);
        write_io(&mut emulator.memory, IF_IO_INDEX, requested & !(1 << index));
        0x0040 + index * 8
    };

    // M4: push the low byte, even when dispatch has been cancelled.
    push_byte(emulator, frames, pc_low);

    // M5: transfer control; the handler opcode is fetched on the next step.
    emulator.cpu_registers.pc = target;
    step_machine_cycle(emulator, frames);
}

fn pending_interrupts(emulator: &GameBoyCore) -> u8 {
    io(&emulator.memory, IE_IO_INDEX) & io(&emulator.memory, IF_IO_INDEX) & 0x1f
}

fn push_byte(emulator: &mut GameBoyCore, frames: &mut GameBoyFrameRing, value: u8) {
    emulator.cpu_registers.sp = emulator.cpu_registers.sp.wrapping_sub(1) & 0xffff;
    let [low, high, _, _] = emulator.cpu_registers.sp.to_le_bytes();
    write8(emulator, u16::from_le_bytes([low, high]), value);
    step_machine_cycle(emulator, frames);
    // A stack targeting FF46 requests DMA just like an instruction's write.
    begin_deferred_oam_dma(emulator);
}

#[cfg(test)]
mod tests;
