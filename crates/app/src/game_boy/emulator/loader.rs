use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task, futures::check_ready};

use crate::game_boy::emulator::rom::{MemoryBankController, RomProperties};
use crate::game_boy::emulator::{GameBoyCore, GameBoyEmulator};
use crate::storage::LocalStorage;
use crate::storage::data::RomMetadata;
use crate::storage::paths::cache_remote_rom;
use crate::storage::provider_sync::http_get_bytes;
use crate::storage::providers::{RemoteApiProvider, RomProvider};
use crate::storage::rom_identifier::rom_identifier;

const MIN_ROM_BYTES: usize = 32_768;
const HEADER_TITLE_START: usize = 0x0134;
const HEADER_TITLE_LEN: usize = 16;
const SGB_FLAG_INDEX: usize = 0x0146;
const CART_TYPE_INDEX: usize = 0x0147;
const ROM_SIZE_INDEX: usize = 0x0148;
const RAM_SIZE_INDEX: usize = 0x0149;
const HEADER_CHECKSUM_INDEX: usize = 0x014d;

const OFFICIAL_LOGO: [u8; 48] = [
    0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0c, 0x00, 0x0d,
    0x00, 0x08, 0x11, 0x1f, 0x88, 0x89, 0x00, 0x0e, 0xdc, 0xcc, 0x6e, 0xe6, 0xdd, 0xdd, 0xd9, 0x99,
    0xbb, 0xbb, 0x67, 0x63, 0x6e, 0x0e, 0xec, 0xcc, 0xdd, 0xdc, 0x99, 0x9f, 0xbb, 0xb9, 0x33, 0x3e,
];

#[derive(Clone, Debug, Default, Resource)]
pub(crate) struct GameBoyRomLoadRequest {
    pub(crate) rom_index: Option<usize>,
    pub(crate) resume_auto_save: bool,
}

#[derive(Clone, Debug, Resource)]
pub(crate) struct GameBoyLoadStatus {
    state: GameBoyLoadState,
}

impl Default for GameBoyLoadStatus {
    fn default() -> Self {
        Self {
            state: GameBoyLoadState::Idle,
        }
    }
}

impl GameBoyLoadStatus {
    pub(crate) fn overlay_message(&self) -> Option<&str> {
        match &self.state {
            GameBoyLoadState::Ready => None,
            GameBoyLoadState::Idle => Some("No ROM selected."),
            GameBoyLoadState::Loading(message) | GameBoyLoadState::Error(message) => {
                Some(message.as_str())
            }
        }
    }

    fn set_loading(&mut self, message: impl Into<String>) {
        self.state = GameBoyLoadState::Loading(message.into());
    }

    fn set_ready(&mut self) {
        self.state = GameBoyLoadState::Ready;
    }

    fn set_error(&mut self, error: GameBoyLoadError) {
        self.state = GameBoyLoadState::Error(error.to_string());
    }
}

#[derive(Clone, Debug)]
enum GameBoyLoadState {
    Idle,
    Loading(String),
    Ready,
    Error(String),
}

#[derive(Default, Resource)]
pub(crate) struct GameBoyRomLoadTaskState {
    task: Option<Task<GameBoyRomLoadTaskResult>>,
}

pub(crate) fn begin_game_boy_rom_load(
    mut task_state: ResMut<GameBoyRomLoadTaskState>,
    mut status: ResMut<GameBoyLoadStatus>,
    request: Res<GameBoyRomLoadRequest>,
    storage: Res<LocalStorage>,
) {
    task_state.task = None;

    let Some(rom_index) = request.rom_index else {
        status.set_error(GameBoyLoadError::NoRomSelected);
        return;
    };
    let Some(rom) = storage.data.roms.get(rom_index).cloned() else {
        status.set_error(GameBoyLoadError::RomMetadataMissing);
        return;
    };
    let Some(provider) = storage
        .data
        .providers
        .iter()
        .find(|provider| provider.uuid == rom.provider_id)
        .cloned()
    else {
        status.set_error(GameBoyLoadError::ProviderMissing);
        return;
    };

    status.set_loading(format!("Loading {}...", rom_display_name(&rom)));
    let paths = storage.paths.clone();
    task_state.task = Some(IoTaskPool::get().spawn(async move {
        GameBoyRomLoadTaskResult {
            rom_index,
            result: load_rom_bytes(rom, provider, paths.roms_dir),
        }
    }));
}

