use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct StorageFile {
    pub kind: StorageFileKind,
    pub name: String,
    pub bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StorageFileKind {
    DownloadedRom,
    Sram,
    Oscillator,
    AutoSave,
    SaveState(u8),
    Other,
}

impl StorageFileKind {
    pub fn from_name(name: &str, rom_file_name: &str) -> Self {
        if name == rom_file_name {
            Self::DownloadedRom
        } else if name == "sram.dat" {
            Self::Sram
        } else if name == "oscillator.dat" {
            Self::Oscillator
        } else if name == "auto.gsv" {
            Self::AutoSave
        } else if let Some(slot) = save_state_slot(name) {
            Self::SaveState(slot)
        } else {
            Self::Other
        }
    }

    pub fn label(self, name: &str) -> String {
        match self {
            Self::DownloadedRom => "Downloaded ROM".to_string(),
            Self::Sram => "SRAM".to_string(),
            Self::Oscillator => "Oscillator state".to_string(),
            Self::AutoSave => "Auto-save".to_string(),
            Self::SaveState(slot) => format!("Save state {slot}"),
            Self::Other => file_stem_label(name),
        }
    }

    fn sort_order(self, name: &str) -> (u8, String) {
        match self {
            Self::DownloadedRom => (0, String::new()),
            Self::Sram => (1, String::new()),
            Self::Oscillator => (2, String::new()),
            Self::AutoSave => (3, String::new()),
            Self::SaveState(slot) => (4, format!("{slot:02}")),
            Self::Other => (5, name.to_string()),
        }
    }
}

pub fn storage_files(rom_dir: &Path, rom_file_name: &str) -> Vec<StorageFile> {
    let Ok(entries) = fs::read_dir(rom_dir) else {
        return Vec::new();
    };
    let mut files = entries
        .filter_map(Result::ok)
        .filter_map(|entry| storage_file(entry.path(), rom_file_name))
        .collect::<Vec<_>>();
    files.sort_by(|left, right| {
        left.kind
            .sort_order(&left.name)
            .cmp(&right.kind.sort_order(&right.name))
            .then_with(|| left.name.cmp(&right.name))
    });
    files
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    if bytes >= MB {
        format!("{} MB", rounded_unit(bytes, MB))
    } else if bytes >= KB {
        format!("{} KB", rounded_unit(bytes, KB))
    } else {
        format!("{bytes} B")
    }
}

fn storage_file(path: PathBuf, rom_file_name: &str) -> Option<StorageFile> {
    let metadata = fs::metadata(&path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let name = path.file_name()?.to_string_lossy().to_string();
    Some(StorageFile {
        kind: StorageFileKind::from_name(&name, rom_file_name),
        name,
        bytes: metadata.len(),
    })
}

fn rounded_unit(bytes: u64, unit: u64) -> u64 {
    ((bytes as f64) / (unit as f64)).round().max(1.0) as u64
}

fn save_state_slot(name: &str) -> Option<u8> {
    name.strip_suffix(".sav")
        .or_else(|| name.strip_suffix(".gsv"))
        .and_then(|slot| slot.parse::<u8>().ok())
        .filter(|slot| *slot < 10)
}

fn file_stem_label(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "Other".to_string())
}
