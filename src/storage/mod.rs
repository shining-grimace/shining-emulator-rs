#![allow(dead_code)]

pub mod data;
pub mod errors;
pub mod input_mappings;
pub mod paths;
pub mod providers;
pub mod rom_identifier;

use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Serialize, de::DeserializeOwned};

use crate::input::mappings::ensure_essential_navigation_mappings;
use crate::storage::data::{GeneralSettings, LastPlayedTimestamps, LocalStorageData, RomMetadata};
use crate::storage::errors::StorageError;
use crate::storage::input_mappings::{InputDeviceType, default_input_mappings};
use crate::storage::paths::StoragePaths;
use crate::storage::providers::default_rom_providers;

#[derive(Resource)]
pub struct LocalStorage {
    pub paths: StoragePaths,
    pub data: LocalStorageData,
}

pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LocalStorage>();
    }
}

impl FromWorld for LocalStorage {
    fn from_world(_world: &mut World) -> Self {
        match LocalStorage::load_or_initialise() {
            Ok(storage) => storage,
            Err(error) => {
                eprintln!("failed to initialise local storage: {error}");
                LocalStorage {
                    paths: StoragePaths::default(),
                    data: LocalStorageData::default(),
                }
            }
        }
    }
}

impl LocalStorage {
    pub fn load_or_initialise() -> Result<Self, StorageError> {
        let paths = StoragePaths::new()?;
        paths.create_dirs()?;

        let settings = read_or_create_json(&paths.settings_file, &GeneralSettings::default())?;
        let providers = read_or_create_json(&paths.providers_file, &default_rom_providers())?;
        let roms = read_or_create_json(&paths.roms_file, &Vec::<RomMetadata>::new())?;
        let timestamps =
            read_or_create_json(&paths.timestamps_file, &LastPlayedTimestamps::default())?;
        let mut input_mappings = read_or_create_json(&paths.input_file, &default_input_mappings())?;
        let mut input_mappings_changed = false;
        for mapping in &mut input_mappings {
            input_mappings_changed |= ensure_essential_navigation_mappings(mapping);
        }
        if !input_mappings
            .iter()
            .any(|mapping| mapping.r#type == InputDeviceType::Keyboard)
        {
            input_mappings.extend(default_input_mappings());
            input_mappings_changed = true;
        }
        if input_mappings_changed {
            write_json(&paths.input_file, &input_mappings)?;
        }

        ensure_default_audio_preset(&paths)?;

        Ok(Self {
            paths,
            data: LocalStorageData {
                settings,
                providers,
                roms,
                timestamps,
                input_mappings,
            },
        })
    }

    pub fn save_settings(&self) -> Result<(), StorageError> {
        write_json(&self.paths.settings_file, &self.data.settings)
    }

    pub fn save_providers(&self) -> Result<(), StorageError> {
        write_json(&self.paths.providers_file, &self.data.providers)
    }

    pub fn save_roms(&self) -> Result<(), StorageError> {
        write_json(&self.paths.roms_file, &self.data.roms)
    }

    pub fn save_timestamps(&self) -> Result<(), StorageError> {
        write_json(&self.paths.timestamps_file, &self.data.timestamps)
    }

    pub fn save_input_mappings(&self) -> Result<(), StorageError> {
        write_json(&self.paths.input_file, &self.data.input_mappings)
    }

    pub fn rom_dir(&self, rom_id: &str) -> PathBuf {
        self.paths.roms_dir.join(rom_id)
    }

    pub fn rom_file_path(&self, rom_id: &str, filename: &str) -> PathBuf {
        self.rom_dir(rom_id).join(filename)
    }

    pub fn auto_save_path(&self, rom_id: &str) -> PathBuf {
        self.rom_dir(rom_id).join("auto.gsv")
    }

    pub fn manual_save_path(&self, rom_id: &str, slot: u8) -> Option<PathBuf> {
        (slot < 10).then(|| self.rom_dir(rom_id).join(format!("{slot}.sav")))
    }

    pub fn sram_path(&self, rom_id: &str) -> PathBuf {
        self.rom_dir(rom_id).join("sram.dat")
    }

    pub fn oscillator_path(&self, rom_id: &str) -> PathBuf {
        self.rom_dir(rom_id).join("oscillator.dat")
    }
}

fn read_or_create_json<T>(path: &Path, default_value: &T) -> Result<T, StorageError>
where
    T: DeserializeOwned + Serialize,
{
    if path.exists() {
        let bytes = fs::read(path)?;
        serde_json::from_slice(&bytes).map_err(|source| StorageError::Json {
            path: path.to_path_buf(),
            source,
        })
    } else {
        write_json(path, default_value)?;
        let bytes = fs::read(path)?;
        serde_json::from_slice(&bytes).map_err(|source| StorageError::Json {
            path: path.to_path_buf(),
            source,
        })
    }
}

fn write_json<T>(path: &Path, value: &T) -> Result<(), StorageError>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(value).map_err(|source| StorageError::Json {
        path: path.to_path_buf(),
        source,
    })?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

fn ensure_default_audio_preset(paths: &StoragePaths) -> Result<(), StorageError> {
    let preset_path = paths.default_audio_preset_file();
    if !preset_path.exists() {
        fs::write(preset_path, "{}\n")?;
    }
    Ok(())
}
