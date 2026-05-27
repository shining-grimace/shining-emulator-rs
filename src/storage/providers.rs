use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage::errors::StorageError;

const GBDEV_GB_PROVIDER_UUID: Uuid = Uuid::from_u128(0x1d077a21_b17d_458c_bbbf_410a1d49240a);
const GBDEV_GBC_PROVIDER_UUID: Uuid = Uuid::from_u128(0x6ef907ab_d667_45d2_9c12_712a3910d134);
const VALIDATION_PROVIDER_UUID: Uuid = Uuid::from_u128(0x73b51ffd_4149_45d0_b820_a56c64dc9ff0);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RomProvider {
    pub uuid: Uuid,
    pub friendly_name: String,
    pub priority: u8,
    pub enabled: bool,
    pub locked: bool,
    pub last_fetched: Option<u64>,
    pub absolute_local_dir_path: Option<PathBuf>,
    pub remote_file_url: Option<String>,
    pub remote_api: Option<RemoteApiProvider>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteApiProvider {
    pub get_url: String,
    pub pagination: Option<RemoteApiPagination>,
    pub response_items: RemoteApiResponseItems,
    pub download_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteApiPagination {
    pub page_count_json_path: String,
    pub query_page: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_pages: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteApiResponseItems {
    pub items_json_path: String,
    pub item_id_json_path: String,
    pub item_name_json_path: Option<String>,
    pub item_author_json_path: Option<String>,
    pub item_license_json_path: Option<String>,
    pub item_filename_json_path: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RomProviderSourceKind {
    LocalDirectory,
    RemoteFile,
    RemoteApi,
}

#[derive(Clone, Debug)]
pub struct RomProviderFormInput {
    pub friendly_name: String,
    pub priority: String,
    pub enabled: bool,
    pub source_kind: RomProviderSourceKind,
    pub local_dir_path: String,
    pub remote_file_url: String,
    pub api_url: String,
    pub download_url: String,
    pub items_json_path: String,
    pub pagination_enabled: bool,
    pub page_count_json_path: String,
    pub query_page: String,
    pub max_pages: String,
    pub item_id_json_path: String,
    pub item_name_json_path: String,
    pub item_author_json_path: String,
    pub item_license_json_path: String,
    pub item_filename_json_path: String,
}

pub fn provider_from_form_input(
    existing: Option<&RomProvider>,
    input: RomProviderFormInput,
) -> Result<RomProvider, StorageError> {
    let friendly_name = required_value(&input.friendly_name, "Display name is required.")?;
    let priority = input
        .priority
        .trim()
        .parse::<u8>()
        .map_err(|_| provider_error("Priority must be a number from 1 to 5."))?;
    if !(1..=5).contains(&priority) {
        return Err(provider_error("Priority must be a number from 1 to 5."));
    }

    let mut provider = existing.cloned().unwrap_or_else(new_provider);
    provider.friendly_name = friendly_name;
    provider.priority = priority;
    provider.enabled = input.enabled;
    provider.absolute_local_dir_path = None;
    provider.remote_file_url = None;
    provider.remote_api = None;

    match input.source_kind {
        RomProviderSourceKind::LocalDirectory => {
            provider.absolute_local_dir_path = Some(PathBuf::from(required_value(
                &input.local_dir_path,
                "Local directory is required.",
            )?));
        }
        RomProviderSourceKind::RemoteFile => {
            let url = required_url(&input.remote_file_url, "Remote file URL is required.")?;
            provider.remote_file_url = Some(url);
        }
        RomProviderSourceKind::RemoteApi => {
            provider.remote_api = Some(remote_api_from_form_input(&input)?);
        }
    }

    validate_rom_provider(&provider)?;
    Ok(provider)
}

pub fn validate_rom_provider(provider: &RomProvider) -> Result<(), StorageError> {
    if provider.friendly_name.trim().is_empty() {
        return Err(provider_error("Display name is required."));
    }
    if !(1..=5).contains(&provider.priority) {
        return Err(provider_error("Priority must be a number from 1 to 5."));
    }

    let source_count = usize::from(provider.absolute_local_dir_path.is_some())
        + usize::from(provider.remote_file_url.is_some())
        + usize::from(provider.remote_api.is_some());
    if source_count != 1 {
        return Err(provider_error(
            "Exactly one ROM provider source type must be configured.",
        ));
    }

    if let Some(path) = &provider.absolute_local_dir_path {
        if path.as_os_str().is_empty() {
            return Err(provider_error("Local directory is required."));
        }
    }
    if let Some(url) = &provider.remote_file_url {
        validate_http_url(url, "Remote file URL")?;
    }
    if let Some(api) = &provider.remote_api {
        validate_remote_api_provider(api)?;
    }

    Ok(())
}

fn remote_api_from_form_input(
    input: &RomProviderFormInput,
) -> Result<RemoteApiProvider, StorageError> {
    let pagination = if input.pagination_enabled {
        Some(RemoteApiPagination {
            page_count_json_path: required_value(
                &input.page_count_json_path,
                "Pagination count path is required when pagination is enabled.",
            )?,
            query_page: required_value(
                &input.query_page,
                "Pagination page param is required when pagination is enabled.",
            )?,
            max_pages: optional_positive_usize(&input.max_pages, "Max pages")?,
        })
    } else {
        None
    };

    Ok(RemoteApiProvider {
        get_url: required_url(&input.api_url, "API URL is required.")?,
        pagination,
        response_items: RemoteApiResponseItems {
            items_json_path: value_or_default(&input.items_json_path, "$"),
            item_id_json_path: required_value(
                &input.item_id_json_path,
                "ROM item ID path is required.",
            )?,
            item_name_json_path: optional_value(&input.item_name_json_path),
            item_author_json_path: optional_value(&input.item_author_json_path),
            item_license_json_path: optional_value(&input.item_license_json_path),
            item_filename_json_path: required_value(
                &input.item_filename_json_path,
                "ROM file name path is required.",
            )?,
        },
        download_url: required_url(&input.download_url, "Download URL is required.")?,
    })
}

fn validate_remote_api_provider(api: &RemoteApiProvider) -> Result<(), StorageError> {
    validate_http_url(&api.get_url, "API URL")?;
    validate_http_url(&api.download_url, "Download URL")?;
    required_value(
        &api.response_items.items_json_path,
        "ROM items path is required.",
    )?;
    required_value(
        &api.response_items.item_id_json_path,
        "ROM item ID path is required.",
    )?;
    required_value(
        &api.response_items.item_filename_json_path,
        "ROM file name path is required.",
    )?;
    if let Some(pagination) = &api.pagination {
        required_value(
            &pagination.page_count_json_path,
            "Pagination count path is required when pagination is enabled.",
        )?;
        required_value(
            &pagination.query_page,
            "Pagination page param is required when pagination is enabled.",
        )?;
        if pagination.max_pages.is_some_and(|max_pages| max_pages == 0) {
            return Err(provider_error("Max pages must be a positive number."));
        }
    }
    Ok(())
}

fn validate_http_url(value: &str, field_name: &str) -> Result<(), StorageError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(provider_error(format!("{field_name} is required.")));
    }
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return Err(provider_error(format!(
            "{field_name} must start with http:// or https://."
        )));
    }
    Ok(())
}

fn required_url(value: &str, error_message: &str) -> Result<String, StorageError> {
    let value = required_value(value, error_message)?;
    validate_http_url(&value, error_message.trim_end_matches(" is required."))?;
    Ok(value)
}

fn required_value(value: &str, error_message: &str) -> Result<String, StorageError> {
    let value = value.trim();
    if value.is_empty() {
        Err(provider_error(error_message))
    } else {
        Ok(value.to_string())
    }
}

fn optional_value(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn optional_positive_usize(value: &str, field_name: &str) -> Result<Option<usize>, StorageError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }

    let parsed = value
        .parse::<usize>()
        .map_err(|_| provider_error(format!("{field_name} must be a positive number.")))?;
    if parsed == 0 {
        return Err(provider_error(format!(
            "{field_name} must be a positive number."
        )));
    }
    Ok(Some(parsed))
}

