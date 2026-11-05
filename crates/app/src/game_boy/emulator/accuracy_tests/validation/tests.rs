use super::super::{baseline::Status, models::Model};
use super::*;
use crate::game_boy::emulator::{bus::write8, cpu::CpuMode};

fn core() -> crate::game_boy::emulator::GameBoyCore {
    let mut bytes = vec![0; 32768];
    bytes[0x147] = 3;
    bytes[0x149] = 2;
    let mut core = models::load_rom(&bytes, "protocol-test", Model::Dmg);
    write8(&mut core, 0, 0x0a);
    core
}

#[test]
fn validation_blargg_report_requires_signature_and_running_state() {
    let mut core = core();
    let mut reporter = runner::BlarggReporter::default();
    assert!(reporter.poll(&core).is_none());
    for (address, byte) in [(0xa001, 0xde), (0xa002, 0xb0), (0xa003, 0x61)] {
        write8(&mut core, address, byte);
    }
    write8(&mut core, 0xa000, 0);
    assert!(reporter.poll(&core).is_none(), "initial zero must not pass");
    write8(&mut core, 0xa000, 0x80);
    assert!(reporter.poll(&core).is_none());
    write8(&mut core, 0xa000, 0);
    assert_eq!(reporter.poll(&core), Some(Status::Pass));
    write8(&mut core, 0xa000, 4);
    assert_eq!(reporter.poll(&core), Some(Status::Fail));
}

#[test]
fn validation_serial_reporting_observes_completed_transfers_without_mutation() {
    let mut core = core();
    let mut reporter = runner::BlarggReporter::default();
    for byte in b"\nPassed\n" {
        core.memory.io_ports[1] = *byte;
        core.serial.is_transferring = true;
        assert!(reporter.poll(&core).is_none());
        assert_eq!(core.memory.io_ports[1], *byte);
        core.serial.is_transferring = false;
        reporter.poll(&core);
    }
    assert_eq!(reporter.poll(&core), Some(Status::Pass));
}

#[test]
fn validation_breakpoint_is_ignored_while_halted_or_preempted() {
    let mut core = core();
    core.memory.rom[0x100] = 0x40;
    core.cpu_registers.b = 3;
    core.cpu_registers.c = 5;
    core.cpu_registers.d = 8;
    core.cpu_registers.e = 13;
    core.cpu_registers.h = 21;
    core.cpu_registers.l = 34;
    assert_eq!(runner::breakpoint(&core), Some(Status::Pass));
    core.cpu_mode = CpuMode::Halted;
    assert_eq!(runner::breakpoint(&core), None);
    core.cpu_mode = CpuMode::Running;
    core.cpu_registers.ime = true;
    core.memory.io_ports[0xff] = 1;
    core.memory.io_ports[0x0f] = 1;
    assert_eq!(runner::breakpoint(&core), None);
}

#[test]
fn validation_baseline_rejects_duplicate_rows_and_preserves_outcome_classes() {
    let row = "DMG\tfail\tblargg/test.gb\n";
    assert!(parse_baseline(&(row.to_owned() + row)).is_err());
    assert!(parse_baseline("DMG\tknown-failure\ttest.gb").is_err());
    assert_ne!(
        parse_baseline(row).unwrap(),
        parse_baseline(&row.replace("fail", "timeout")).unwrap()
    );
}

#[test]
fn validation_screen_comparison_preserves_pixel_structure() {
    let a = [0, 0, 0, 255, 255, 255, 0, 0, 0];
    let b = [10, 20, 30, 90, 80, 70, 10, 20, 30];
    assert_eq!(runner::color_pattern(&a, 3), runner::color_pattern(&b, 3));
    assert_ne!(
        runner::color_pattern(&a, 3),
        runner::color_pattern(&[0; 9], 3)
    );
}

#[test]
fn validation_runner_allows_a_prepared_cgb_speed_switch_stop() {
    let mut bytes = vec![0; 32768];
    bytes[0x100..0x104].copy_from_slice(&[0x10, 0, 0x18, 0xfe]);
    let mut core = models::load_rom(&bytes, "speed-switch", Model::Cgb);
    core.memory.io_ports[0x4d] = 1;
    let mut case = catalog().unwrap().cases.remove(0);
    case.seconds = 1;
    let result = runner::run(&mut core, &case, &root());
    assert_eq!(result.status, Status::Timeout);
    assert_eq!(core.cpu_mode, CpuMode::Running);
    assert_eq!(core.memory.io_ports[0x4d] & 0x81, 0x80);
}
