use std::fs;
use std::path::PathBuf;

use bevy::asset::HandleTemplate;
use bevy::math::Rect;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::input::selection::PrimaryInputDevice;
use crate::storage::LocalStorage;
use crate::storage::data::RomMetadata;
use crate::storage::provider_sync::{ProviderSyncTaskResult, ProviderSyncTaskState};
use crate::storage::providers::RomProvider;
use crate::storage::rom_data::{StorageFile, StorageFileKind, format_size, storage_files};
use crate::ui_elements::action_hint::action_hints_with_labels;
use crate::ui_elements::button::button;
use crate::ui_elements::description::description;
use crate::ui_elements::info_message::{InfoMessage, info_message, set_latest_info_message};
use crate::ui_elements::interactions::{
    ActivatedUiElement, DefaultFocusTarget, IgnorePicking, InitialFocus, UI_FOCUS_NONE, UiFocusId,
    UiFocusNav, UiFocusNavIds,
};
use crate::ui_elements::list_view::{ListColumn, ListRow, ListViewConfig, list_view};
use crate::ui_elements::responsive::{
    ResponsiveButtonRow, ResponsiveColumns, ResponsiveFieldRow, ResponsiveLandscapeOnly,
    ResponsivePercentWidth, ResponsivePortraitOnly, ResponsiveScreenPadding,
    UI_PORTRAIT_SCREEN_PADDING,
};
use crate::ui_elements::scroll_view::{ScrollViewConfig, flow_scroll_view, scroll_view};
use crate::ui_elements::settings_header::settings_header;
use crate::ui_elements::styles::{UI_MAX_CONTENT_WIDTH, UI_PANEL_GAP, UI_SCREEN_PADDING};
use crate::ui_elements::theme::{UiThemeImageColor, UiThemeTextColor};

const CONTENT_GAP: f32 = 24.0;
const LEFT_WIDTH_PERCENT: f32 = 48.0;
const RIGHT_WIDTH_PERCENT: f32 = 52.0;
const FIELD_GAP: f32 = 18.0;
const HERO_TEXTURE_SIZE: f32 = 454.0;
const HERO_GRID_UNITS: f32 = 2.0;
const HERO_IMAGE_SIZE: f32 = 184.0;
const STORAGE_HERO_X: f32 = 0.0;
const STORAGE_HERO_Y: f32 = 1.0;

const TARGET_CHECK_UPDATES: u16 = 1;
const TARGET_FILE_LIST: u16 = 2;
const TARGET_DELETE_SRAM: u16 = 3;
const TARGET_DELETE_ALL: u16 = 4;

#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct RomDataEditTarget {
    pub rom_index: Option<usize>,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct DeleteSramButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct DeleteAllFilesButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct CheckUpdatesButton;

pub struct RomDataScenePlugin;

impl Plugin for RomDataScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RomDataEditTarget>()
            .add_systems(OnEnter(AppState::RomData), spawn_rom_data_scene)
            .add_systems(
                Update,
                finish_rom_data_provider_sync.run_if(in_state(AppState::RomData)),
            )
            .add_systems(OnExit(AppState::RomData), reset_rom_data_provider_sync)
            .add_observer(handle_delete_button_activation);
    }
}

fn spawn_rom_data_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    storage: Res<LocalStorage>,
    target: Res<RomDataEditTarget>,
    primary_input: Res<PrimaryInputDevice>,
) {
    let rom = selected_rom(&storage, &target).cloned();
    commands.spawn_scene(rom_data_scene(
        &assets,
        *theme,
        &storage,
        &primary_input,
        rom,
    ));
}

fn handle_delete_button_activation(
    activated: On<Add, ActivatedUiElement>,
    check_updates_buttons: Query<(), With<CheckUpdatesButton>>,
    delete_sram_buttons: Query<(), With<DeleteSramButton>>,
    delete_all_buttons: Query<(), With<DeleteAllFilesButton>>,
    storage: ResMut<LocalStorage>,
    target: Res<RomDataEditTarget>,
    mut sync_state: ResMut<ProviderSyncTaskState>,
    state: Res<State<AppState>>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
) {
    if *state.get() != AppState::RomData {
        return;
    }

    let Some(rom) = selected_rom(&storage, &target) else {
        return;
    };

    if check_updates_buttons.get(activated.entity).is_ok() {
        if sync_state.is_running() {
            set_latest_info_message(&mut messages, "ROM provider sync is already running.");
            return;
        }
        let Some(provider) = provider_for_rom(&storage, rom).cloned() else {
            set_latest_info_message(&mut messages, "ROM provider was not found.");
            return;
        };
        set_latest_info_message(&mut messages, "Checking for ROM updates...");
        sync_state.start_provider_sync(provider);
        return;
    }

    let Some(rom_id) = rom.id.as_deref() else {
        set_latest_info_message(&mut messages, "No storage files exist for this ROM.");
        return;
    };

    if delete_sram_buttons.get(activated.entity).is_ok() {
        let removed_sram = remove_file_if_exists(storage.sram_path(rom_id));
        let removed_oscillator = remove_file_if_exists(storage.oscillator_path(rom_id));
        let removed = removed_sram || removed_oscillator;
        set_latest_info_message(
            &mut messages,
            if removed {
                "SRAM files deleted."
            } else {
                "No SRAM files were found."
            },
        );
    } else if delete_all_buttons.get(activated.entity).is_ok() {
        let rom_dir = storage.rom_dir(rom_id);
        if rom_dir.exists() {
            if let Err(error) = fs::remove_dir_all(&rom_dir) {
                eprintln!("failed to delete ROM storage files: {error}");
                set_latest_info_message(&mut messages, "ROM storage files could not be deleted.");
                return;
            }
        }
        set_latest_info_message(&mut messages, "All ROM storage files deleted.");
    }
}

