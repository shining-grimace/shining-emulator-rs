use bevy::prelude::warn;

use crate::game_boy::emulator::GameBoyCore;
use crate::game_boy::emulator::bus::{read8, read16, write8, write16};
use crate::game_boy::emulator::cpu::CpuMode;

const FLAG_Z: u8 = 0x80;
const FLAG_N: u8 = 0x40;
const FLAG_H: u8 = 0x20;
const FLAG_C: u8 = 0x10;
const IF_IO_INDEX: usize = 0x0f;
const IE_IO_INDEX: usize = 0xff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Reg8 {
    B,
    C,
    D,
    E,
    H,
    L,
    Hl,
    A,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Reg16 {
    Bc,
    De,
    Hl,
    Sp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Condition {
    Nz,
    Z,
    Nc,
    C,
}

pub(crate) fn perform_op(core: &mut GameBoyCore) -> i32 {
    let consume_halt_bug = core.cpu_registers.halt_bug;
    let cycles = perform_op_inner(core);
    if consume_halt_bug {
        core.cpu_registers.halt_bug = false;
    }
    cycles
}

fn perform_op_inner(core: &mut GameBoyCore) -> i32 {
    if core.cpu_mode == CpuMode::Halted {
        return 4;
    }

    let pc = pc(core);
    let opcode = read8(core, pc);

    if let Some(cycles) = execute_regular_family(core, opcode) {
        return cycles;
    }

    match opcode {
        0x00 => {
            advance_pc(core, 1);
            4
        }
        0x07 => {
            let carry = core.cpu_registers.a & 0x80 != 0;
            core.cpu_registers.a = core.cpu_registers.a.rotate_left(1);
            core.cpu_registers.f = if carry { FLAG_C } else { 0 };
            advance_pc(core, 1);
            4
        }
        0x08 => {
            let address = imm16(core);
            write16(core, address, core.cpu_registers.sp as u16);
            advance_pc(core, 3);
            20
        }
        0x0f => {
            let carry = core.cpu_registers.a & 0x01 != 0;
            core.cpu_registers.a = core.cpu_registers.a.rotate_right(1);
            core.cpu_registers.f = if carry { FLAG_C } else { 0 };
            advance_pc(core, 1);
            4
        }
        0x10 => {
            core.cpu_mode = CpuMode::Stopped;
            advance_pc(core, 1);
            4
        }
        0x17 => {
            let old_carry = carry(core);
            let new_carry = core.cpu_registers.a & 0x80 != 0;
            core.cpu_registers.a = (core.cpu_registers.a << 1) | u8::from(old_carry);
            core.cpu_registers.f = if new_carry { FLAG_C } else { 0 };
            advance_pc(core, 1);
            4
        }
        0x18 => {
            relative_jump(core);
            12
        }
        0x1f => {
            let old_carry = carry(core);
            let new_carry = core.cpu_registers.a & 0x01 != 0;
            core.cpu_registers.a = (core.cpu_registers.a >> 1) | if old_carry { 0x80 } else { 0 };
            core.cpu_registers.f = if new_carry { FLAG_C } else { 0 };
            advance_pc(core, 1);
            4
        }
        0x20 => conditional_relative_jump(core, Condition::Nz),
        0x27 => {
            daa(core);
            advance_pc(core, 1);
            4
        }
        0x28 => conditional_relative_jump(core, Condition::Z),
        0x2f => {
            core.cpu_registers.a = !core.cpu_registers.a;
            core.cpu_registers.f = (core.cpu_registers.f & (FLAG_Z | FLAG_C)) | FLAG_N | FLAG_H;
            advance_pc(core, 1);
            4
        }
        0x30 => conditional_relative_jump(core, Condition::Nc),
        0x37 => {
            core.cpu_registers.f = (core.cpu_registers.f & FLAG_Z) | FLAG_C;
            advance_pc(core, 1);
            4
        }
        0x38 => conditional_relative_jump(core, Condition::C),
        0x3f => {
            let next_carry = !carry(core);
            core.cpu_registers.f &= FLAG_Z;
            if next_carry {
                core.cpu_registers.f |= FLAG_C;
            }
            advance_pc(core, 1);
            4
        }
        0x76 => {
            let halt_bug = !core.cpu_registers.ime
                && core.cpu_registers.ime_enable_delay != 1
                && interrupt_pending(core);
            if halt_bug {
                advance_pc(core, 1);
                core.cpu_registers.halt_bug = true;
            } else {
                core.cpu_mode = CpuMode::Halted;
                advance_pc(core, 1);
            }
            4
        }
        0xc0 => conditional_return(core, Condition::Nz),
        0xc1 => pop_rr(core, Reg16::Bc),
        0xc2 => conditional_absolute_jump(core, Condition::Nz),
        0xc3 => {
            core.cpu_registers.pc = u32::from(imm16(core));
            16
        }
        0xc4 => conditional_call(core, Condition::Nz),
        0xc5 => push_rr(core, Reg16::Bc),
        0xc6 => {
            let value = imm8(core);
            add_a(core, value, false);
            advance_pc(core, 2);
            8
        }
        0xc7 => restart(core, 0x00),
        0xc8 => conditional_return(core, Condition::Z),
        0xc9 => return_from_call(core, false),
        0xca => conditional_absolute_jump(core, Condition::Z),
        0xcb => execute_cb(core),
        0xcc => conditional_call(core, Condition::Z),
        0xcd => call(core),
        0xce => {
            let value = imm8(core);
            add_a(core, value, carry(core));
            advance_pc(core, 2);
            8
        }
        0xcf => restart(core, 0x08),
        0xd0 => conditional_return(core, Condition::Nc),
        0xd1 => pop_rr(core, Reg16::De),
        0xd2 => conditional_absolute_jump(core, Condition::Nc),
        0xd3 => invalid_instruction(core, opcode),
        0xd4 => conditional_call(core, Condition::Nc),
        0xd5 => push_rr(core, Reg16::De),
        0xd6 => {
            let value = imm8(core);
            sub_a(core, value, false);
            advance_pc(core, 2);
            8
        }
        0xd7 => restart(core, 0x10),
        0xd8 => conditional_return(core, Condition::C),
        0xd9 => return_from_call(core, true),
        0xda => conditional_absolute_jump(core, Condition::C),
        0xdb => invalid_instruction(core, opcode),
        0xdc => conditional_call(core, Condition::C),
        0xdd => invalid_instruction(core, opcode),
        0xde => {
            let value = imm8(core);
            sub_a(core, value, carry(core));
            advance_pc(core, 2);
            8
        }
        0xdf => restart(core, 0x18),
        0xe0 => {
            let address = 0xff00 | u16::from(imm8(core));
            write8(core, address, core.cpu_registers.a);
            advance_pc(core, 2);
            12
        }
        0xe1 => pop_rr(core, Reg16::Hl),
        0xe2 => {
            let address = 0xff00 | u16::from(core.cpu_registers.c);
            write8(core, address, core.cpu_registers.a);
            advance_pc(core, 1);
            8
        }
        0xe3 | 0xe4 => invalid_instruction(core, opcode),
        0xe5 => push_rr(core, Reg16::Hl),
        0xe6 => {
            let value = imm8(core);
            and_a(core, value);
            advance_pc(core, 2);
            8
        }
        0xe7 => restart(core, 0x20),
        0xe8 => {
            add_signed_to_sp(core);
            advance_pc(core, 2);
            16
        }
        0xe9 => {
            core.cpu_registers.pc = u32::from(hl(core));
            4
        }
        0xea => {
            let address = imm16(core);
            write8(core, address, core.cpu_registers.a);
            advance_pc(core, 3);
            16
        }
        0xeb | 0xec | 0xed => invalid_instruction(core, opcode),
        0xee => {
            let value = imm8(core);
            xor_a(core, value);
            advance_pc(core, 2);
            8
        }
        0xef => restart(core, 0x28),
        0xf0 => {
            let address = 0xff00 | u16::from(imm8(core));
            core.cpu_registers.a = read8(core, address);
            advance_pc(core, 2);
            12
        }
        0xf1 => {
            let value = pop(core);
            set_af(core, value);
            advance_pc(core, 1);
            12
        }
        0xf2 => {
            let address = 0xff00 | u16::from(core.cpu_registers.c);
            core.cpu_registers.a = read8(core, address);
            advance_pc(core, 1);
            8
        }
        0xf3 => {
            core.cpu_registers.ime = false;
            core.cpu_registers.ime_enable_delay = 0;
            advance_pc(core, 1);
            4
        }
        0xf4 => invalid_instruction(core, opcode),
        0xf5 => push_af(core),
        0xf6 => {
            let value = imm8(core);
            or_a(core, value);
            advance_pc(core, 2);
            8
        }
        0xf7 => restart(core, 0x30),
        0xf8 => {
            let result = sp_plus_signed(core);
            set_hl(core, result);
            advance_pc(core, 2);
            12
        }
        0xf9 => {
            core.cpu_registers.sp = u32::from(hl(core));
            advance_pc(core, 1);
            8
        }
        0xfa => {
            let address = imm16(core);
            core.cpu_registers.a = read8(core, address);
            advance_pc(core, 3);
            16
        }
        0xfb => {
            core.cpu_registers.ime_enable_delay = 2;
            advance_pc(core, 1);
            4
        }
        0xfc | 0xfd => invalid_instruction(core, opcode),
        0xfe => {
            let value = imm8(core);
            cp_a(core, value);
            advance_pc(core, 2);
            8
        }
        0xff => restart(core, 0x38),
        _ => invalid_instruction(core, opcode),
    }
}

fn execute_regular_family(core: &mut GameBoyCore, opcode: u8) -> Option<i32> {
    if matches!(opcode, 0x01 | 0x11 | 0x21 | 0x31) {
        let register = reg16_from_column(opcode >> 4);
        let value = imm16(core);
        write_reg16(core, register, value);
        advance_pc(core, 3);
        return Some(12);
    }
    if matches!(opcode, 0x02 | 0x12) {
        let address = if opcode == 0x02 { bc(core) } else { de(core) };
        write8(core, address, core.cpu_registers.a);
        advance_pc(core, 1);
        return Some(8);
    }
    if matches!(opcode, 0x03 | 0x13 | 0x23 | 0x33) {
        let register = reg16_from_column(opcode >> 4);
        let value = read_reg16(core, register).wrapping_add(1);
        write_reg16(core, register, value);
        advance_pc(core, 1);
        return Some(8);
    }
    if matches!(
        opcode,
        0x04 | 0x0c | 0x14 | 0x1c | 0x24 | 0x2c | 0x34 | 0x3c
    ) {
        let register = reg8_from_opcode_middle(opcode);
        let value = read_reg8(core, register);
        let result = inc8(core, value);
        write_reg8(core, register, result);
        advance_pc(core, 1);
        return Some(if register == Reg8::Hl { 12 } else { 4 });
    }
    if matches!(
        opcode,
        0x05 | 0x0d | 0x15 | 0x1d | 0x25 | 0x2d | 0x35 | 0x3d
    ) {
        let register = reg8_from_opcode_middle(opcode);
        let value = read_reg8(core, register);
        let result = dec8(core, value);
        write_reg8(core, register, result);
        advance_pc(core, 1);
        return Some(if register == Reg8::Hl { 12 } else { 4 });
    }
    if matches!(
        opcode,
        0x06 | 0x0e | 0x16 | 0x1e | 0x26 | 0x2e | 0x36 | 0x3e
    ) {
        let register = reg8_from_opcode_middle(opcode);
        let value = imm8(core);
        write_reg8(core, register, value);
        advance_pc(core, 2);
        return Some(if register == Reg8::Hl { 12 } else { 8 });
    }
    if matches!(opcode, 0x09 | 0x19 | 0x29 | 0x39) {
        let register = reg16_from_column(opcode >> 4);
        let value = read_reg16(core, register);
        add_hl(core, value);
        advance_pc(core, 1);
        return Some(8);
    }
    if matches!(opcode, 0x0a | 0x1a) {
        let address = if opcode == 0x0a { bc(core) } else { de(core) };
        core.cpu_registers.a = read8(core, address);
        advance_pc(core, 1);
        return Some(8);
    }
    if matches!(opcode, 0x0b | 0x1b | 0x2b | 0x3b) {
        let register = reg16_from_column(opcode >> 4);
        let value = read_reg16(core, register).wrapping_sub(1);
        write_reg16(core, register, value);
        advance_pc(core, 1);
        return Some(8);
    }
    if matches!(opcode, 0x22 | 0x2a | 0x32 | 0x3a) {
        let address = hl(core);
        match opcode {
            0x22 => write8(core, address, core.cpu_registers.a),
            0x2a => core.cpu_registers.a = read8(core, address),
            0x32 => write8(core, address, core.cpu_registers.a),
            _ => core.cpu_registers.a = read8(core, address),
        }
        let next_hl = if matches!(opcode, 0x22 | 0x2a) {
            address.wrapping_add(1)
        } else {
            address.wrapping_sub(1)
        };
        set_hl(core, next_hl);
        advance_pc(core, 1);
        return Some(8);
    }
    if (0x40..=0x7f).contains(&opcode) && opcode != 0x76 {
        let destination = reg8_from_index((opcode >> 3) & 0x07);
        let source = reg8_from_index(opcode & 0x07);
        let value = read_reg8(core, source);
        write_reg8(core, destination, value);
        advance_pc(core, 1);
        return Some(if destination == Reg8::Hl || source == Reg8::Hl {
            8
        } else {
            4
        });
    }
    if (0x80..=0xbf).contains(&opcode) {
        let source = reg8_from_index(opcode & 0x07);
        let value = read_reg8(core, source);
        match (opcode - 0x80) / 8 {
            0 => add_a(core, value, false),
            1 => add_a(core, value, carry(core)),
            2 => sub_a(core, value, false),
            3 => sub_a(core, value, carry(core)),
            4 => and_a(core, value),
            5 => xor_a(core, value),
            6 => or_a(core, value),
            _ => cp_a(core, value),
        }
        advance_pc(core, 1);
        return Some(if source == Reg8::Hl { 8 } else { 4 });
    }
    None
}

fn execute_cb(core: &mut GameBoyCore) -> i32 {
    let opcode = read8(core, operand_address(core, 1));
    let register = reg8_from_index(opcode & 0x07);
    let mut value = read_reg8(core, register);

    match opcode {
        0x00..=0x07 => value = rotate_left_circular(core, value, true),
        0x08..=0x0f => value = rotate_right_circular(core, value, true),
        0x10..=0x17 => value = rotate_left_through_carry(core, value, true),
        0x18..=0x1f => value = rotate_right_through_carry(core, value, true),
        0x20..=0x27 => value = shift_left_arithmetic(core, value),
        0x28..=0x2f => value = shift_right_arithmetic(core, value),
        0x30..=0x37 => value = swap_nibbles(core, value),
        0x38..=0x3f => value = shift_right_logical(core, value),
        0x40..=0x7f => {
            test_bit(core, value, (opcode - 0x40) / 8);
            advance_pc(core, 2);
            return if register == Reg8::Hl { 12 } else { 8 };
        }
        0x80..=0xbf => value &= !(1_u8 << ((opcode - 0x80) / 8)),
        _ => value |= 1_u8 << ((opcode - 0xc0) / 8),
    }

    write_reg8(core, register, value);
    advance_pc(core, 2);
    if register == Reg8::Hl { 16 } else { 8 }
}

fn pc(core: &GameBoyCore) -> u16 {
    core.cpu_registers.pc as u16
}

fn advance_pc(core: &mut GameBoyCore, amount: u16) {
    core.cpu_registers.pc = u32::from(pc_after_instruction(core, amount));
}

fn pc_after_instruction(core: &GameBoyCore, amount: u16) -> u16 {
    let halt_bug_adjustment = u16::from(core.cpu_registers.halt_bug);
    pc(core).wrapping_add(amount.saturating_sub(halt_bug_adjustment))
}

fn operand_address(core: &GameBoyCore, offset: u16) -> u16 {
    let halt_bug_adjustment = u16::from(core.cpu_registers.halt_bug);
    pc(core).wrapping_add(offset.saturating_sub(halt_bug_adjustment))
}

fn imm8(core: &GameBoyCore) -> u8 {
    read8(core, operand_address(core, 1))
}

fn imm16(core: &GameBoyCore) -> u16 {
    u16::from(read8(core, operand_address(core, 1)))
        | (u16::from(read8(core, operand_address(core, 2))) << 8)
}

fn signed_imm8(core: &GameBoyCore) -> i8 {
    imm8(core) as i8
}

fn relative_jump(core: &mut GameBoyCore) {
    let offset = i16::from(signed_imm8(core));
    let next_pc = pc_after_instruction(core, 2).wrapping_add_signed(offset);
    core.cpu_registers.pc = u32::from(next_pc);
}

fn conditional_relative_jump(core: &mut GameBoyCore, condition: Condition) -> i32 {
    if condition_met(core, condition) {
        relative_jump(core);
        12
    } else {
        advance_pc(core, 2);
        8
    }
}

fn conditional_absolute_jump(core: &mut GameBoyCore, condition: Condition) -> i32 {
    if condition_met(core, condition) {
        core.cpu_registers.pc = u32::from(imm16(core));
        16
    } else {
        advance_pc(core, 3);
        12
    }
}

fn condition_met(core: &GameBoyCore, condition: Condition) -> bool {
    match condition {
        Condition::Nz => core.cpu_registers.f & FLAG_Z == 0,
        Condition::Z => core.cpu_registers.f & FLAG_Z != 0,
        Condition::Nc => core.cpu_registers.f & FLAG_C == 0,
        Condition::C => core.cpu_registers.f & FLAG_C != 0,
    }
}

fn call(core: &mut GameBoyCore) -> i32 {
    let target = imm16(core);
    let return_address = pc_after_instruction(core, 3);
    push(core, return_address);
    core.cpu_registers.pc = u32::from(target);
    24
}

fn conditional_call(core: &mut GameBoyCore, condition: Condition) -> i32 {
    if condition_met(core, condition) {
        call(core)
    } else {
        advance_pc(core, 3);
        12
    }
}

fn return_from_call(core: &mut GameBoyCore, enable_interrupts: bool) -> i32 {
    core.cpu_registers.pc = u32::from(pop(core));
    if enable_interrupts {
        core.cpu_registers.ime = true;
        core.cpu_registers.ime_enable_delay = 0;
    }
    16
}

fn conditional_return(core: &mut GameBoyCore, condition: Condition) -> i32 {
    if condition_met(core, condition) {
        core.cpu_registers.pc = u32::from(pop(core));
        20
    } else {
        advance_pc(core, 1);
        8
    }
}

fn restart(core: &mut GameBoyCore, address: u16) -> i32 {
    let return_address = pc_after_instruction(core, 1);
    push(core, return_address);
    core.cpu_registers.pc = u32::from(address);
    16
}

fn push_rr(core: &mut GameBoyCore, register: Reg16) -> i32 {
    let value = read_reg16(core, register);
    push(core, value);
    advance_pc(core, 1);
    16
}

fn push_af(core: &mut GameBoyCore) -> i32 {
    push(core, af(core));
    advance_pc(core, 1);
    16
}

fn pop_rr(core: &mut GameBoyCore, register: Reg16) -> i32 {
    let value = pop(core);
    write_reg16(core, register, value);
    advance_pc(core, 1);
    12
}

fn push(core: &mut GameBoyCore, value: u16) {
    let sp = (core.cpu_registers.sp as u16).wrapping_sub(2);
    core.cpu_registers.sp = u32::from(sp);
    write16(core, sp, value);
}

fn pop(core: &mut GameBoyCore) -> u16 {
    let sp = core.cpu_registers.sp as u16;
    let value = read16(core, sp);
    core.cpu_registers.sp = u32::from(sp.wrapping_add(2));
    value
}

fn reg8_from_opcode_middle(opcode: u8) -> Reg8 {
    reg8_from_index((opcode >> 3) & 0x07)
}

fn reg8_from_index(index: u8) -> Reg8 {
    match index {
        0 => Reg8::B,
        1 => Reg8::C,
        2 => Reg8::D,
        3 => Reg8::E,
        4 => Reg8::H,
        5 => Reg8::L,
        6 => Reg8::Hl,
        _ => Reg8::A,
    }
}

fn reg16_from_column(column: u8) -> Reg16 {
    match column {
        0 => Reg16::Bc,
        1 => Reg16::De,
        2 => Reg16::Hl,
        _ => Reg16::Sp,
    }
}

fn read_reg8(core: &GameBoyCore, register: Reg8) -> u8 {
    match register {
        Reg8::B => core.cpu_registers.b,
        Reg8::C => core.cpu_registers.c,
        Reg8::D => core.cpu_registers.d,
        Reg8::E => core.cpu_registers.e,
        Reg8::H => core.cpu_registers.h,
        Reg8::L => core.cpu_registers.l,
        Reg8::Hl => read8(core, hl(core)),
        Reg8::A => core.cpu_registers.a,
    }
}

fn write_reg8(core: &mut GameBoyCore, register: Reg8, value: u8) {
    match register {
        Reg8::B => core.cpu_registers.b = value,
        Reg8::C => core.cpu_registers.c = value,
        Reg8::D => core.cpu_registers.d = value,
        Reg8::E => core.cpu_registers.e = value,
        Reg8::H => core.cpu_registers.h = value,
        Reg8::L => core.cpu_registers.l = value,
        Reg8::Hl => {
            let address = hl(core);
            write8(core, address, value);
        }
        Reg8::A => core.cpu_registers.a = value,
    }
}

fn read_reg16(core: &GameBoyCore, register: Reg16) -> u16 {
    match register {
        Reg16::Bc => bc(core),
        Reg16::De => de(core),
        Reg16::Hl => hl(core),
        Reg16::Sp => core.cpu_registers.sp as u16,
    }
}

fn write_reg16(core: &mut GameBoyCore, register: Reg16, value: u16) {
    match register {
        Reg16::Bc => set_bc(core, value),
        Reg16::De => set_de(core, value),
        Reg16::Hl => set_hl(core, value),
        Reg16::Sp => core.cpu_registers.sp = u32::from(value),
    }
}

fn bc(core: &GameBoyCore) -> u16 {
    u16::from_be_bytes([core.cpu_registers.b, core.cpu_registers.c])
}

fn de(core: &GameBoyCore) -> u16 {
    u16::from_be_bytes([core.cpu_registers.d, core.cpu_registers.e])
}

fn hl(core: &GameBoyCore) -> u16 {
    u16::from_be_bytes([core.cpu_registers.h, core.cpu_registers.l])
}

fn af(core: &GameBoyCore) -> u16 {
    u16::from_be_bytes([core.cpu_registers.a, core.cpu_registers.f & 0xf0])
}

fn set_bc(core: &mut GameBoyCore, value: u16) {
    let [b, c] = value.to_be_bytes();
    core.cpu_registers.b = b;
    core.cpu_registers.c = c;
}

fn set_de(core: &mut GameBoyCore, value: u16) {
    let [d, e] = value.to_be_bytes();
    core.cpu_registers.d = d;
    core.cpu_registers.e = e;
}

fn set_hl(core: &mut GameBoyCore, value: u16) {
    let [h, l] = value.to_be_bytes();
    core.cpu_registers.h = h;
    core.cpu_registers.l = l;
}

fn set_af(core: &mut GameBoyCore, value: u16) {
    let [a, f] = value.to_be_bytes();
    core.cpu_registers.a = a;
    core.cpu_registers.f = f & 0xf0;
}

fn carry(core: &GameBoyCore) -> bool {
    core.cpu_registers.f & FLAG_C != 0
}

fn set_znhc(core: &mut GameBoyCore, z: bool, n: bool, h: bool, c: bool) {
    core.cpu_registers.f = 0;
    if z {
        core.cpu_registers.f |= FLAG_Z;
    }
    if n {
        core.cpu_registers.f |= FLAG_N;
    }
    if h {
        core.cpu_registers.f |= FLAG_H;
    }
    if c {
        core.cpu_registers.f |= FLAG_C;
    }
}

fn inc8(core: &mut GameBoyCore, value: u8) -> u8 {
    let result = value.wrapping_add(1);
    let carry = core.cpu_registers.f & FLAG_C;
    core.cpu_registers.f = carry;
    if result == 0 {
        core.cpu_registers.f |= FLAG_Z;
    }
    if value & 0x0f == 0x0f {
        core.cpu_registers.f |= FLAG_H;
    }
    result
}

fn dec8(core: &mut GameBoyCore, value: u8) -> u8 {
    let result = value.wrapping_sub(1);
    let carry = core.cpu_registers.f & FLAG_C;
    core.cpu_registers.f = carry | FLAG_N;
    if result == 0 {
        core.cpu_registers.f |= FLAG_Z;
    }
    if value & 0x0f == 0 {
        core.cpu_registers.f |= FLAG_H;
    }
    result
}

fn add_hl(core: &mut GameBoyCore, value: u16) {
    let lhs = hl(core);
    let result = lhs.wrapping_add(value);
    let z = core.cpu_registers.f & FLAG_Z != 0;
    set_znhc(
        core,
        z,
        false,
        (lhs & 0x0fff) + (value & 0x0fff) > 0x0fff,
        u32::from(lhs) + u32::from(value) > 0xffff,
    );
    set_hl(core, result);
}

fn add_a(core: &mut GameBoyCore, value: u8, carry_in: bool) {
    let carry_value = u8::from(carry_in);
    let lhs = core.cpu_registers.a;
    let result = lhs.wrapping_add(value).wrapping_add(carry_value);
    set_znhc(
        core,
        result == 0,
        false,
        (lhs & 0x0f) + (value & 0x0f) + carry_value > 0x0f,
        u16::from(lhs) + u16::from(value) + u16::from(carry_value) > 0xff,
    );
    core.cpu_registers.a = result;
}

fn sub_a(core: &mut GameBoyCore, value: u8, carry_in: bool) {
    let carry_value = u8::from(carry_in);
    let lhs = core.cpu_registers.a;
    let result = lhs.wrapping_sub(value).wrapping_sub(carry_value);
    set_znhc(
        core,
        result == 0,
        true,
        (lhs & 0x0f) < ((value & 0x0f) + carry_value),
        u16::from(lhs) < u16::from(value) + u16::from(carry_value),
    );
    core.cpu_registers.a = result;
}

fn and_a(core: &mut GameBoyCore, value: u8) {
    core.cpu_registers.a &= value;
    set_znhc(core, core.cpu_registers.a == 0, false, true, false);
}

fn xor_a(core: &mut GameBoyCore, value: u8) {
    core.cpu_registers.a ^= value;
    set_znhc(core, core.cpu_registers.a == 0, false, false, false);
}

fn or_a(core: &mut GameBoyCore, value: u8) {
    core.cpu_registers.a |= value;
    set_znhc(core, core.cpu_registers.a == 0, false, false, false);
}

fn cp_a(core: &mut GameBoyCore, value: u8) {
    let lhs = core.cpu_registers.a;
    set_znhc(
        core,
        lhs == value,
        true,
        (value & 0x0f) > (lhs & 0x0f),
        value > lhs,
    );
}

fn daa(core: &mut GameBoyCore) {
    let mut correction = 0;
    let mut set_carry = false;
    let negative = core.cpu_registers.f & FLAG_N != 0;

    if core.cpu_registers.f & FLAG_H != 0 || (!negative && core.cpu_registers.a & 0x0f > 9) {
        correction |= 0x06;
    }
    if core.cpu_registers.f & FLAG_C != 0 || (!negative && core.cpu_registers.a > 0x99) {
        correction |= 0x60;
        set_carry = true;
    }

    if negative {
        core.cpu_registers.a = core.cpu_registers.a.wrapping_sub(correction);
    } else {
        core.cpu_registers.a = core.cpu_registers.a.wrapping_add(correction);
    }

    core.cpu_registers.f &= FLAG_N;
    if core.cpu_registers.a == 0 {
        core.cpu_registers.f |= FLAG_Z;
    }
    if set_carry {
        core.cpu_registers.f |= FLAG_C;
    }
}

fn add_signed_to_sp(core: &mut GameBoyCore) {
    let result = sp_plus_signed(core);
    core.cpu_registers.sp = u32::from(result);
}

fn sp_plus_signed(core: &mut GameBoyCore) -> u16 {
    let sp = core.cpu_registers.sp as u16;
    let offset = signed_imm8(core);
    let unsigned_offset = u16::from(offset as u8);
    let result = sp.wrapping_add_signed(i16::from(offset));
    set_znhc(
        core,
        false,
        false,
        (sp & 0x000f) + (unsigned_offset & 0x000f) > 0x000f,
        (sp & 0x00ff) + (unsigned_offset & 0x00ff) > 0x00ff,
    );
    result
}

fn rotate_left_circular(core: &mut GameBoyCore, value: u8, set_zero: bool) -> u8 {
    let result = value.rotate_left(1);
    set_znhc(
        core,
        set_zero && result == 0,
        false,
        false,
        value & 0x80 != 0,
    );
    result
}

fn rotate_right_circular(core: &mut GameBoyCore, value: u8, set_zero: bool) -> u8 {
    let result = value.rotate_right(1);
    set_znhc(
        core,
        set_zero && result == 0,
        false,
        false,
        value & 0x01 != 0,
    );
    result
}

fn rotate_left_through_carry(core: &mut GameBoyCore, value: u8, set_zero: bool) -> u8 {
    let result = (value << 1) | u8::from(carry(core));
    set_znhc(
        core,
        set_zero && result == 0,
        false,
        false,
        value & 0x80 != 0,
    );
    result
}

fn rotate_right_through_carry(core: &mut GameBoyCore, value: u8, set_zero: bool) -> u8 {
    let result = (value >> 1) | if carry(core) { 0x80 } else { 0 };
    set_znhc(
        core,
        set_zero && result == 0,
        false,
        false,
        value & 0x01 != 0,
    );
    result
}

fn shift_left_arithmetic(core: &mut GameBoyCore, value: u8) -> u8 {
    let result = value << 1;
    set_znhc(core, result == 0, false, false, value & 0x80 != 0);
    result
}

fn shift_right_arithmetic(core: &mut GameBoyCore, value: u8) -> u8 {
    let result = (value >> 1) | (value & 0x80);
    set_znhc(core, result == 0, false, false, value & 0x01 != 0);
    result
}

fn shift_right_logical(core: &mut GameBoyCore, value: u8) -> u8 {
    let result = value >> 1;
    set_znhc(core, result == 0, false, false, value & 0x01 != 0);
    result
}

fn swap_nibbles(core: &mut GameBoyCore, value: u8) -> u8 {
    let result = value.rotate_left(4);
    set_znhc(core, result == 0, false, false, false);
    result
}

fn test_bit(core: &mut GameBoyCore, value: u8, bit: u8) {
    let carry = core.cpu_registers.f & FLAG_C;
    core.cpu_registers.f = carry | FLAG_H;
    if value & (1_u8 << bit) == 0 {
        core.cpu_registers.f |= FLAG_Z;
    }
}

fn invalid_instruction(core: &mut GameBoyCore, opcode: u8) -> i32 {
    warn!("invalid Game Boy CPU instruction 0x{opcode:02x}");
    core.runtime.is_running = false;
    core.cpu_timing.clocks_acc.max(4)
}

fn interrupt_pending(core: &GameBoyCore) -> bool {
    let enabled = core.memory.io_ports.get(IE_IO_INDEX).copied().unwrap_or(0);
    let requested = core.memory.io_ports.get(IF_IO_INDEX).copied().unwrap_or(0);
    enabled & requested & 0x1f != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_adds_registers() {
        let mut core = GameBoyCore::default();
        core.memory.rom[0x0100] = 0x3e;
        core.memory.rom[0x0101] = 0x12;
        core.memory.rom[0x0102] = 0x06;
        core.memory.rom[0x0103] = 0x30;
        core.memory.rom[0x0104] = 0x80;
        core.cpu_registers.pc = 0x0100;

        assert_eq!(perform_op(&mut core), 8);
        assert_eq!(perform_op(&mut core), 8);
        assert_eq!(perform_op(&mut core), 4);

        assert_eq!(core.cpu_registers.a, 0x42);
        assert_eq!(core.cpu_registers.f, 0);
    }

    #[test]
    fn call_and_return_use_stack_memory() {
        let mut core = GameBoyCore::default();
        core.memory.rom[0x0100] = 0xcd;
        core.memory.rom[0x0101] = 0x00;
        core.memory.rom[0x0102] = 0x02;
        core.memory.rom[0x0200] = 0xc9;
        core.cpu_registers.pc = 0x0100;
        core.cpu_registers.sp = 0xfffe;

        assert_eq!(perform_op(&mut core), 24);
        assert_eq!(core.cpu_registers.pc, 0x0200);
        assert_eq!(perform_op(&mut core), 16);
        assert_eq!(core.cpu_registers.pc, 0x0103);
        assert_eq!(core.cpu_registers.sp, 0xfffe);
    }

    #[test]
    fn cb_bit_operation_preserves_carry_and_sets_half_carry() {
        let mut core = GameBoyCore::default();
        core.memory.rom[0x0100] = 0xcb;
        core.memory.rom[0x0101] = 0x7c;
        core.cpu_registers.pc = 0x0100;
        core.cpu_registers.h = 0x7f;
        core.cpu_registers.f = FLAG_C;

        assert_eq!(perform_op(&mut core), 8);

        assert_eq!(core.cpu_registers.f, FLAG_Z | FLAG_H | FLAG_C);
    }

    #[test]
    fn halt_with_disabled_ime_and_pending_interrupt_triggers_halt_bug() {
        let mut core = GameBoyCore::default();
        core.memory.rom[0x0100] = 0x76;
        core.memory.io_ports[IE_IO_INDEX] = 0x01;
        core.memory.io_ports[IF_IO_INDEX] = 0x01;
        core.cpu_registers.pc = 0x0100;

        assert_eq!(perform_op(&mut core), 4);

        assert_eq!(core.cpu_mode, CpuMode::Running);
        assert_eq!(core.cpu_registers.pc, 0x0101);
        assert!(core.cpu_registers.halt_bug);
    }

    #[test]
    fn halt_bug_shifts_immediate_operands_and_pc_advance() {
        let mut core = GameBoyCore::default();
        core.memory.rom[0x0100] = 0x06;
        core.memory.rom[0x0101] = 0x42;
        core.cpu_registers.pc = 0x0100;
        core.cpu_registers.halt_bug = true;

        assert_eq!(perform_op(&mut core), 8);

        assert_eq!(core.cpu_registers.b, 0x06);
        assert_eq!(core.cpu_registers.pc, 0x0101);
        assert!(!core.cpu_registers.halt_bug);
    }
}
