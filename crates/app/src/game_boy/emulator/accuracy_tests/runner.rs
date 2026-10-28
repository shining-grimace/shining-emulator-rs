use super::{GameBoyCore, Status};
use crate::game_boy::emulator::bus::read8;
use crate::game_boy::emulator::cpu::CpuMode;
use crate::game_boy::emulator::systems::execute_next_test_step;
use crate::game_boy::frame_buffer::GameBoyFrameRing;

pub(super) fn run(core: &mut GameBoyCore, max_steps: usize) -> MooneyeResult {
    let mut frames = GameBoyFrameRing::default();
    for steps in 0..max_steps {
        if !core.runtime.is_running {
            return MooneyeResult::InvalidOpcode {
                steps,
                pc: core.cpu_registers.pc,
            };
        }
        let interrupt_pending = core.cpu_registers.ime
            && core.memory.io_ports[0xff] & core.memory.io_ports[0x0f] & 0x1f != 0;
        if core.cpu_mode == CpuMode::Running
            && !interrupt_pending
            && core.dma.vram.cpu_halt_m_cycles == 0
            && read8(
                core,
                u16::try_from(core.cpu_registers.pc).expect("PC fits u16"),
            ) == 0x40
        {
            let registers = RegisterSignature::from_core(core);
            match registers.status() {
                Some(Status::Pass) => return MooneyeResult::Pass { steps },
                Some(Status::Fail) => {
                    return MooneyeResult::Fail {
                        steps,
                        registers,
                        pc: core.cpu_registers.pc,
                    };
                }
                _ => {} // Ordinary LD B,B instructions are not result reports.
            }
        }
        if core.cpu_mode == CpuMode::Stopped {
            return MooneyeResult::Stopped {
                steps,
                pc: core.cpu_registers.pc,
            };
        }

        execute_next_test_step(core, &mut frames);
    }

    MooneyeResult::Timeout { steps: max_steps }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RegisterSignature {
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
}

impl RegisterSignature {
    fn status(self) -> Option<Status> {
        match [self.b, self.c, self.d, self.e, self.h, self.l] {
            [3, 5, 8, 13, 21, 34] => Some(Status::Pass),
            [0x42, 0x42, 0x42, 0x42, 0x42, 0x42] => Some(Status::Fail),
            _ => None,
        }
    }

    fn from_core(core: &GameBoyCore) -> Self {
        Self {
            b: core.cpu_registers.b,
            c: core.cpu_registers.c,
            d: core.cpu_registers.d,
            e: core.cpu_registers.e,
            h: core.cpu_registers.h,
            l: core.cpu_registers.l,
        }
    }
}

#[derive(Debug)]
pub(super) enum MooneyeResult {
    InvalidOpcode {
        steps: usize,
        pc: u32,
    },
    Pass {
        steps: usize,
    },
    Fail {
        steps: usize,
        registers: RegisterSignature,
        pc: u32,
    },
    Timeout {
        steps: usize,
    },
    Stopped {
        steps: usize,
        pc: u32,
    },
}

impl MooneyeResult {
    pub(super) fn status(&self) -> Status {
        match self {
            Self::Pass { .. } => Status::Pass,
            Self::Fail { .. } => Status::Fail,
            Self::Timeout { .. } => Status::Timeout,
            Self::Stopped { .. } => Status::Stopped,
            Self::InvalidOpcode { .. } => Status::InvalidOpcode,
        }
    }

    pub(super) fn description(&self) -> String {
        match self {
            Self::InvalidOpcode { steps, pc } => {
                format!("invalid opcode at PC {pc:#06x} after {steps} steps")
            }
            Self::Pass { steps } => format!("pass after {steps} steps"),
            Self::Fail {
                steps,
                registers,
                pc,
            } => format!("fail at PC {pc:#06x} after {steps} steps; B,C,D,E,H,L = {registers:?}"),
            Self::Timeout { steps } => {
                format!("timeout after {steps} steps")
            }
            Self::Stopped { steps, pc } => {
                format!("stopped at PC {pc:#06x} after {steps} steps")
            }
        }
    }
}