pub(crate) fn has_pending_game_boy_rom_load(task_state: Res<GameBoyRomLoadTaskState>) -> bool {
    task_state.task.is_some()
}

pub(crate) fn finish_game_boy_rom_load(
    mut task_state: ResMut<GameBoyRomLoadTaskState>,
    mut status: ResMut<GameBoyLoadStatus>,
    mut storage: ResMut<LocalStorage>,
    mut emulators: Query<&mut GameBoyCore, With<GameBoyEmulator>>,
) {
    let Some(task) = task_state.task.as_mut() else {
        return;
    };
    let Some(result) = check_ready(task) else {
        return;
    };
    task_state.task = None;

    let loaded = match result.result {
        Ok(loaded) => loaded,
        Err(error) => {
            status.set_error(error);
            return;
        }
    };

    let Some(mut emulator) = emulators.iter_mut().next() else {
        status.set_error(GameBoyLoadError::EmulatorUnavailable);
        return;
    };

    let clock_frequency_hz = match load_rom_into_emulator(&loaded, &mut emulator) {
        Ok(clock_frequency_hz) => clock_frequency_hz,
        Err(error) => {
            emulator.runtime.is_running = false;
            status.set_error(error);
            return;
        }
    };
    emulator.audio_unit.reset_for_rom_load(clock_frequency_hz);

    update_loaded_rom_storage(&mut storage, result.rom_index, &loaded.rom_id);
    status.set_ready();
}

fn load_rom_into_emulator(
    loaded: &LoadedRomBytes,
    emulator: &mut GameBoyCore,
) -> Result<i64, GameBoyLoadError> {
    let properties = parse_rom_properties(&loaded.bytes)?;
    if usize::try_from(properties.size_bytes)
        .ok()
        .is_some_and(|size_bytes| size_bytes > loaded.bytes.len())
    {
        return Err(GameBoyLoadError::InvalidHeader(format!(
            "ROM header declares {} bytes, but the loaded file has {} bytes.",
            properties.size_bytes,
            loaded.bytes.len()
        )));
    }

    if !emulator.reset_for_rom_load(properties, loaded.opened_file_name.clone(), &loaded.bytes) {
        return Err(GameBoyLoadError::Unknown(
            "emulated ROM memory is unavailable".to_string(),
        ));
    }

    Ok(emulator.cpu_timing.clock_frequency_hz)
}

fn parse_rom_properties(bytes: &[u8]) -> Result<RomProperties, GameBoyLoadError> {
    if bytes.len() < MIN_ROM_BYTES {
        return Err(GameBoyLoadError::NotGameBoyRom);
    }
    if bytes.get(0x0104..0x0104 + OFFICIAL_LOGO.len()).is_none() {
        return Err(GameBoyLoadError::HeaderCouldNotBeRead);
    }

    let title_bytes = bytes
        .get(HEADER_TITLE_START..HEADER_TITLE_START + HEADER_TITLE_LEN)
        .ok_or(GameBoyLoadError::HeaderCouldNotBeRead)?;
    let mut title = [0; 17];
    title[..HEADER_TITLE_LEN].copy_from_slice(title_bytes);

    let last_title_byte = title[15];
    let cgb_flag = matches!(last_title_byte, 0x80 | 0xc0);
    if cgb_flag {
        title[11] = 0;
    }

    let cart_type = header_byte(bytes, CART_TYPE_INDEX)?;
    let size_enum = header_byte(bytes, ROM_SIZE_INDEX)?;
    let ram_size_enum = header_byte(bytes, RAM_SIZE_INDEX)?;
    let mut sgb_flag = header_byte(bytes, SGB_FLAG_INDEX)? == 0x03;
    if cgb_flag {
        sgb_flag = false;
    }

    let mut properties = RomProperties {
        valid: true,
        title,
        cgb_flag,
        sgb_flag,
        cart_type: u32::from(cart_type),
        size_enum: u32::from(size_enum),
        check_sum: u32::from(header_byte(bytes, HEADER_CHECKSUM_INDEX)?),
        ..Default::default()
    };

    apply_cartridge_type(cart_type, &mut properties)?;
    apply_rom_size(size_enum, &mut properties)?;
    apply_ram_size(ram_size_enum, &mut properties)?;
    Ok(properties)
}

