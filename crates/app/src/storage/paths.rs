use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(not(target_os = "android"))]
use directories::ProjectDirs;

use crate::storage::errors::StorageError;
use crate::storage::rom_identifier::rom_identifier;

const APP_STORAGE_DIR_NAME: &str = "shining-emulator";
const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "Shining Grimace";
const APPLICATION: &str = "Shining Emulator";
const SETTINGS_FILE_NAME: &str = "settings.json";
const PROVIDERS_FILE_NAME: &str = "providers.json";
const ROMS_FILE_NAME: &str = "roms.json";
const TIMESTAMPS_FILE_NAME: &str = "timestamps.json";
const INPUT_FILE_NAME: &str = "input.json";
const ROMS_DIR_NAME: &str = "roms";
const AUDIO_DIR_NAME: &str = "audio";
const DEFAULT_AUDIO_PRESET_FILE_NAME: &str = "preset0.json";

#[derive(Clone, Debug)]
pub struct StoragePaths {
    pub root_dir: PathBuf,
    pub settings_file: PathBuf,
    pub providers_file: PathBuf,
    pub roms_file: PathBuf,
    pub timestamps_file: PathBuf,
    pub input_file: PathBuf,
    pub roms_dir: PathBuf,
    pub audio_dir: PathBuf,
}

impl StoragePaths {
    pub fn new() -> Result<Self, StorageError> {
        let root_dir = app_storage_dir()?;
        Ok(Self::from_root(root_dir))
    }

    pub fn from_root(root_dir: PathBuf) -> Self {
        Self {
            settings_file: root_dir.join(SETTINGS_FILE_NAME),
            providers_file: root_dir.join(PROVIDERS_FILE_NAME),
            roms_file: root_dir.join(ROMS_FILE_NAME),
            timestamps_file: root_dir.join(TIMESTAMPS_FILE_NAME),
            input_file: root_dir.join(INPUT_FILE_NAME),
            roms_dir: root_dir.join(ROMS_DIR_NAME),
            audio_dir: root_dir.join(AUDIO_DIR_NAME),
            root_dir,
        }
    }

    pub fn create_dirs(&self) -> Result<(), StorageError> {
        fs::create_dir_all(&self.root_dir)?;
        fs::create_dir_all(&self.roms_dir)?;
        fs::create_dir_all(&self.audio_dir)?;
        Ok(())
    }

    pub fn default_audio_preset_file(&self) -> PathBuf {
        self.audio_dir.join(DEFAULT_AUDIO_PRESET_FILE_NAME)
    }

    pub fn audio_preset_file(&self, index: u8) -> PathBuf {
        self.audio_dir.join(format!("preset{index}.json"))
    }
}

impl Default for StoragePaths {
    fn default() -> Self {
        let root_dir = env::temp_dir().join(APP_STORAGE_DIR_NAME);
        Self::from_root(root_dir)
    }
}

pub(crate) fn cache_remote_rom(roms_dir: &Path, file_name: &str, bytes: &[u8]) -> bool {
    let rom_id = rom_identifier(bytes);
    let path = roms_dir.join(rom_id).join(file_name);
    let Some(parent) = path.parent() else {
        return false;
    };
    if fs::create_dir_all(parent).is_err() {
        return false;
    }
    fs::write(path, bytes).is_ok()
}

#[cfg(not(target_os = "android"))]
fn app_storage_dir() -> Result<PathBuf, StorageError> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or(StorageError::MissingProjectDirectory)
}

#[cfg(target_os = "android")]
fn app_storage_dir() -> Result<PathBuf, StorageError> {
    bevy::android::ANDROID_APP
        .get()
        .and_then(|app| app.internal_data_path())
        .map(|path| path.join(APP_STORAGE_DIR_NAME))
        .ok_or(StorageError::MissingProjectDirectory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_rom_cache_creates_nested_filename_parent_dirs() {
        let root = env::temp_dir().join(format!(
            "shining-emulator-cache-test-{}",
            std::process::id()
        ));
        let bytes = b"cached ROM bytes";

        assert!(cache_remote_rom(&root, "files/test.gb", bytes));

        let rom_id = rom_identifier(bytes);
        assert!(root.join(rom_id).join("files/test.gb").is_file());

        let _ = fs::remove_dir_all(root);
    }
}