fn finish_rom_data_provider_sync(
    mut sync_state: ResMut<ProviderSyncTaskState>,
    mut storage: ResMut<LocalStorage>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
) {
    let Some(result) = sync_state.poll() else {
        return;
    };

    match result.result {
        Ok(provider_result) => {
            let count = provider_result.roms.len();
            let failures = storage.apply_provider_sync_result(ProviderSyncTaskResult {
                results: vec![provider_result],
                failures: Vec::new(),
            });
            if failures.is_empty() {
                set_latest_info_message(
                    &mut messages,
                    &format!("ROM provider checked. {count} ROM item(s) found."),
                );
            } else {
                set_latest_info_message(&mut messages, &failures.join(" "));
            }
        }
        Err(error) => {
            set_latest_info_message(&mut messages, &format!("{}: {error}", result.provider_name));
        }
    }
}

fn reset_rom_data_provider_sync(mut sync_state: ResMut<ProviderSyncTaskState>) {
    sync_state.clear();
}

fn rom_data_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    storage: &LocalStorage,
    primary_input: &PrimaryInputDevice,
    rom: Option<RomMetadata>,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let body_font = font.clone();
    let landscape_font = font.clone();
    let body_heroes = assets.heroes.clone();
    let landscape_heroes = assets.heroes.clone();
    let files = rom
        .as_ref()
        .and_then(|rom| rom.id.as_deref().map(|id| (rom, id)))
        .map(|(rom, id)| storage_files(&storage.rom_dir(id), &rom.file_name))
        .unwrap_or_default();
    let left_files = files.clone();
    let landscape_left_files = files.clone();
    let landscape_files = files.clone();
    let landscape_rom = rom.clone();

    bsn! {
        DespawnOnExit::<AppState>(AppState::RomData)
        Node {
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(UI_SCREEN_PADDING)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        ResponsiveScreenPadding { landscape: UI_SCREEN_PADDING, portrait: UI_PORTRAIT_SCREEN_PADDING }
        Children [
            (
                Node {
                    width: percent(100),
                    max_width: px(UI_MAX_CONTENT_WIDTH),
                    height: percent(100),
                    min_height: px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(CONTENT_GAP),
                }
                Children [
                    settings_header(font.clone(), assets.icons.clone(), theme, "ROM Storage Details"),
                    (
                        Node {
                            width: percent(100),
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            min_height: px(0.0),
                            display: Display::None,
                        }
                        ResponsiveLandscapeOnly
                        Children [
                            rom_data_landscape_body(landscape_font, landscape_heroes, theme, storage, landscape_rom, landscape_left_files, landscape_files),
                        ]
                    ),
                    (
                        Node {
                            width: percent(100),
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            min_height: px(0.0),
                            display: Display::None,
                        }
                        ResponsivePortraitOnly
                        Children [
                            (
                                #RomDataBodyScrollBar
                                flow_scroll_view(
                                    theme,
                                    #RomDataBodyScrollBar,
                                    ScrollViewConfig {
                                        width: percent(100),
                                        min_height: px(0.0),
                                        thumb_height: 112.0,
                                    },
                                    move |_| rom_data_body(body_font, body_heroes, theme, storage, rom, left_files, files)
                                )
                            )
                        ]
                    ),
                    info_message(font.clone(), theme, "", false),
                    action_hints_with_labels(font, assets.icons.clone(), theme, storage, primary_input, "Back", "Select"),
                ]
            )
        ]
    }
}