fn header_byte(bytes: &[u8], index: usize) -> Result<u8, GameBoyLoadError> {
    bytes
        .get(index)
        .copied()
        .ok_or(GameBoyLoadError::HeaderCouldNotBeRead)
}

fn apply_cartridge_type(
    cart_type: u8,
    properties: &mut RomProperties,
) -> Result<(), GameBoyLoadError> {
    match cart_type {
        0x00 => properties.mbc = MemoryBankController::None,
        0x08 => {
            properties.mbc = MemoryBankController::None;
            properties.has_sram = true;
        }
        0x09 => {
            properties.mbc = MemoryBankController::None;
            properties.has_sram = true;
        }
        0x01 => properties.mbc = MemoryBankController::Mbc1,
        0x02 => {
            properties.mbc = MemoryBankController::Mbc1;
            properties.has_sram = true;
        }
        0x03 => {
            properties.mbc = MemoryBankController::Mbc1;
            properties.has_sram = true;
        }
        0x05 | 0x06 => properties.mbc = MemoryBankController::Mbc2,
        0x0f => properties.mbc = MemoryBankController::Mbc3,
        0x10 | 0x12 | 0x13 => {
            properties.mbc = MemoryBankController::Mbc3;
            properties.has_sram = true;
        }
        0x11 => properties.mbc = MemoryBankController::Mbc3,
        0x19 => properties.mbc = MemoryBankController::Mbc5,
        0x1a | 0x1b => {
            properties.mbc = MemoryBankController::Mbc5;
            properties.has_sram = true;
        }
        0x1c => {
            properties.mbc = MemoryBankController::Mbc5;
            properties.has_rumble = true;
        }
        0x1d | 0x1e => {
            properties.mbc = MemoryBankController::Mbc5;
            properties.has_sram = true;
            properties.has_rumble = true;
        }
        _ => {
            return Err(GameBoyLoadError::InvalidHeader(format!(
                "unsupported cartridge type 0x{cart_type:02x}"
            )));
        }
    }
    Ok(())
}

fn apply_rom_size(size_enum: u8, properties: &mut RomProperties) -> Result<(), GameBoyLoadError> {
    let (size_bytes, bank_select_mask) = match size_enum {
        0x00 => (32_768, 0x00),
        0x01 => (65_536, 0x03),
        0x02 => (131_072, 0x07),
        0x03 => (262_144, 0x0f),
        0x04 => (524_288, 0x1f),
        0x05 => (1_048_576, 0x3f),
        0x06 => (2_097_152, 0x7f),
        0x07 => (4_194_304, 0xff),
        0x08 => (8_388_608, 0x1ff),
        0x52 => (1_179_648, 0x7f),
        0x53 => (1_310_720, 0x7f),
        0x54 => (1_572_864, 0x7f),
        _ => {
            return Err(GameBoyLoadError::InvalidHeader(format!(
                "unsupported ROM size code 0x{size_enum:02x}"
            )));
        }
    };
    properties.size_bytes = size_bytes;
    properties.bank_select_mask = bank_select_mask;
    Ok(())
}

