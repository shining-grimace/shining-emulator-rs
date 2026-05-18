use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

pub(super) fn default_rom_providers() -> Vec<RomProvider> {
    vec![
        gbdev_provider(
            GBDEV_GB_PROVIDER_UUID,
            "Homebrew Hub GB",
            "https://gbdev.io/api/homebrew/gb",
        ),
        gbdev_provider(
            GBDEV_GBC_PROVIDER_UUID,
            "Homebrew Hub GBC",
            "https://gbdev.io/api/homebrew/gbc",
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
            pagination: None,
            response_items: RemoteApiResponseItems {
                items_json_path: "$".to_string(),
                item_id_json_path: "id".to_string(),
                item_name_json_path: Some("name".to_string()),
                item_author_json_path: Some("author".to_string()),
                item_license_json_path: Some("license".to_string()),
                item_filename_json_path: "filename".to_string(),
            },
            download_url: "https://gbdev.io/{filename}".to_string(),
        }),
    }
}
