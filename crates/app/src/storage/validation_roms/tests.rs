use std::cell::Cell;
use std::collections::HashSet;
use std::fs;

use super::*;

const DIRECTORY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-roms/site-directory.json"
));
const APP_CATALOG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-roms/site-in-app.json"
));
const NOTICE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-roms/licenses/Mooneye-MIT.txt"
));

fn fixture(url: &str) -> Result<Vec<u8>, StorageError> {
    let base = site::config()?.base_url;
    let path = url
        .strip_prefix(&base)
        .ok_or_else(|| site::error("wrong host"))?;
    if path == "directory.json" {
        Ok(DIRECTORY.to_vec())
    } else if path.ends_with("/in-app.json") {
        Ok(APP_CATALOG.to_vec())
    } else if path.ends_with("/licenses/Mooneye-MIT.txt") {
        Ok(NOTICE.to_vec())
    } else if path.ends_with("/roms/mooneye-test-suite/acceptance/div_timing.gb") {
        let root = std::env::var_os("SHINING_TEST_ROM_ROOT").map_or_else(
            || {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../.test-roms/validation")
            },
            std::path::PathBuf::from,
        );
        fs::read(root.join("mooneye-test-suite/acceptance/div_timing.gb")).map_err(Into::into)
    } else {
        Err(site::error("unexpected request"))
    }
}

#[test]
fn validation_catalog_selects_only_permitted_cases_with_unique_hardware_keys() {
    let catalog = catalog().unwrap();
    let mut keys = HashSet::new();
    for case in &catalog.cases {
        assert!(keys.insert(case.file_name()));
        assert!(case.seconds > 0 && case.seconds <= 120);
        assert!(!case.path.contains(".."));
        if case.in_app {
            assert!(!case.path.starts_with("blargg/"));
            assert!(case.skip.is_none());
            assert!(super::case(&case.file_name()).is_ok());
        }
    }
    assert_eq!(catalog.cases.len(), 330);
    assert_eq!(metadata().unwrap().len(), 34);
    assert!(case("blargg/dmg_sound.gb").is_err());
}

#[test]
fn validation_web_catalog_checks_the_directory_and_pinned_selection() {
    let calls = Cell::new(0);
    let roms = site::roms_with(|url| {
        calls.set(calls.get() + 1);
        fixture(url)
    })
    .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(roms.len(), 34);
    assert!(roms.iter().all(|rom| !rom.file_name.contains("blargg")));
    assert!(site::roms_with(|_| Err(site::error("HTTP failure"))).is_err());
}

#[test]
fn validation_web_catalog_rejects_schema_release_and_content_changes() {
    for (from, to) in [
        ("\"schema_version\": 1", "\"schema_version\": 9"),
        ("validation-v7.0-1", "unexpected-release"),
    ] {
        let changed = String::from_utf8(DIRECTORY.to_vec())
            .unwrap()
            .replace(from, to);
        assert!(site::roms_with(|_| Ok(changed.as_bytes().to_vec())).is_err());
    }
    assert!(
        site::roms_with(|url| {
            if url.ends_with("in-app.json") {
                Ok(b"{}".to_vec())
            } else {
                fixture(url)
            }
        })
        .is_err()
    );
}

#[test]
#[ignore = "requires prepared fixtures; run python3 scripts/test-roms.py run"]
fn validation_download_verifies_rom_and_notice_then_uses_cache_offline() {
    let root = std::env::temp_dir().join(format!("validation-web-{}", uuid::Uuid::new_v4()));
    let name = "DMG/mooneye-test-suite/acceptance/div_timing.gb";
    let calls = Cell::new(0);
    let (case, bytes, origin) = download::load_with(name, None, &root, |url| {
        calls.set(calls.get() + 1);
        fixture(url)
    })
    .unwrap();
    assert_eq!(calls.get(), 2);
    assert!(case.verify(&bytes));
    assert!(origin.starts_with(&site::config().unwrap().base_url));
    let id = crate::storage::rom_identifier::rom_identifier(&bytes);
    let (_, cached, _) = download::load_with(name, Some(&id), &root, |_| {
        panic!("valid cached ROM and license must not require network access")
    })
    .unwrap();
    assert_eq!(cached, bytes);
    fs::write(root.join(&id).join(name), b"corrupt cache").unwrap();
    let (_, repaired, _) = download::load_with(name, Some(&id), &root, fixture).unwrap();
    assert_eq!(repaired, bytes);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn validation_download_rejects_bad_roms_licenses_and_non_app_entries() {
    let root = std::env::temp_dir().join(format!("validation-bad-web-{}", uuid::Uuid::new_v4()));
    let name = "DMG/mooneye-test-suite/acceptance/div_timing.gb";
    assert!(download::load_with(name, None, &root, |_| Ok(b"bad license".to_vec())).is_err());
    assert!(!root.exists());
    assert!(
        download::load_with(name, None, &root, |url| {
            if url.contains("/roms/") {
                Ok(b"bad ROM".to_vec())
            } else {
                fixture(url)
            }
        })
        .is_err()
    );
    assert!(
        download::load_with("DMG/blargg/test.gb", None, &root, |_| {
            panic!("non-app entries must be rejected before fetching")
        })
        .is_err()
    );
    fs::remove_dir_all(root).unwrap();
}
