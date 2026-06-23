use std::fs;
use std::path::{Path, PathBuf};

use super::GameBoyCore;
use super::bus::{begin_deferred_oam_dma, read8, step_oam_dma, step_system_counter};
use super::cpu::CpuMode;
use super::execution::perform_op;
use super::rom::{MemoryBankController, RomProperties};

const MOONEYE_ROM_ROOT_ENV: &str = "SHINING_MOONEYE_ROM_ROOT";
const EXPECTED_ACCEPTANCE_ROM_COUNT: usize = 75;
const MAX_INSTRUCTIONS: usize = 2_000_000;
const MACHINE_CYCLE_CLOCKS: u16 = 4;
const LD_B_B_OPCODE: u8 = 0x40;
const EXPECTED_PASS_ROMS: &[&str] = &[
    "acceptance/boot_regs-dmgABC.gb",
    "acceptance/bits/reg_f.gb",
    "acceptance/bits/mem_oam.gb",
    "acceptance/div_timing.gb",
    "acceptance/instr/daa.gb",
    "acceptance/oam_dma/basic.gb",
    "acceptance/timer/div_write.gb",
    "acceptance/timer/tim00.gb",
    "acceptance/timer/tim00_div_trigger.gb",
    "acceptance/timer/tim01.gb",
    "acceptance/timer/tim01_div_trigger.gb",
    "acceptance/timer/tim10.gb",
    "acceptance/timer/tim10_div_trigger.gb",
    "acceptance/timer/tim11.gb",
    "acceptance/timer/tim11_div_trigger.gb",
    "acceptance/timer/tima_reload.gb",
];
const MOONEYE_PASS_REGISTERS: RegisterSignature = RegisterSignature {
    b: 3,
    c: 5,
    d: 8,
    e: 13,
    h: 21,
    l: 34,
};

#[test]
#[ignore = "requires downloaded Mooneye test ROMs; run scripts/mooneye-tests.sh"]
fn mooneye_acceptance_suite_tracks_known_statuses() {
    let roms = mooneye_acceptance_roms();
    assert_eq!(
        roms.len(),
        EXPECTED_ACCEPTANCE_ROM_COUNT,
        "expected {EXPECTED_ACCEPTANCE_ROM_COUNT} Mooneye acceptance ROMs in the fixture cache; \
         found {}. Run scripts/mooneye-tests.sh to refresh the cache.",
        roms.len()
    );

    let mut failures = Vec::new();
    let mut expected_passes = 0;
    let mut known_failures = 0;

    for rom in roms {
        let expected = expected_status(&rom);
        let result = run_mooneye_rom(&rom);

        match (expected, &result) {
            (ExpectedStatus::Pass, MooneyeResult::Pass { .. }) => {
                expected_passes += 1;
            }
            (ExpectedStatus::Pass, _) => failures.push(format!(
                "{} was expected to pass but returned {}",
                rom,
                result.description()
            )),
            (ExpectedStatus::KnownFailure, MooneyeResult::Pass { .. }) => failures.push(format!(
                "{rom} unexpectedly passed; promote it to EXPECTED_PASS_ROMS"
            )),
            (ExpectedStatus::KnownFailure, _) => {
                known_failures += 1;
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Mooneye acceptance status changed:\n{}",
        failures.join("\n")
    );
    assert_eq!(
        expected_passes,
        EXPECTED_PASS_ROMS.len(),
        "not all expected-passing Mooneye ROMs were present"
    );
    assert!(
        known_failures > 0,
        "Mooneye suite no longer has known failures; promote newly passing ROMs"
    );
}

fn run_mooneye_rom(relative_path: &str) -> MooneyeResult {
    let rom = read_mooneye_fixture(relative_path);
    let mut core = load_mooneye_test_rom(&rom, relative_path);
    run_until_mooneye_result(&mut core)
}

fn expected_status(relative_path: &str) -> ExpectedStatus {
    if EXPECTED_PASS_ROMS.contains(&relative_path) {
        ExpectedStatus::Pass
    } else {
        ExpectedStatus::KnownFailure
    }
}

fn mooneye_acceptance_roms() -> Vec<String> {
    let root = mooneye_rom_root();
    let acceptance_root = root.join("acceptance");
    let mut roms = Vec::new();
    collect_mooneye_roms(&root, &acceptance_root, &mut roms);
    roms.sort();
    roms
}

fn collect_mooneye_roms(root: &Path, directory: &Path, roms: &mut Vec<String>) {
    let entries = fs::read_dir(directory).unwrap_or_else(|error| {
        panic!(
            "failed to read Mooneye acceptance directory at {}: {error}\n\
             Run scripts/mooneye-tests.sh to download fixtures and execute these tests.",
            directory.display()
        )
    });

    for entry in entries {
        let entry = entry.expect("Mooneye directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_mooneye_roms(root, &path, roms);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("gb") {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("Mooneye ROM path should be below ROM root")
            .to_string_lossy()
            .replace('\\', "/");
        roms.push(relative);
    }
}

fn read_mooneye_fixture(relative_path: &str) -> Vec<u8> {
    let path = mooneye_rom_root().join(relative_path);
    fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read Mooneye test ROM fixture at {}: {error}\n\
             Run scripts/mooneye-tests.sh to download fixtures and execute these tests.",
            path.display()
        )
    })
}