fn rom_data_body(
    font: Handle<Font>,
    heroes: Handle<Image>,
    theme: ActiveTheme,
    storage: &LocalStorage,
    rom: Option<RomMetadata>,
    left_files: Vec<StorageFile>,
    files: Vec<StorageFile>,
) -> impl Scene {
    let left_font = font.clone();
    let right_font = font;

    bsn! {
        Node {
            width: percent(100),
            min_height: px(0.0),
            flex_direction: FlexDirection::Row,
            column_gap: px(UI_PANEL_GAP),
            padding: UiRect::right(px(18.0)),
        }
        ResponsiveColumns { gap: UI_PANEL_GAP }
        Children [
            left_panel(left_font, heroes, theme, storage, rom, left_files),
            right_panel(right_font, theme, files),
        ]
    }
}

fn rom_data_landscape_body(
    font: Handle<Font>,
    heroes: Handle<Image>,
    theme: ActiveTheme,
    storage: &LocalStorage,
    rom: Option<RomMetadata>,
    left_files: Vec<StorageFile>,
    files: Vec<StorageFile>,
) -> impl Scene {
    let left_font = font.clone();
    let right_font = font;
    let left_heroes = heroes;

    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            min_height: px(0.0),
            flex_direction: FlexDirection::Row,
            column_gap: px(UI_PANEL_GAP),
        }
        Children [
            (
                #RomDataLeftScrollBar
                scroll_view(
                    theme,
                    #RomDataLeftScrollBar,
                    ScrollViewConfig {
                        width: percent(LEFT_WIDTH_PERCENT),
                        min_height: px(0.0),
                        thumb_height: 112.0,
                    },
                    move |_| left_panel(left_font, left_heroes, theme, storage, rom, left_files)
                )
            ),
            (
                #RomDataRightScrollBar
                scroll_view(
                    theme,
                    #RomDataRightScrollBar,
                    ScrollViewConfig {
                        width: percent(RIGHT_WIDTH_PERCENT),
                        min_height: px(0.0),
                        thumb_height: 112.0,
                    },
                    move |_| right_panel(right_font, theme, files)
                )
            ),
        ]
    }
}

fn left_panel(
    font: Handle<Font>,
    heroes: Handle<Image>,
    theme: ActiveTheme,
    storage: &LocalStorage,
    rom: Option<RomMetadata>,
    files: Vec<StorageFile>,
) -> impl Scene {
    let storage_location = rom
        .as_ref()
        .and_then(|rom| rom.id.as_deref())
        .map(|id| storage.rom_dir(id).display().to_string())
        .unwrap_or_else(|| "(No storage directory)".to_string());
    let rom_name = rom
        .as_ref()
        .map(rom_display_name)
        .unwrap_or_else(|| "(No ROM selected)".to_string());
    let rom_size = files
        .iter()
        .find(|file| file.kind == StorageFileKind::DownloadedRom)
        .map(|file| format_size(file.bytes))
        .unwrap_or_else(|| "(Not downloaded)".to_string());
    let remote_origin = rom
        .as_ref()
        .and_then(|rom| {
            provider_for_rom(storage, rom)
                .map(|provider| provider.remote_file_url.is_some() || provider.remote_api.is_some())
        })
        .unwrap_or(false);

    bsn! {
        Node {
            width: percent(LEFT_WIDTH_PERCENT),
            min_height: px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(FIELD_GAP),
        }
        ResponsivePercentWidth { landscape: LEFT_WIDTH_PERCENT }
        Children [
            description(font.clone(), theme, "Storage Location:"),
            description(font.clone(), theme, storage_location),
            (
                Node {
                    width: percent(100),
                    height: px(8.0),
                }
            ),
            description(font.clone(), theme, "Overview"),
            detail_row(font.clone(), theme, "ROM Name:", rom_name),
            detail_row(font.clone(), theme, "ROM Size:", rom_size),
            detail_row(font.clone(), theme, "Downloaded from remote origin:", remote_origin.to_string()),
            (
                Node {
                    width: percent(100),
                    justify_content: JustifyContent::FlexEnd,
                    padding: UiRect::right(px(22.0)),
                }
                Children [
                    (
                        button(font.clone(), "Check for Updates", theme, UiFocusNav::default())
                        CheckUpdatesButton
                        UiFocusId { id: TARGET_CHECK_UPDATES }
                        UiFocusNavIds { up: UI_FOCUS_NONE, right: TARGET_FILE_LIST, down: UI_FOCUS_NONE, left: UI_FOCUS_NONE }
                        InitialFocus { enabled: true }
                        DefaultFocusTarget
                    ),
                ]
            ),
            storage_hero_image(heroes, theme),
        ]
    }
}

