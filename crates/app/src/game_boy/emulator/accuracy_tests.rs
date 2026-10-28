//! Headless regression checks for the pinned Mooneye acceptance ROMs.
//! No emulator behavior is patched to make tests pass. See the adjacent README.
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;

use super::GameBoyCore;
use super::rom::{MemoryBankController, RomProperties};
use baseline::Status;
use models::Targets;

mod baseline;
mod fixtures;
mod models;
mod runner;
mod tests;

const EXPECTED_ACCEPTANCE_ROM_COUNT: usize = 75;
const MAX_STEPS: usize = 2_000_000;

#[test]
#[ignore = "requires downloaded Mooneye test ROMs; run scripts/mooneye-tests.sh"]
fn mooneye_acceptance_suite_tracks_known_statuses() {
    let roms = fixtures::acceptance_roms();
    assert_eq!(
        roms.len(),
        EXPECTED_ACCEPTANCE_ROM_COUNT,
        "Mooneye fixture inventory changed; refresh the pinned fixtures or review the suite version"
    );
    let mut expected =
        baseline::parse(&fs::read_to_string(baseline::path()).expect("read Mooneye baseline"))
            .expect("valid Mooneye baseline");
    let mut changes = Vec::new();
    let mut report =
        String::from("# Mooneye mts-20240926-1737-443f6e1; model<TAB>status<TAB>ROM\n");
    let mut counts = BTreeMap::new();
    let mut skipped = 0;
    let mut executed = 0;

    for rom in &roms {
        let models = match models::targets(rom).expect("explicit Mooneye hardware metadata") {
            Targets::Run(models) => models,
            Targets::Skip(reason) => {
                println!("SKIP {rom}: {reason}");
                writeln!(report, "# SKIP {rom}: {reason}").expect("write report text");
                skipped += 1;
                continue;
            }
        };
        let bytes = fixtures::read_fixture(rom);
        for &model in models {
            let mut core = models::load_rom(&bytes, rom, model);
            let result = runner::run(&mut core, MAX_STEPS);
            let actual = result.status();
            let key = format!("{}\t{rom}", model.name());
            println!(
                "{} {} {rom}: {}",
                model.name(),
                actual.name(),
                result.description()
            );
            writeln!(report, "{}\t{}\t{rom}", model.name(), actual.name())
                .expect("write report text");
            *counts.entry(actual.name()).or_insert(0_usize) += 1;
            executed += 1;
            let previous = expected.remove(&key);
            if previous != Some(actual) {
                changes.push(format!(
                    "{key}: expected {previous:?}, got {}",
                    result.description()
                ));
            }
        }
    }
    for missing in expected.keys() {
        changes.push(format!("baseline case was not run: {missing}"));
    }
    println!(
        "Mooneye: {} ROMs, {executed} model cases; {counts:?}; {skipped} unsupported ROMs skipped",
        roms.len()
    );
    // Writes an inspection artifact only; the checked-in baseline is never
    // updated automatically, including when this run detects a regression.
    if let Some(path) = std::env::var_os("SHINING_MOONEYE_REPORT") {
        fs::write(path, report).expect("write Mooneye result report");
    }
    assert!(
        changes.is_empty(),
        "Mooneye baseline changed:\n{}",
        changes.join("\n")
    );
}
