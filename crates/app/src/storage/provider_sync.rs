use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use bevy::prelude::Resource;
use bevy::tasks::{IoTaskPool, Task, futures::check_ready};
use serde_json::Value;
use uuid::Uuid;

use crate::storage::data::RomMetadata;
use crate::storage::errors::StorageError;
use crate::storage::providers::{RemoteApiProvider, RomProvider};
use crate::storage::rom_identifier::rom_identifier;

const HTTP_TIMEOUT_SECONDS: u64 = 12;

static HTTP_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(HTTP_TIMEOUT_SECONDS)))
        .user_agent("shining-emulator-rs")
        .build()
        .new_agent()
});

#[derive(Clone, Debug)]
pub struct ProviderSyncResult {
    pub provider_id: Uuid,
    pub roms: Vec<RomMetadata>,
}

#[derive(Clone, Debug, Default)]
pub struct ProviderSyncTaskResult {
    pub results: Vec<ProviderSyncResult>,
    pub failures: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ProviderTestResult {
    pub rom_count: usize,
    pub message: String,
}

#[derive(Clone, Debug, Default, Resource)]
pub struct ProviderSyncMessages {
    pub failures: Vec<String>,
}

#[derive(Default, Resource)]
pub struct ProviderSyncTaskState {
    task: Option<Task<SingleProviderSyncTaskResult>>,
}

pub struct SingleProviderSyncTaskResult {
    pub provider_name: String,
    pub result: Result<ProviderSyncResult, String>,
}

impl ProviderSyncTaskState {
    pub fn is_running(&self) -> bool {
        self.task.is_some()
    }

    pub fn start_provider_sync(&mut self, provider: RomProvider) -> bool {
        if self.task.is_some() {
            return false;
        }

        let provider_name = provider.friendly_name.clone();
        self.task = Some(IoTaskPool::get().spawn(async move {
            SingleProviderSyncTaskResult {
                provider_name,
                result: sync_provider(&provider).map_err(|error| error.to_string()),
            }
        }));
        true
    }

    pub fn poll(&mut self) -> Option<SingleProviderSyncTaskResult> {
        let result = {
            let task = self.task.as_mut()?;
            check_ready(task)
        }?;
        self.task = None;
        Some(result)
    }

    pub fn clear(&mut self) {
        self.task = None;
    }
}

pub fn test_provider(provider: &RomProvider) -> Result<ProviderTestResult, StorageError> {
    if let Some(path) = &provider.absolute_local_dir_path {
        let roms = local_roms(provider.uuid, path)?;
        return Ok(ProviderTestResult {
            rom_count: roms.len(),
            message: format!("Local directory contains {} ROM file(s).", roms.len()),
        });
    }

    if let Some(url) = &provider.remote_file_url {
        http_get_bytes(url).map_err(|error| {
            StorageError::Provider(format!("Remote file could not be read: {error}"))
        })?;
        return Ok(ProviderTestResult {
            rom_count: 1,
            message: "Remote file is reachable.".to_string(),
        });
    }

    if let Some(api) = &provider.remote_api {
        let roms = remote_api_roms(provider.uuid, api)?;
        return Ok(ProviderTestResult {
            rom_count: roms.len(),
            message: format!("Remote API returned {} ROM item(s).", roms.len()),
        });
    }

    Err(StorageError::Provider(
        "Provider has no source configured.".to_string(),
    ))
}

pub fn sync_provider(provider: &RomProvider) -> Result<ProviderSyncResult, StorageError> {
    let roms = if let Some(path) = &provider.absolute_local_dir_path {
        local_roms(provider.uuid, path)?
    } else if let Some(url) = &provider.remote_file_url {
        vec![remote_file_rom(provider.uuid, url)?]
    } else if let Some(api) = &provider.remote_api {
        remote_api_roms(provider.uuid, api)?
    } else {
        Vec::new()
    };

    Ok(ProviderSyncResult {
        provider_id: provider.uuid,
        roms,
    })
}

fn local_roms(provider_id: Uuid, path: &Path) -> Result<Vec<RomMetadata>, StorageError> {
    #[cfg(target_os = "android")]
    if let Some(uri) = path.to_str().filter(|path| path.starts_with("content://")) {
        return android_content_uri_roms(provider_id, uri);
    }

    if !path.is_dir() {
        return Err(StorageError::Provider(format!(
            "Local provider could not be read: {}",
            path.display()
        )));
    }

    let mut roms = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if !is_rom_path(&path) {
            continue;
        }
        let bytes = fs::read(&path)?;
        let Some(file_name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        roms.push(RomMetadata {
            id: Some(rom_identifier(&bytes)),
            provider_id,
            file_name: file_name.clone(),
            friendly_name: Some(file_stem_label(&file_name)),
            author: None,
            license: None,
            remote_provider_id: None,
        });
    }

    Ok(roms)
}

#[cfg(target_os = "android")]
fn android_content_uri_roms(
    provider_id: Uuid,
    uri: &str,
) -> Result<Vec<RomMetadata>, StorageError> {
    let files = crate::platform::read_android_local_directory_roms(uri).map_err(|error| {
        StorageError::Provider(format!("Local provider could not be read: {uri}: {error}"))
    })?;

    Ok(files
        .into_iter()
        .filter(|file| is_rom_file_name(&file.file_name))
        .map(|file| RomMetadata {
            id: Some(rom_identifier(&file.bytes)),
            provider_id,
            file_name: file.file_name.clone(),
            friendly_name: Some(file_stem_label(&file.file_name)),
            author: None,
            license: None,
            remote_provider_id: None,
        })
        .collect())
}

fn remote_file_rom(provider_id: Uuid, url: &str) -> Result<RomMetadata, StorageError> {
    http_get_bytes(url).map_err(|error| {
        StorageError::Provider(format!("Remote file could not be read: {error}"))
    })?;

    let file_name = url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("remote.gb")
        .to_string();

    Ok(RomMetadata {
        id: None,
        provider_id,
        friendly_name: Some(file_stem_label(&file_name)),
        file_name,
        author: None,
        license: None,
        remote_provider_id: None,
    })
}

fn remote_api_roms(
    provider_id: Uuid,
    api: &RemoteApiProvider,
) -> Result<Vec<RomMetadata>, StorageError> {
    let first_page = fetch_json(&api.get_url)?;
    let mut roms = remote_api_page_roms(provider_id, api, &first_page)?;
    if let Some(pagination) = &api.pagination {
        let page_count = jsonpath_first_string(&first_page, &pagination.page_count_json_path)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1)
            .max(1);
        let last_page = pagination_last_page(page_count, pagination.max_pages);
        for page in 2..=last_page {
            let url = append_query_param(&api.get_url, &pagination.query_page, page);
            let page_json = fetch_json(&url)?;
            roms.extend(remote_api_page_roms(provider_id, api, &page_json)?);
        }
    }
    Ok(roms)
}

