use serde::{Deserialize, Serialize};

/// OAM data observed by the PPU, independently of later DMA/CPU changes.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(try_from = "Snapshot", into = "Snapshot")]
pub(crate) struct OamScanline {
    pub(crate) bytes: [u8; 160],
    pub(crate) selected: u64,
    pub(crate) fetched: u64,
    pub(crate) valid: bool,
}

impl Default for OamScanline {
    fn default() -> Self {
        Self {
            bytes: [0; 160],
            selected: 0,
            fetched: 0,
            valid: false,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Snapshot {
    bytes: Vec<u8>,
    selected: u64,
    fetched: u64,
    valid: bool,
}

impl From<OamScanline> for Snapshot {
    fn from(line: OamScanline) -> Self {
        Self {
            bytes: line.bytes.to_vec(),
            selected: line.selected,
            fetched: line.fetched,
            valid: line.valid,
        }
    }
}

impl TryFrom<Snapshot> for OamScanline {
    type Error = String;
    fn try_from(snapshot: Snapshot) -> Result<Self, Self::Error> {
        if snapshot.selected >> 40 != 0
            || snapshot.fetched & !snapshot.selected != 0
            || snapshot.selected.count_ones() > 10
        {
            return Err("invalid PPU OAM selection mask".to_string());
        }
        let bytes = snapshot
            .bytes
            .try_into()
            .map_err(|_| "PPU OAM snapshot must contain 160 bytes".to_string())?;
        Ok(Self {
            bytes,
            selected: snapshot.selected,
            fetched: snapshot.fetched,
            valid: snapshot.valid,
        })
    }
}
