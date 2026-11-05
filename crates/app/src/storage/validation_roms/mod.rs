//! Shared pinned selections for hosted diagnostics and headless regression ROMs.
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::data::RomMetadata;
use super::errors::StorageError;
use super::providers::VALIDATION_PROVIDER_UUID;

mod download;
mod site;
#[cfg(test)]
mod tests;

#[derive(Debug, Deserialize)]
pub(crate) struct Catalog {
    pub(crate) collection: String,
    pub(crate) cases: Vec<Case>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(crate) struct Case {
    pub(crate) path: String,
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) protocol: Protocol,
    #[serde(default = "default_seconds")]
    pub(crate) seconds: u64,
    #[serde(default)]
    pub(crate) in_app: bool,
    pub(crate) sha256: String,
    pub(crate) name: String,
    pub(crate) author: String,
    pub(crate) license: String,
    pub(crate) skip: Option<String>,
    pub(crate) screen: Option<String>,
    pub(crate) screen_sha256: Option<String>,
}

fn default_seconds() -> u64 {
    12
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Protocol {
    #[default]
    Breakpoint,
    Blargg,
    Screen,
}

impl Case {
    pub(crate) fn file_name(&self) -> String {
        format!("{}/{}", self.model, self.path)
    }

    pub(crate) fn verify(&self, bytes: &[u8]) -> bool {
        hex::encode(Sha256::digest(bytes)) == self.sha256
    }
}

pub(crate) fn catalog() -> Result<Catalog, StorageError> {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-roms/catalog.json"
    )))
    .map_err(|error| StorageError::Provider(format!("Invalid validation ROM catalog: {error}")))
}

pub(crate) fn metadata() -> Result<Vec<RomMetadata>, StorageError> {
    Ok(catalog()?
        .cases
        .into_iter()
        .filter(|case| case.in_app && case.skip.is_none())
        .map(|case| {
            let file_name = case.file_name();
            RomMetadata {
                id: None,
                provider_id: VALIDATION_PROVIDER_UUID,
                file_name: file_name.clone(),
                friendly_name: Some(case.name),
                author: Some(case.author),
                license: Some(case.license),
                remote_provider_id: Some(file_name),
            }
        })
        .collect())
}

pub(crate) fn case(file_name: &str) -> Result<Case, StorageError> {
    catalog()?
        .cases
        .into_iter()
        .find(|case| case.in_app && case.skip.is_none() && case.file_name() == file_name)
        .ok_or_else(|| StorageError::Provider(format!("Unknown validation ROM: {file_name}")))
}

pub(crate) fn roms() -> Result<Vec<RomMetadata>, StorageError> {
    site::roms_with(super::provider_sync::http_get_bytes)
}

pub(crate) use download::load;
