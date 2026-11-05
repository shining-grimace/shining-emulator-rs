use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{Catalog, metadata};
use crate::storage::{data::RomMetadata, errors::StorageError};

#[derive(Deserialize)]
pub(super) struct Config {
    pub(super) base_url: String,
    pub(super) release: String,
}

#[derive(Deserialize)]
struct Directory {
    schema_version: u32,
    releases: Vec<Release>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(super) struct Release {
    pub(super) id: String,
    pub(super) catalogs: CatalogRefs,
    pub(super) licenses: Vec<FileRef>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(super) struct CatalogRefs {
    pub(super) in_app: FileRef,
    headless: FileRef,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(super) struct FileRef {
    pub(super) path: String,
    sha256: String,
}

impl FileRef {
    pub(super) fn verify(&self, bytes: &[u8]) -> bool {
        hex::encode(Sha256::digest(bytes)) == self.sha256
    }
}

#[derive(Deserialize)]
struct SiteCatalog {
    schema_version: u32,
    release: String,
    #[serde(flatten)]
    catalog: Catalog,
}

pub(super) fn error(message: &str) -> StorageError {
    StorageError::Provider(format!("Validation ROM catalog: {message}"))
}

pub(super) fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, StorageError> {
    serde_json::from_slice(bytes).map_err(|failure| error(&failure.to_string()))
}

pub(super) fn config() -> Result<Config, StorageError> {
    decode(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-roms/site-config.json"
    )))
}

pub(super) fn pinned_release() -> Result<Release, StorageError> {
    let directory: Directory = decode(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-roms/site-directory.json"
    )))?;
    select_release(directory)
}

fn select_release(directory: Directory) -> Result<Release, StorageError> {
    if directory.schema_version != 1 {
        return Err(error("unsupported directory schema"));
    }
    let config = config()?;
    let mut matching = directory
        .releases
        .into_iter()
        .filter(|release| release.id == config.release);
    let release = matching
        .next()
        .ok_or_else(|| error("pinned release is missing"))?;
    if matching.next().is_some() {
        return Err(error("duplicate release"));
    }
    Ok(release)
}

pub(super) fn roms_with<F>(fetch: F) -> Result<Vec<RomMetadata>, StorageError>
where
    F: Fn(&str) -> Result<Vec<u8>, StorageError>,
{
    let config = config()?;
    let remote = select_release(decode(&fetch(&format!(
        "{}directory.json",
        config.base_url
    ))?)?)?;
    if remote != pinned_release()? {
        return Err(error("website release differs from the pinned directory"));
    }
    let bytes = fetch(&format!(
        "{}{}",
        config.base_url, remote.catalogs.in_app.path
    ))?;
    if !remote.catalogs.in_app.verify(&bytes) {
        return Err(error("in-app catalog checksum mismatch"));
    }
    let published: SiteCatalog = decode(&bytes)?;
    let local = super::catalog()?;
    let expected: Vec<_> = local
        .cases
        .into_iter()
        .filter(|case| case.in_app && case.skip.is_none())
        .map(|mut case| {
            // The in-app catalog's purpose implies membership, and it does
            // not carry headless execution settings.
            case.in_app = false;
            case.protocol = super::Protocol::Breakpoint;
            case.seconds = 12;
            case
        })
        .collect();
    if published.schema_version != 1
        || published.release != config.release
        || published.catalog.collection != local.collection
        || published.catalog.cases != expected
    {
        return Err(error(
            "website selection differs from the pinned in-app cases",
        ));
    }
    metadata()
}