fn apply_ram_size(size_enum: u8, properties: &mut RomProperties) -> Result<(), GameBoyLoadError> {
    match size_enum {
        0x00 | 0x01 | 0x02 | 0x03 => Ok(()),
        _ => Err(GameBoyLoadError::InvalidHeader(format!(
            "unsupported RAM size code 0x{size_enum:02x}"
        ))),
    }?;
    if properties.mbc == MemoryBankController::Mbc2 {
        properties.has_sram = true;
    }
    Ok(())
}

fn load_rom_bytes(
    rom: RomMetadata,
    provider: RomProvider,
    roms_dir: PathBuf,
) -> Result<LoadedRomBytes, GameBoyLoadError> {
    let opened_file_name;
    let bytes = if let Some(path) = &provider.absolute_local_dir_path {
        opened_file_name = path.join(&rom.file_name).display().to_string();
        local_rom_bytes(path, &rom.file_name)?
    } else {
        let cached_path = rom
            .id
            .as_ref()
            .map(|id| roms_dir.join(id).join(&rom.file_name));
        if let Some(path) = cached_path.filter(|path| path.is_file()) {
            opened_file_name = path.display().to_string();
            read_file(&path)?
        } else {
            let url = provider_rom_download_url(&provider, &rom)?;
            opened_file_name = url.clone();
            let bytes = http_get_bytes(&url)
                .map_err(|error| GameBoyLoadError::RemoteDownloadFailed(error.to_string()))?;
            if !cache_remote_rom(&roms_dir, &rom.file_name, &bytes) {
                warn!("failed to cache downloaded ROM: {}", rom.file_name);
            }
            bytes
        }
    };
    let rom_id = rom_identifier(&bytes);

    Ok(LoadedRomBytes {
        file_name: rom.file_name,
        opened_file_name,
        bytes,
        rom_id,
    })
}

fn local_rom_bytes(path: &Path, file_name: &str) -> Result<Vec<u8>, GameBoyLoadError> {
    #[cfg(target_os = "android")]
    if let Some(uri) = path.to_str().filter(|path| path.starts_with("content://")) {
        let files = crate::platform::read_android_local_directory_roms(uri)
            .map_err(GameBoyLoadError::OpenFailedMessage)?;
        return files
            .into_iter()
            .find(|file| file.file_name == file_name)
            .map(|file| file.bytes)
            .ok_or_else(|| {
                GameBoyLoadError::OpenFailedMessage(format!("{file_name} was not found in {uri}"))
            });
    }

    read_file(&path.join(file_name))
}

fn read_file(path: &Path) -> Result<Vec<u8>, GameBoyLoadError> {
    fs::read(path).map_err(|source| GameBoyLoadError::OpenFailed {
        path: path.to_path_buf(),
        source: source.to_string(),
    })
}

fn provider_rom_download_url(
    provider: &RomProvider,
    rom: &RomMetadata,
) -> Result<String, GameBoyLoadError> {
    if let Some(url) = &provider.remote_file_url {
        return Ok(url.clone());
    }
    let Some(api) = &provider.remote_api else {
        return Err(GameBoyLoadError::OpenFailedMessage(
            "ROM provider does not have a readable source.".to_string(),
        ));
    };

    remote_api_download_url(api, rom)
}

fn remote_api_download_url(
    api: &RemoteApiProvider,
    rom: &RomMetadata,
) -> Result<String, GameBoyLoadError> {
    let mut url = api.download_url.clone();
    if url.contains("{id}") {
        let Some(id) = &rom.remote_provider_id else {
            return Err(GameBoyLoadError::OpenFailedMessage(
                "Remote ROM item does not include a provider ID.".to_string(),
            ));
        };
        url = url.replace("{id}", id);
    }
    Ok(url.replace("{filename}", &rom.file_name))
}

fn update_loaded_rom_storage(storage: &mut LocalStorage, rom_index: usize, rom_id: &str) {
    let changed = if let Some(rom) = storage.data.roms.get_mut(rom_index) {
        if rom.id.as_deref() != Some(rom_id) {
            rom.id = Some(rom_id.to_string());
            true
        } else {
            false
        }
    } else {
        false
    };

    if changed {
        if let Err(error) = storage.save_roms() {
            warn!("failed to save ROM metadata after loading ROM: {error}");
        }
    }
}

