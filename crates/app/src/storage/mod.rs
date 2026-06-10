#![allow(dead_code)]

pub mod data;
pub mod errors;
pub mod input_mappings;
pub mod paths;
pub mod provider_sync;
pub mod providers;
pub mod rom_data;
pub mod rom_identifier;

use std::fs;
use std::path::{Path, PathBuf};
use std::{collections::HashSet, mem};

use bevy::prelude::*;
use serde::{Serialize, de::DeserializeOwned};

use crate::audio::preset_graph::default_audio_preset;
use crate::input::mappings::ensure_essential_navigation_mappings;
use crate::storage::data::{GeneralSettings, LastPlayedTimestamps, LocalStorageData, RomMetadata};
use crate::storage::errors::StorageError;
use crate::storage::input_mappings::{InputDeviceType, default_input_mappings};
use crate::storage::paths::StoragePaths;
use crate::storage::provider_sync::{
    ProviderSyncMessages, ProviderSyncTaskResult, ProviderSyncTaskState, sync_provider,
};
use crate::storage::providers::{apply_default_provider_updates, default_rom_providers};

#[derive(Resource)]
pub struct LocalStorage {
    pub paths: StoragePaths,
    pub data: LocalStorageData,
}

pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LocalStorage>()
            .init_resource::<ProviderSyncMessages>()
            .init_resource::<ProviderSyncTaskState>();
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
        let mut providers = read_or_create_json(&paths.providers_file, &default_rom_providers())?;
        let providers_changed = apply_default_provider_updates(&mut providers);
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
        if providers_changed {
            write_json(&paths.providers_file, &providers)?;
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

    pub fn sync_enabled_providers(&mut self) -> Vec<String> {
        let providers = self
            .data
            .providers
            .iter()
            .filter(|provider| provider.enabled)
            .cloned()
            .collect::<Vec<_>>();
        self.sync_provider_list(&providers)
    }

    pub fn sync_provider_at(&mut self, index: usize) -> Result<usize, StorageError> {
        let provider = self
            .data
            .providers
            .get(index)
            .cloned()
            .ok_or_else(|| StorageError::Provider("Provider was not found.".to_string()))?;
        let result = sync_provider(&provider)?;
        let count = result.roms.len();
        self.replace_provider_roms(result.provider_id, result.roms);
        if let Some(provider) = self.data.providers.get_mut(index) {
            provider.last_fetched = Some(unix_timestamp());
        }
        self.save_providers()?;
        self.save_roms()?;
        Ok(count)
    }

    pub fn sync_provider_list(
        &mut self,
        providers: &[crate::storage::providers::RomProvider],
    ) -> Vec<String> {
        let mut failures = Vec::new();
        for provider in providers {
            match sync_provider(provider) {
                Ok(result) => {
                    self.replace_provider_roms(result.provider_id, result.roms);
                    if let Some(stored_provider) = self
                        .data
                        .providers
                        .iter_mut()
                        .find(|stored_provider| stored_provider.uuid == provider.uuid)
                    {
                        stored_provider.last_fetched = Some(unix_timestamp());
                    }
                }
                Err(error) => failures.push(format!("{}: {error}", provider.friendly_name)),
            }
        }
        if let Err(error) = self.save_providers() {
            failures.push(format!("Provider settings could not be saved: {error}"));
        }
        if let Err(error) = self.save_roms() {
            failures.push(format!("ROM metadata could not be saved: {error}"));
        }
        failures
    }

    pub fn apply_provider_sync_result(&mut self, result: ProviderSyncTaskResult) -> Vec<String> {
        let mut failures = result.failures;
        for result in result.results {
            self.replace_provider_roms(result.provider_id, result.roms);
            if let Some(provider) = self
                .data
                .providers
                .iter_mut()
                .find(|provider| provider.uuid == result.provider_id)
            {
                provider.last_fetched = Some(unix_timestamp());
            }
        }
        if let Err(error) = self.save_providers() {
            failures.push(format!("Provider settings could not be saved: {error}"));
        }
        if let Err(error) = self.save_roms() {
            failures.push(format!("ROM metadata could not be saved: {error}"));
        }
        failures
    }

    fn replace_provider_roms(&mut self, provider_id: uuid::Uuid, roms: Vec<RomMetadata>) {
        let played_rom_ids = self
            .data
            .timestamps
            .last_played
            .iter()
            .map(|timestamp| timestamp.id.clone())
            .collect::<HashSet<_>>();
        let mut incoming_roms = roms;
        let existing_roms = mem::take(&mut self.data.roms);

        for existing_rom in existing_roms {
            if existing_rom.provider_id != provider_id {
                self.data.roms.push(existing_rom);
                continue;
            }

            if let Some(index) = incoming_roms
                .iter()
                .position(|rom| same_provider_rom(rom, &existing_rom))
            {
                let mut replacement = incoming_roms.remove(index);
                if replacement.id.is_none() {
                    replacement.id = existing_rom.id;
                }
                self.data.roms.push(replacement);
            } else if rom_was_played(&existing_rom, &played_rom_ids) {
                self.data.roms.push(existing_rom);
            }
        }

        self.data.roms.extend(incoming_roms);
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

fn same_provider_rom(left: &RomMetadata, right: &RomMetadata) -> bool {
    left.provider_id == right.provider_id && rom_sync_key(left) == rom_sync_key(right)
}

fn rom_sync_key(rom: &RomMetadata) -> String {
    if let Some(remote_provider_id) = &rom.remote_provider_id {
        format!("remote:{remote_provider_id}:{}", rom.file_name)
    } else if let Some(id) = &rom.id {
        format!("id:{id}")
    } else {
        format!("file:{}", rom.file_name)
    }
}

fn rom_was_played(rom: &RomMetadata, played_rom_ids: &HashSet<String>) -> bool {
    rom.id
        .as_ref()
        .is_some_and(|id| played_rom_ids.contains(id))
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
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
        write_json(&preset_path, &default_audio_preset())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::data::LastPlayedTimestamp;
    use uuid::Uuid;

    const PROVIDER_ID: Uuid = Uuid::from_u128(0xaaaaaaaa_aaaa_aaaa_aaaa_aaaaaaaaaaaa);
    const OTHER_PROVIDER_ID: Uuid = Uuid::from_u128(0xbbbbbbbb_bbbb_bbbb_bbbb_bbbbbbbbbbbb);

    #[test]
    fn provider_sync_removes_unplayed_roms_missing_from_latest_result() {
        let mut storage = local_storage(vec![
            remote_rom(PROVIDER_ID, "old", "old.gb", None),
            remote_rom(OTHER_PROVIDER_ID, "other", "other.gb", None),
        ]);

        storage.replace_provider_roms(
            PROVIDER_ID,
            vec![remote_rom(PROVIDER_ID, "new", "new.gb", None)],
        );

        assert_eq!(
            rom_names(&storage),
            vec!["other.gb".to_string(), "new.gb".to_string()]
        );
    }

    #[test]
    fn provider_sync_preserves_played_roms_missing_from_latest_result() {
        let mut storage = local_storage(vec![remote_rom(
            PROVIDER_ID,
            "old",
            "old.gb",
            Some("played-rom"),
        )]);
        storage
            .data
            .timestamps
            .last_played
            .push(LastPlayedTimestamp {
                id: "played-rom".to_string(),
                timestamp: 100,
            });

        storage.replace_provider_roms(PROVIDER_ID, Vec::new());

        assert_eq!(rom_names(&storage), vec!["old.gb".to_string()]);
    }

    #[test]
    fn provider_sync_preserves_existing_rom_id_for_matching_remote_result() {
        let mut storage = local_storage(vec![remote_rom(
            PROVIDER_ID,
            "slug",
            "game.gb",
            Some("downloaded-rom"),
        )]);

        storage.replace_provider_roms(
            PROVIDER_ID,
            vec![remote_rom(PROVIDER_ID, "slug", "game.gb", None)],
        );

        assert_eq!(
            storage.data.roms.first().and_then(|rom| rom.id.as_deref()),
            Some("downloaded-rom")
        );
    }

    fn local_storage(roms: Vec<RomMetadata>) -> LocalStorage {
        LocalStorage {
            paths: StoragePaths::default(),
            data: LocalStorageData { roms, ..default() },
        }
    }

    fn remote_rom(
        provider_id: Uuid,
        remote_provider_id: &str,
        file_name: &str,
        id: Option<&str>,
    ) -> RomMetadata {
        RomMetadata {
            id: id.map(str::to_string),
            provider_id,
            file_name: file_name.to_string(),
            friendly_name: None,
            author: None,
            license: None,
            remote_provider_id: Some(remote_provider_id.to_string()),
        }
    }

    fn rom_names(storage: &LocalStorage) -> Vec<String> {
        storage
            .data
            .roms
            .iter()
            .map(|rom| rom.file_name.clone())
            .collect()
    }
}