fn pagination_last_page(page_count: usize, max_pages: Option<usize>) -> usize {
    max_pages
        .filter(|max_pages| *max_pages > 0)
        .unwrap_or(page_count)
        .min(page_count)
        .max(1)
}

fn remote_api_page_roms(
    provider_id: Uuid,
    api: &RemoteApiProvider,
    json: &Value,
) -> Result<Vec<RomMetadata>, StorageError> {
    let items = jsonpath_values(json, &api.response_items.items_json_path)?;
    let mut roms = Vec::new();
    for item in expand_arrays(items) {
        let item_id =
            jsonpath_first_string(item, &item_path(&api.response_items.item_id_json_path));
        let file_name = jsonpath_first_string(
            item,
            &item_path(&api.response_items.item_filename_json_path),
        )
        .ok_or_else(|| {
            StorageError::Provider("Remote API item did not include a file name.".to_string())
        })?;
        let friendly_name = api
            .response_items
            .item_name_json_path
            .as_ref()
            .and_then(|path| jsonpath_first_string(item, &item_path(path)));
        let author = api
            .response_items
            .item_author_json_path
            .as_ref()
            .and_then(|path| jsonpath_first_string(item, &item_path(path)));
        let license = api
            .response_items
            .item_license_json_path
            .as_ref()
            .and_then(|path| jsonpath_first_string(item, &item_path(path)));

        roms.push(RomMetadata {
            id: None,
            provider_id,
            file_name,
            friendly_name,
            author,
            license,
            remote_provider_id: item_id,
        });
    }
    Ok(roms)
}

fn expand_arrays(values: Vec<&Value>) -> Vec<&Value> {
    values
        .into_iter()
        .flat_map(|value| match value {
            Value::Array(items) => items.iter().collect::<Vec<_>>(),
            _ => vec![value],
        })
        .collect()
}

fn fetch_json(url: &str) -> Result<Value, StorageError> {
    let text = String::from_utf8(http_get_bytes(url)?).map_err(|error| {
        StorageError::Provider(format!("Remote API response was not UTF-8: {error}"))
    })?;
    serde_json::from_str::<Value>(&text)
        .map_err(|error| StorageError::Provider(format!("Remote API did not return JSON: {error}")))
}

fn jsonpath_values<'a>(json: &'a Value, path: &str) -> Result<Vec<&'a Value>, StorageError> {
    jsonpath_lib::select(json, &normalise_jsonpath(path))
        .map_err(|error| StorageError::Provider(format!("JSONPath failed for {path}: {error}")))
}

fn jsonpath_first_string(json: &Value, path: &str) -> Option<String> {
    jsonpath_values(json, path)
        .ok()
        .and_then(|values| values.into_iter().next().and_then(value_to_string))
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn item_path(path: &str) -> String {
    let path = path.trim();
    normalise_jsonpath(path)
}

fn normalise_jsonpath(path: &str) -> String {
    let path = path.trim();
    if path.is_empty() {
        "$".to_string()
    } else if path.starts_with('$') {
        path.to_string()
    } else if path.starts_with('.') || path.starts_with('[') {
        format!("${path}")
    } else {
        format!("$.{path}")
    }
}

fn append_query_param(url: &str, key: &str, value: usize) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}{key}={value}")
}

fn is_rom_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some_and(is_rom_file_name)
}

fn is_rom_file_name(file_name: &str) -> bool {
    Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "gb" | "gbc"))
}

fn file_stem_label(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_name)
        .to_string()
}

pub(crate) fn http_get_bytes(url: &str) -> Result<Vec<u8>, StorageError> {
    let mut response = HTTP_AGENT
        .get(url)
        .call()
        .map_err(|error| StorageError::Provider(http_error_message(url, error)))?;
    let mut reader = response.body_mut().with_config().reader();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn http_error_message(url: &str, error: ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(code) => format!("{url} returned HTTP {code}."),
        error => format!("{url} could not be reached: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_last_page_uses_page_count_without_limit() {
        assert_eq!(pagination_last_page(12, None), 12);
    }

    #[test]
    fn pagination_last_page_applies_limit() {
        assert_eq!(pagination_last_page(12, Some(5)), 5);
    }

    #[test]
    fn pagination_last_page_never_exceeds_available_pages() {
        assert_eq!(pagination_last_page(3, Some(5)), 3);
    }
}
