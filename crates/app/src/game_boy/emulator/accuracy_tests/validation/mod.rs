//! All three suites share the production CPU/peripheral execution path.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use super::models::{self, Model};
use crate::storage::validation_roms::{Case, catalog};

mod runner;
#[cfg(test)]
mod tests;

fn root() -> PathBuf {
    std::env::var_os("SHINING_TEST_ROM_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.test-roms/validation"),
        PathBuf::from,
    )
}

fn baseline() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-roms/baselines/validation.tsv")
}

fn parse_baseline(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut rows = BTreeMap::new();
    for line in text
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3
            || !matches!(
                fields[1],
                "pass" | "fail" | "timeout" | "stopped" | "invalid-opcode" | "skip"
            )
        {
            return Err(format!("Invalid validation baseline row: {line}"));
        }
        let key = format!("{}\t{}", fields[0], fields[2]);
        if rows.insert(key.clone(), fields[1].to_owned()).is_some() {
            return Err(format!("Duplicate baseline case: {key}"));
        }
    }
    Ok(rows)
}

fn selected(case: &Case, suite: &str, filter: &str) -> bool {
    case.path.starts_with(suite) && (filter.is_empty() || case.file_name().contains(filter))
}

fn run_suite(suite: &str) {
    let catalog = catalog().expect("valid pinned test catalog");
    let filter = std::env::var("SHINING_TEST_ROM_FILTER").unwrap_or_default();
    let cases: Vec<_> = catalog
        .cases
        .iter()
        .filter(|case| selected(case, suite, &filter))
        .collect();
    assert!(
        !cases.is_empty(),
        "No cases match suite {suite} and filter {filter:?}"
    );
    let expected =
        parse_baseline(&fs::read_to_string(baseline()).expect("read validation baseline"))
            .expect("valid validation baseline");
    let keys: BTreeSet<_> = cases.iter().map(|case| case.file_name()).collect();
    let mut changes = Vec::new();
    let mut counts = BTreeMap::new();
    let mut report = format!("# {}; model<TAB>status<TAB>ROM\n", catalog.collection);
    for case in cases {
        let (status, detail) = if let Some(reason) = &case.skip {
            ("skip".to_owned(), reason.clone())
        } else {
            let path = root().join(&case.path);
            let bytes = fs::read(&path).unwrap_or_else(|error| {
                panic!(
                    "Cannot read {}: {error}; run python3 scripts/test-roms.py prepare",
                    path.display()
                )
            });
            assert!(
                case.verify(&bytes),
                "Fixture checksum mismatch: {}",
                case.path
            );
            let model = match case.model.as_str() {
                "DMG" => Model::Dmg,
                "SGB" => Model::Sgb,
                "CGB-E" => Model::Cgb,
                other => panic!("Unsupported hardware without a skip reason: {other}"),
            };
            let mut core = models::load_rom(&bytes, &case.path, model);
            let result = runner::run(&mut core, case, &root());
            (result.status.name().to_owned(), result.detail)
        };
        println!("{} {status} {}: {detail}", case.model, case.path);
        writeln!(report, "{}\t{status}\t{}", case.model, case.path).unwrap();
        *counts.entry(status.clone()).or_insert(0_usize) += 1;
        let key = format!("{}\t{}", case.model, case.path);
        if expected.get(&key) != Some(&status) {
            changes.push(format!(
                "{key}: expected {:?}, got {status}",
                expected.get(&key)
            ));
        }
    }
    // A full-suite run also detects stale baseline rows and removed fixtures.
    if filter.is_empty() {
        for key in expected.keys() {
            let (model, path) = key.split_once('\t').expect("parsed baseline key");
            if path.starts_with(suite) && !keys.contains(&format!("{model}/{path}")) {
                changes.push(format!("Baseline case was not run: {key}"));
            }
        }
    }
    println!("{suite}: {counts:?}");
    if let Some(directory) = std::env::var_os("SHINING_TEST_ROM_REPORT_DIR") {
        fs::create_dir_all(&directory).expect("create report directory");
        fs::write(
            PathBuf::from(directory).join(format!("{suite}.tsv")),
            report,
        )
        .expect("write candidate report");
    }
    assert!(
        changes.is_empty(),
        "Validation baseline changed:\n{}",
        changes.join("\n")
    );
}

#[test]
#[ignore = "requires prepared fixtures; run python3 scripts/test-roms.py run"]
fn validation_blargg_regressions() {
    run_suite("blargg");
}

#[test]
#[ignore = "requires prepared fixtures; run python3 scripts/test-roms.py run"]
fn validation_samesuite_regressions() {
    run_suite("same-suite");
}

#[test]
#[ignore = "requires prepared fixtures; run python3 scripts/test-roms.py run"]
fn validation_mooneye_regressions() {
    run_suite("mooneye-test-suite");
}