fn right_panel(font: Handle<Font>, theme: ActiveTheme, files: Vec<StorageFile>) -> impl Scene {
    bsn! {
        Node {
            width: percent(RIGHT_WIDTH_PERCENT),
            min_height: px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(20.0),
        }
        ResponsivePercentWidth { landscape: RIGHT_WIDTH_PERCENT }
        Children [
            description(font.clone(), theme, "All Files"),
            (
                list_view(font.clone(), theme, file_list_config(&files))
                UiFocusId { id: TARGET_FILE_LIST }
                UiFocusNavIds { up: UI_FOCUS_NONE, right: UI_FOCUS_NONE, down: TARGET_DELETE_SRAM, left: TARGET_CHECK_UPDATES }
            ),
            (
                Node {
                    width: percent(100),
                    justify_content: JustifyContent::FlexEnd,
                    column_gap: px(22.0),
                }
                ResponsiveButtonRow { gap: 22.0 }
                Children [
                    (
                        button(font.clone(), "Delete SRAM", theme, UiFocusNav::default())
                        DeleteSramButton
                        UiFocusId { id: TARGET_DELETE_SRAM }
                        UiFocusNavIds { up: TARGET_FILE_LIST, right: TARGET_DELETE_ALL, down: UI_FOCUS_NONE, left: TARGET_CHECK_UPDATES }
                    ),
                    (
                        button(font, "Delete All Files", theme, UiFocusNav::default())
                        DeleteAllFilesButton
                        UiFocusId { id: TARGET_DELETE_ALL }
                        UiFocusNavIds { up: TARGET_FILE_LIST, right: UI_FOCUS_NONE, down: UI_FOCUS_NONE, left: TARGET_DELETE_SRAM }
                    ),
                ]
            ),
        ]
    }
}

fn detail_row(
    font: Handle<Font>,
    theme: ActiveTheme,
    label: &'static str,
    value: String,
) -> impl Scene {
    let value_font = font.clone();
    bsn! {
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(18.0),
        }
        ResponsiveFieldRow { gap: 18.0 }
        Children [
            description(font.clone(), theme, label),
            (
                Text({value})
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(value_font)),
                    font_size: px(20.0),
                }
                TextColor({theme.primary})
                UiThemeTextColor::Primary
                IgnorePicking
            )
        ]
    }
}

fn storage_hero_image(image: Handle<Image>, theme: ActiveTheme) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(HERO_IMAGE_SIZE),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        IgnorePicking
        Children [
            (
                Node {
                    width: px(HERO_IMAGE_SIZE),
                    height: px(HERO_IMAGE_SIZE),
                }
                ImageNode {
                    image: {image},
                    color: {theme.primary},
                    rect: {Some(hero_grid_rect(STORAGE_HERO_X, STORAGE_HERO_Y))},
                }
                UiThemeImageColor::Primary
                IgnorePicking
            )
        ]
    }
}

fn hero_grid_rect(x: f32, y: f32) -> Rect {
    let unit = HERO_TEXTURE_SIZE / HERO_GRID_UNITS;
    Rect {
        min: Vec2::new(x * unit, y * unit),
        max: Vec2::new((x + 1.0) * unit, (y + 1.0) * unit),
    }
}

fn file_list_config(files: &[StorageFile]) -> ListViewConfig {
    ListViewConfig {
        nav: UiFocusNav::default(),
        scrollbar_nav: UiFocusNav::default(),
        columns: vec![
            ListColumn {
                heading: "Type",
                width_percent: 34.0,
            },
            ListColumn {
                heading: "Filename",
                width_percent: 42.0,
            },
            ListColumn {
                heading: "Size",
                width_percent: 24.0,
            },
        ],
        rows: files.iter().map(file_row).collect(),
        virtual_total_rows: None,
    }
}

fn file_row(file: &StorageFile) -> ListRow {
    ListRow {
        cells: vec![
            file.kind.label(&file.name),
            file.name.clone(),
            format_size(file.bytes),
        ],
        nav: UiFocusNav::default(),
    }
}

fn remove_file_if_exists(path: PathBuf) -> bool {
    if !path.exists() {
        return false;
    }
    match fs::remove_file(&path) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("failed to delete storage file {}: {error}", path.display());
            false
        }
    }
}

fn selected_rom<'a>(
    storage: &'a LocalStorage,
    target: &RomDataEditTarget,
) -> Option<&'a RomMetadata> {
    target
        .rom_index
        .and_then(|index| storage.data.roms.get(index))
        .or_else(|| storage.data.roms.first())
}

fn rom_display_name(rom: &RomMetadata) -> String {
    rom.friendly_name
        .clone()
        .unwrap_or_else(|| rom.file_name.clone())
}

fn provider_for_rom<'a>(storage: &'a LocalStorage, rom: &RomMetadata) -> Option<&'a RomProvider> {
    storage
        .data
        .providers
        .iter()
        .find(|provider| provider.uuid == rom.provider_id)
}