fn mooneye_rom_root() -> PathBuf {
    if let Some(path) = std::env::var_os(MOONEYE_ROM_ROOT_ENV) {
        return PathBuf::from(path);
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".test-roms/mooneye")
}

fn load_mooneye_test_rom(bytes: &[u8], name: &str) -> GameBoyCore {
    let mut core = GameBoyCore::default();
    let properties = RomProperties {
        valid: true,
        mbc: MemoryBankController::None,
        size_bytes: bytes.len() as i32,
        check_sum: u32::from(bytes[0x014d]),
        ..Default::default()
    };

    assert!(
        core.reset_for_rom_load(properties, name.to_string(), format!("{name}.gb"), bytes,),
        "{name} could not be loaded into Game Boy memory"
    );
    core
}

fn run_until_mooneye_result(core: &mut GameBoyCore) -> MooneyeResult {
    for instructions in 0..MAX_INSTRUCTIONS {
        if read8(core, core.cpu_registers.pc as u16) == LD_B_B_OPCODE {
            let registers = RegisterSignature::from_core(core);
            return if registers == MOONEYE_PASS_REGISTERS {
                MooneyeResult::Pass { instructions }
            } else {
                MooneyeResult::Fail {
                    instructions,
                    registers,
                    pc: core.cpu_registers.pc,
                }
            };
        }
        if core.cpu_mode != CpuMode::Running {
            return MooneyeResult::Stopped {
                instructions,
                pc: core.cpu_registers.pc,
            };
        }

        let clocks = perform_op(core);
        core.cpu_registers.pc &= 0xffff;
        for _ in 0..machine_cycles_for_clocks(clocks) {
            step_system_counter(core, MACHINE_CYCLE_CLOCKS);
            step_oam_dma(core);
            core.audio_unit
                .advance_ticks(i32::from(MACHINE_CYCLE_CLOCKS));
        }
        begin_deferred_oam_dma(core);
    }

    MooneyeResult::Timeout {
        instructions: MAX_INSTRUCTIONS,
    }
}

fn machine_cycles_for_clocks(clocks: i32) -> usize {
    let clocks = clocks.max(i32::from(MACHINE_CYCLE_CLOCKS));
    clocks
        .saturating_add(i32::from(MACHINE_CYCLE_CLOCKS) - 1)
        .checked_div(i32::from(MACHINE_CYCLE_CLOCKS))
        .and_then(|cycles| usize::try_from(cycles).ok())
        .unwrap_or(1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RegisterSignature {
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
}

impl RegisterSignature {
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
enum MooneyeResult {
    Pass {
        instructions: usize,
    },
    Fail {
        instructions: usize,
        registers: RegisterSignature,
        pc: u32,
    },
    Timeout {
        instructions: usize,
    },
    Stopped {
        instructions: usize,
        pc: u32,
    },
}

impl MooneyeResult {
    fn description(&self) -> String {
        match self {
            Self::Pass { instructions } => format!("pass after {instructions} instructions"),
            Self::Fail {
                instructions,
                registers,
                pc,
            } => format!(
                "fail at PC {pc:#06x} after {instructions} instructions; B,C,D,E,H,L = {registers:?}"
            ),
            Self::Timeout { instructions } => {
                format!("timeout after {instructions} instructions")
            }
            Self::Stopped { instructions, pc } => {
                format!("stopped at PC {pc:#06x} after {instructions} instructions")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedStatus {
    Pass,
    KnownFailure,
}
