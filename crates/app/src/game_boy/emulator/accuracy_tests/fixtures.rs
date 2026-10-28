use std::fs;
use std::path::{Path, PathBuf};

const MOONEYE_ROM_ROOT_ENV: &str = "SHINING_MOONEYE_ROM_ROOT";

pub(super) fn acceptance_roms() -> Vec<String> {
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

pub(super) fn read_fixture(relative_path: &str) -> Vec<u8> {
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