fn rom_display_name(rom: &RomMetadata) -> String {
    rom.friendly_name
        .clone()
        .unwrap_or_else(|| rom.file_name.clone())
}

#[derive(Debug)]
struct GameBoyRomLoadTaskResult {
    rom_index: usize,
    result: Result<LoadedRomBytes, GameBoyLoadError>,
}

#[derive(Debug)]
struct LoadedRomBytes {
    file_name: String,
    opened_file_name: String,
    bytes: Vec<u8>,
    rom_id: String,
}

#[derive(Debug)]
enum GameBoyLoadError {
    NoRomSelected,
    RomMetadataMissing,
    ProviderMissing,
    EmulatorUnavailable,
    OpenFailed { path: PathBuf, source: String },
    OpenFailedMessage(String),
    RemoteDownloadFailed(String),
    NotGameBoyRom,
    HeaderCouldNotBeRead,
    InvalidHeader(String),
    Unknown(String),
}

impl fmt::Display for GameBoyLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRomSelected => write!(formatter, "No ROM was selected."),
            Self::RomMetadataMissing => write!(formatter, "ROM metadata could not be found."),
            Self::ProviderMissing => write!(formatter, "The ROM provider could not be found."),
            Self::EmulatorUnavailable => write!(formatter, "The emulator is not available."),
            Self::OpenFailed { path, source } => write!(
                formatter,
                "The file could not be opened for reading: {}: {source}",
                path.display()
            ),
            Self::OpenFailedMessage(message) => {
                write!(
                    formatter,
                    "The file could not be opened for reading: {message}"
                )
            }
            Self::RemoteDownloadFailed(message) => {
                write!(
                    formatter,
                    "The file could not be opened for reading: {message}"
                )
            }
            Self::NotGameBoyRom => write!(formatter, "The file is not a GameBoy ROM."),
            Self::HeaderCouldNotBeRead => write!(formatter, "The ROM header could not be read."),
            Self::InvalidHeader(message) => {
                write!(
                    formatter,
                    "The ROM header contains invalid values: {message}"
                )
            }
            Self::Unknown(message) => write!(formatter, "An unknown error occurred: {message}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_rom_header_loads_as_plain_game_boy_rom() {
        let rom = minimal_rom(0x00, 0x00, 0x00);

        let properties = parse_rom_properties(&rom).expect("minimal ROM should parse");

        assert!(properties.valid);
        assert_eq!(properties.mbc, MemoryBankController::None);
        assert_eq!(properties.size_bytes, MIN_ROM_BYTES as i32);
        assert!(!properties.cgb_flag);
        assert!(!properties.sgb_flag);
    }

    #[test]
    fn unsupported_cartridge_type_is_a_header_error() {
        let rom = minimal_rom(0xff, 0x00, 0x00);

        let error = parse_rom_properties(&rom).expect_err("cartridge type should fail");

        assert!(matches!(error, GameBoyLoadError::InvalidHeader(_)));
    }

    fn minimal_rom(cart_type: u8, rom_size: u8, ram_size: u8) -> Vec<u8> {
        let mut rom = vec![0; MIN_ROM_BYTES];
        rom[0x0104..0x0104 + OFFICIAL_LOGO.len()].copy_from_slice(&OFFICIAL_LOGO);
        rom[HEADER_TITLE_START..HEADER_TITLE_START + 4].copy_from_slice(b"TEST");
        rom[SGB_FLAG_INDEX] = 0x00;
        rom[CART_TYPE_INDEX] = cart_type;
        rom[ROM_SIZE_INDEX] = rom_size;
        rom[RAM_SIZE_INDEX] = ram_size;
        rom[HEADER_CHECKSUM_INDEX] = 0x00;
        rom
    }
}