fn value_or_default(value: &str, default_value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        default_value.to_string()
    } else {
        value.to_string()
    }
}

fn provider_error(message: impl Into<String>) -> StorageError {
    StorageError::Provider(message.into())
}

pub fn new_provider() -> RomProvider {
    RomProvider {
        uuid: Uuid::new_v4(),
        friendly_name: String::new(),
        priority: 1,
        enabled: true,
        locked: false,
        last_fetched: None,
        absolute_local_dir_path: None,
        remote_file_url: None,
        remote_api: None,
    }
}

pub(super) fn default_rom_providers() -> Vec<RomProvider> {
    vec![
        gbdev_provider(
            GBDEV_GB_PROVIDER_UUID,
            "Homebrew Hub GB",
            "https://hh3.gbdev.io/api/search?platform=GB",
        ),
        gbdev_provider(
            GBDEV_GBC_PROVIDER_UUID,
            "Homebrew Hub GBC",
            "https://hh3.gbdev.io/api/search?platform=GBC",
        ),
        RomProvider {
            uuid: VALIDATION_PROVIDER_UUID,
            friendly_name: "Game Boy validation ROMs".to_string(),
            priority: 5,
            enabled: false,
            locked: true,
            last_fetched: None,
            absolute_local_dir_path: None,
            remote_file_url: None,
            remote_api: None,
        },
    ]
}

