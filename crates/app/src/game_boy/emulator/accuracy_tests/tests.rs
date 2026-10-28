use super::baseline::{self, Status};
use super::models::{self, Model, Targets};
use super::runner;

fn target_names(path: &str) -> Vec<&'static str> {
    match models::targets(path).unwrap() {
        Targets::Run(models) => models.iter().map(|model| model.name()).collect(),
        Targets::Skip(_) => Vec::new(),
    }
}

#[test]
fn mooneye_group_suffixes_select_hardware_not_cartridge_capabilities() {
    assert_eq!(target_names("acceptance/test-G.gb"), ["DMG"]);
    assert_eq!(target_names("acceptance/test-GS.gb"), ["DMG", "SGB"]);
    assert_eq!(target_names("acceptance/test-S.gb"), ["SGB"]);
    assert_eq!(target_names("acceptance/test-C.gb"), ["CGB"]);
    assert_eq!(
        target_names("acceptance/test-GSC.gb"),
        ["DMG", "SGB", "CGB"]
    );
    assert_eq!(target_names("acceptance/test-GA.gb"), ["DMG"]);
    assert_eq!(target_names("acceptance/test.gb"), ["DMG", "SGB"]);
    assert_eq!(target_names("acceptance/boot_regs-dmgABC.gb"), ["DMG"]);
    assert_eq!(target_names("acceptance/boot_regs-sgb.gb"), ["SGB"]);
}

#[test]
fn mooneye_unsupported_models_are_skipped_and_unknown_suffixes_rejected() {
    for suffix in ["dmg0", "mgb", "sgb2", "A", "agb", "ags"] {
        assert!(matches!(
            models::targets(&format!("test-{suffix}.gb")),
            Ok(Targets::Skip(_))
        ));
    }
    assert!(models::targets("test-unrecognised.gb").is_err());
}

#[test]
fn mooneye_selected_hardware_controls_boot_state() {
    let mut bytes = vec![0; 32768];
    bytes[0x0143] = 0x80; // Even CGB cartridge metadata must not select hardware.
    for (model, a, c, renderer) in [
        (Model::Dmg, 1, 0x13, super::super::gpu::LineRenderer::Gb),
        (Model::Sgb, 1, 0x14, super::super::gpu::LineRenderer::Sgb),
        (Model::Cgb, 0x11, 0, super::super::gpu::LineRenderer::Cgb),
    ] {
        let core = models::load_rom(&bytes, "synthetic", model);
        assert_eq!(core.cpu_registers.a, a);
        assert_eq!(core.cpu_registers.c, c);
        assert_eq!(core.line_renderer, renderer);
    }
}

#[test]
fn mooneye_acceptance_loader_preserves_declared_mapper_and_ram() {
    let mut bytes = vec![0; 32768];
    bytes[0x0147] = 0x1b;
    bytes[0x0149] = 2;
    let core = models::load_rom(&bytes, "sources-GS", Model::Dmg);
    assert_eq!(
        core.rom.properties.mbc,
        super::super::rom::MemoryBankController::Mbc5
    );
    assert_eq!(core.sram.persistence_len(), Some(8192));
}

fn program(bytes: &[u8]) -> super::GameBoyCore {
    let mut rom = vec![0; 32768];
    rom[0x0100..0x0100 + bytes.len()].copy_from_slice(bytes);
    models::load_rom(&rom, "synthetic", Model::Dmg)
}

#[test]
fn mooneye_result_protocol_ignores_ordinary_ld_b_b_instructions() {
    let mut core = program(&[0x40, 0xc3, 0x00, 0x01]);
    assert_eq!(runner::run(&mut core, 10).status(), Status::Timeout);
    let mut core = program(&[0x40]);
    core.cpu_registers.b = 3;
    core.cpu_registers.c = 5;
    core.cpu_registers.d = 8;
    core.cpu_registers.e = 13;
    core.cpu_registers.h = 21;
    core.cpu_registers.l = 34;
    assert_eq!(runner::run(&mut core, 10).status(), Status::Pass);
    let mut core = program(&[0x40]);
    core.cpu_registers.b = 0x42;
    core.cpu_registers.c = 0x42;
    core.cpu_registers.d = 0x42;
    core.cpu_registers.e = 0x42;
    core.cpu_registers.h = 0x42;
    core.cpu_registers.l = 0x42;
    assert_eq!(runner::run(&mut core, 10).status(), Status::Fail);
}

#[test]
fn mooneye_runner_distinguishes_stopped_and_invalid_opcode() {
    let mut core = program(&[0x10, 0]);
    assert_eq!(runner::run(&mut core, 10).status(), Status::Stopped);
    let mut core = program(&[0xd3]);
    assert_eq!(runner::run(&mut core, 10).status(), Status::InvalidOpcode);
}

#[test]
fn mooneye_breakpoint_is_not_reported_while_halted_or_preempted() {
    let mut core = program(&[0x40]);
    core.cpu_registers.b = 3;
    core.cpu_registers.c = 5;
    core.cpu_registers.d = 8;
    core.cpu_registers.e = 13;
    core.cpu_registers.h = 21;
    core.cpu_registers.l = 34;
    core.cpu_mode = super::super::cpu::CpuMode::Halted;
    assert_eq!(runner::run(&mut core, 10).status(), Status::Timeout);

    core.cpu_mode = super::super::cpu::CpuMode::Running;
    core.cpu_registers.ime = true;
    core.memory.io_ports[0xff] = 1;
    core.memory.io_ports[0x0f] = 1;
    core.memory.rom[0x0040] = 0xd3;
    assert_eq!(runner::run(&mut core, 10).status(), Status::InvalidOpcode);
}

#[test]
fn mooneye_baseline_rejects_ambiguous_rows_and_preserves_failure_classes() {
    let rows = "DMG\tfail\tacceptance/a.gb\nSGB\ttimeout\tacceptance/a.gb\n";
    let baseline = baseline::parse(rows).unwrap();
    assert_eq!(baseline["DMG\tacceptance/a.gb"], Status::Fail);
    assert_eq!(baseline["SGB\tacceptance/a.gb"], Status::Timeout);
    assert_ne!(Status::Fail, Status::Timeout);
    assert!(baseline::parse(&(rows.to_owned() + rows)).is_err());
    assert!(baseline::parse("DMG\tknown-failure\tacceptance/a.gb").is_err());
    assert!(baseline::parse("MGB\tpass\tacceptance/a.gb").is_err());
}
