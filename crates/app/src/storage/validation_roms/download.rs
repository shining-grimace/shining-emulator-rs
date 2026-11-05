use std::fs;
use std::path::Path;

use super::{Case, case, site};
use crate::storage::{errors::StorageError, paths::cache_remote_rom};

pub(crate) fn load(
    file_name: &str,
    cached_id: Option<&str>,
    roms_dir: &Path,
) -> Result<(Case, Vec<u8>, String), StorageError> {
    load_with(
        file_name,
        cached_id,
        roms_dir,
        crate::storage::provider_sync::http_get_bytes,
    )
}

pub(super) fn load_with<F>(
    file_name: &str,
    cached_id: Option<&str>,
    roms_dir: &Path,
    fetch: F,
) -> Result<(Case, Vec<u8>, String), StorageError>
where
    F: Fn(&str) -> Result<Vec<u8>, StorageError>,
{
    let case = case(file_name)?;
    let config = site::config()?;
    let notice_name = if case.path.starts_with("same-suite/") {
        "SameSuite-X11.txt"
    } else {
        "Mooneye-MIT.txt"
    };
    let notice = site::pinned_release()?
        .licenses
        .into_iter()
        .find(|notice| notice.path.ends_with(notice_name))
        .ok_or_else(|| site::error("license notice is missing"))?;
    let license_path = roms_dir
        .join("validation-licenses")
        .join(&config.release)
        .join(notice_name);
    if !fs::read(&license_path).is_ok_and(|bytes| notice.verify(&bytes)) {
        let bytes = fetch(&format!("{}{}", config.base_url, notice.path))?;
        if !notice.verify(&bytes) {
            return Err(site::error("license checksum mismatch"));
        }
        if let Some(parent) = license_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&license_path, bytes)?;
    }
    if let Some(id) =
        cached_id.filter(|id| id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        let path = roms_dir.join(id).join(file_name);
        if let Ok(bytes) = fs::read(&path)
            && case.verify(&bytes)
        {
            return Ok((case, bytes, path.display().to_string()));
        }
    }
    let url = format!(
        "{}v1/{}/roms/{}",
        config.base_url, config.release, case.path
    );
    let bytes = fetch(&url)?;
    if !case.verify(&bytes) {
        return Err(site::error("downloaded ROM checksum mismatch"));
    }
    if !cache_remote_rom(roms_dir, file_name, &bytes) {
        return Err(site::error("could not cache the downloaded ROM"));
    }
    Ok((case, bytes, url))
}