pub(super) fn apply_default_provider_updates(providers: &mut [RomProvider]) -> bool {
    let mut changed = false;
    for provider in providers {
        if !matches!(
            provider.uuid,
            GBDEV_GB_PROVIDER_UUID | GBDEV_GBC_PROVIDER_UUID
        ) {
            continue;
        }
        let Some(pagination) = provider
            .remote_api
            .as_mut()
            .and_then(|api| api.pagination.as_mut())
        else {
            continue;
        };
        if pagination.max_pages.is_none() {
            pagination.max_pages = Some(5);
            changed = true;
        }
    }
    changed
}

fn gbdev_provider(uuid: Uuid, friendly_name: &str, get_url: &str) -> RomProvider {
    RomProvider {
        uuid,
        friendly_name: friendly_name.to_string(),
        priority: 3,
        enabled: true,
        locked: true,
        last_fetched: None,
        absolute_local_dir_path: None,
        remote_file_url: None,
        remote_api: Some(RemoteApiProvider {
            get_url: get_url.to_string(),
            pagination: Some(RemoteApiPagination {
                page_count_json_path: "page_total".to_string(),
                query_page: "page".to_string(),
                max_pages: Some(5),
            }),
            response_items: RemoteApiResponseItems {
                items_json_path: "entries[*]".to_string(),
                item_id_json_path: "slug".to_string(),
                item_name_json_path: Some("title".to_string()),
                item_author_json_path: Some("developer".to_string()),
                item_license_json_path: Some("license".to_string()),
                item_filename_json_path: "files[*].filename".to_string(),
            },
            download_url:
                "https://raw.githubusercontent.com/gbdev/database/master/entries/{id}/{filename}"
                    .to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_homebrew_hub_providers_limit_remote_api_pages() {
        let providers = default_rom_providers();

        assert_eq!(providers[0].remote_api_max_pages(), Some(5));
        assert_eq!(providers[1].remote_api_max_pages(), Some(5));
    }

    #[test]
    fn default_provider_updates_fill_missing_homebrew_hub_page_limit() {
        let mut providers = default_rom_providers();
        providers[0]
            .remote_api
            .as_mut()
            .and_then(|api| api.pagination.as_mut())
            .unwrap()
            .max_pages = None;

        assert!(apply_default_provider_updates(&mut providers));
        assert_eq!(providers[0].remote_api_max_pages(), Some(5));
    }

    #[test]
    fn default_provider_updates_do_not_overwrite_existing_page_limit() {
        let mut providers = default_rom_providers();
        providers[0]
            .remote_api
            .as_mut()
            .and_then(|api| api.pagination.as_mut())
            .unwrap()
            .max_pages = Some(7);

        assert!(!apply_default_provider_updates(&mut providers));
        assert_eq!(providers[0].remote_api_max_pages(), Some(7));
    }

    trait RomProviderTestExt {
        fn remote_api_max_pages(&self) -> Option<usize>;
    }

    impl RomProviderTestExt for RomProvider {
        fn remote_api_max_pages(&self) -> Option<usize> {
            self.remote_api
                .as_ref()
                .and_then(|api| api.pagination.as_ref())
                .and_then(|pagination| pagination.max_pages)
        }
    }
}
