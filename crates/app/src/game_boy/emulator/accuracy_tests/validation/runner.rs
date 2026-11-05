use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::super::baseline::Status;
use crate::game_boy::emulator::{
    GameBoyCore, bus::read8, cpu::CpuMode, systems::execute_next_test_step,
};
use crate::game_boy::frame_buffer::GameBoyFrameRing;
use crate::storage::validation_roms::{Case, Protocol};

pub(super) struct Outcome {
    pub(super) status: Status,
    pub(super) detail: String,
}

pub(super) fn breakpoint(core: &GameBoyCore) -> Option<Status> {
    let pc = u16::try_from(core.cpu_registers.pc).ok()?;
    let interrupt = core.cpu_registers.ime
        && core.memory.io_ports[0xff] & core.memory.io_ports[0x0f] & 0x1f != 0;
    if core.cpu_mode != CpuMode::Running
        || interrupt
        || core.dma.vram.cpu_halt_m_cycles != 0
        || read8(core, pc) != 0x40
    {
        return None;
    }
    let r = &core.cpu_registers;
    match [r.b, r.c, r.d, r.e, r.h, r.l] {
        [3, 5, 8, 13, 21, 34] => Some(Status::Pass),
        [0x42, 0x42, 0x42, 0x42, 0x42, 0x42] => Some(Status::Fail),
        _ => None,
    }
}

#[derive(Default)]
pub(super) struct BlarggReporter {
    running_seen: bool,
    serial_byte: Option<u8>,
    pub(super) serial: String,
}

impl BlarggReporter {
    pub(super) fn poll(&mut self, core: &GameBoyCore) -> Option<Status> {
        let signature = [
            read8(core, 0xa001),
            read8(core, 0xa002),
            read8(core, 0xa003),
        ] == [0xde, 0xb0, 0x61];
        let status = read8(core, 0xa000);
        self.running_seen |= signature && status == 0x80;
        if signature && self.running_seen && status != 0x80 {
            return Some(if status == 0 {
                Status::Pass
            } else {
                Status::Fail
            });
        }
        // Observe completed transfers without changing SC, SB or their timing.
        if core.serial.is_transferring {
            if self.serial_byte.is_none() {
                self.serial_byte = Some(core.memory.io_ports[1]);
            }
        } else if let Some(byte) = self.serial_byte.take()
            && self.serial.len() < 8192
        {
            self.serial.push(char::from(byte));
        }
        if self
            .serial
            .split_inclusive('\n')
            .any(|line| line.ends_with('\n') && line.trim() == "Passed")
        {
            Some(Status::Pass)
        } else if self.serial.contains("Failed") {
            Some(Status::Fail)
        } else {
            None
        }
    }

    fn detail(&self, core: &GameBoyCore) -> String {
        let text: Vec<_> = (0xa004..0xc000)
            .map(|address| read8(core, address))
            .take_while(|byte| *byte != 0)
            .collect();
        if self.running_seen {
            format!(
                "code {:02x}; {}",
                read8(core, 0xa000),
                String::from_utf8_lossy(&text).trim()
            )
        } else {
            self.serial.trim().to_owned()
        }
    }
}

pub(super) fn run(core: &mut GameBoyCore, case: &Case, root: &Path) -> Outcome {
    let mut frames = GameBoyFrameRing::default();
    let mut reporter = BlarggReporter::default();
    let limit = case.seconds * 4_194_304;
    let mut steps = 0_u64;
    let status = loop {
        if !core.runtime.is_running {
            break Status::InvalidOpcode;
        }
        let status = match case.protocol {
            Protocol::Breakpoint => breakpoint(core),
            Protocol::Blargg => reporter.poll(core),
            Protocol::Screen => None,
        };
        if let Some(status) = status {
            break status;
        }
        if core.cpu_mode == CpuMode::Stopped
            && !(core.rom.properties.cgb_flag && core.memory.io_ports[0x4d] & 1 != 0)
        {
            break Status::Stopped;
        }
        if core.audio_unit.cumulative_ticks >= limit || steps >= 100_000_000 {
            break if case.protocol == Protocol::Screen {
                screen_status(case, root, &frames)
            } else {
                Status::Timeout
            };
        }
        execute_next_test_step(core, &mut frames);
        steps += 1;
        if steps.is_multiple_of(4096) {
            core.audio_unit.drain_pending_events();
        }
    };
    let detail = match case.protocol {
        Protocol::Blargg => reporter.detail(core),
        Protocol::Breakpoint if status == Status::Fail => {
            // SameSuite retains actual samples in WRAM; compare via the original
            // ROM's expected table when investigating this dump.
            format!("WRAM C000..C020: {:02x?}", &core.memory.wram[..32])
        }
        Protocol::Screen => "golden screen structure (palette independent)".to_owned(),
        Protocol::Breakpoint => String::new(),
    };
    Outcome {
        status,
        detail: format!(
            "PC {:04x}, {steps} steps, {} ticks; {detail}",
            core.cpu_registers.pc, core.audio_unit.cumulative_ticks
        ),
    }
}

// Compare the exact pattern of pixel colors, allowing a consistent palette
// remapping. This checks the reported result, not the display's color fidelity.
pub(super) fn color_pattern(bytes: &[u8], channels: usize) -> Vec<usize> {
    let mut colors = HashMap::new();
    bytes
        .chunks_exact(channels)
        .map(|pixel| {
            let next = colors.len();
            *colors.entry(pixel[..3].to_vec()).or_insert(next)
        })
        .collect()
}

fn screen_status(case: &Case, root: &Path, frames: &GameBoyFrameRing) -> Status {
    let data = fs::read(root.join(case.screen.as_ref().expect("screen fixture")))
        .expect("read golden screen");
    assert_eq!(
        Some(hex::encode(Sha256::digest(&data))).as_ref(),
        case.screen_sha256.as_ref(),
        "Golden screen checksum mismatch"
    );
    let expected = image::load_from_memory(&data)
        .expect("valid PNG fixture")
        .to_rgb8();
    let Some(frame) = frames.latest_written_frame() else {
        return Status::Timeout;
    };
    if color_pattern(frame.pixels(), 3) == color_pattern(expected.as_raw(), 3) {
        Status::Pass
    } else {
        Status::Fail
    }
}
