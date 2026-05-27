use bevy::prelude::*;
use bevy::ui::UiScale;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::{ActiveTheme, ActiveThemeChanged, active_theme_for_setting};
use crate::app_ui_scale::{UI_SCALE_LABELS, apply_ui_scale_setting};
use crate::input::mappings::RuntimeInputMappings;
use crate::scenes::rom_provider::RomProviderEditTarget;
use crate::storage::LocalStorage;
use crate::storage::data::RomMetadata;
use crate::storage::provider_sync::{ProviderSyncTaskResult, ProviderSyncTaskState};
use crate::storage::providers::RomProvider;
use crate::ui_elements::action_hint::action_hints_with_labels;
use crate::ui_elements::button::button;
use crate::ui_elements::description::description;
use crate::ui_elements::heading::heading;
use crate::ui_elements::info_message::{InfoMessage, info_message_text, set_latest_info_message};
use crate::ui_elements::interactions::{
    ActivatedUiElement, DefaultFocusTarget, DisabledUiElement, InitialFocus, SelectedUiElement,
    UI_FOCUS_NONE, UiElementKind, UiFocusId, UiFocusNav, UiFocusNavIds, UiListCellText,
    UiMultiSelect, UiScrollArea,
};
use crate::ui_elements::list_view::{
    DEFAULT_VIRTUAL_ROW_POOL_SIZE, ListColumn, ListRow, ListViewConfig, VirtualListContent,
    VirtualListRow, VirtualListScrollArea, VirtualListWindow, collect_descendants_with,
    collect_list_item_entities, list_view, set_list_row_cells, virtual_list_content_height,
    virtual_list_rows, virtual_list_window,
};
use crate::ui_elements::multi_select::{MultiSelectConfig, multi_select};
use crate::ui_elements::scroll_view::{ScrollViewConfig, scroll_view};
use crate::ui_elements::styles::{UI_MAX_CONTENT_WIDTH, UI_PANEL_GAP, UI_SCREEN_PADDING};

const SETTINGS_CONTENT_GAP: f32 = 24.0;
const SETTINGS_CONTROL_GAP: f32 = 20.0;
const SETTINGS_RIGHT_SECTION_GAP: f32 = 28.0;
const SETTINGS_BUTTON_ROW_GAP: f32 = 16.0;
const SETTINGS_LEFT_WIDTH_PERCENT: f32 = 48.0;
const SETTINGS_RIGHT_WIDTH_PERCENT: f32 = 52.0;
const SETTINGS_SAVE_ERROR_MESSAGE: &str = "Settings could not be saved";

const FIELD_FORCE_BUTTON_OVERLAY: u8 = 0;
const FIELD_EMULATION_MODEL: u8 = 1;
const FIELD_SGB_OVERLAY_ENABLE: u8 = 2;
const FIELD_UPSCALING_MODE: u8 = 3;
const FIELD_UI_SCALE: u8 = 4;
const FIELD_UI_THEME: u8 = 5;

const TARGET_OVERLAY: u16 = 0;
const TARGET_MODEL: u16 = 1;
const TARGET_SGB: u16 = 2;
const TARGET_UPSCALING: u16 = 3;
const TARGET_UI_SCALE: u16 = 4;
const TARGET_THEME: u16 = 5;
const TARGET_PRIMARY_INPUT: u16 = 6;
const TARGET_EDIT_MAPPINGS: u16 = 7;
const TARGET_AUDIO_PRESET: u16 = 8;
const TARGET_DELETE_MAPPING: u16 = 9;
const TARGET_EDIT_MAPPING: u16 = 10;
const TARGET_CREATE_MAPPING: u16 = 11;
const TARGET_ROM_STORAGE_LIST: u16 = 12;
const TARGET_STORAGE_DELETE: u16 = 13;
const TARGET_STORAGE_DETAILS: u16 = 14;
const TARGET_PROVIDER_LIST: u16 = 15;
const TARGET_PROVIDER_SYNC: u16 = 16;
const TARGET_PROVIDER_DELETE: u16 = 17;
const TARGET_PROVIDER_EDIT: u16 = 18;
const TARGET_PROVIDER_CREATE: u16 = 19;
const ROM_STORAGE_COLUMN_COUNT: usize = 3;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct SettingsSelect {
    field: u8,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct RomStorageListView;

#[derive(Clone, Copy, Component, Debug, Default)]
struct RomStorageRowsBound;

#[derive(Clone, Copy, Component, Debug, Default)]
struct RomStorageScrollArea;

#[derive(Clone, Copy, Component, Debug, Default)]
struct RomStorageContent;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct ProviderList;

#[derive(Clone, Copy, Component, Debug, Default)]
struct ProviderRowsBound;

#[derive(Clone, Copy, Component, Debug)]
struct ProviderRow {
    index: usize,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct ProviderSyncButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct ProviderDeleteButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct ProviderEditButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct ProviderCreateButton;

pub struct SettingsScenePlugin;

impl Plugin for SettingsScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Settings), spawn_settings_scene)
            .add_systems(
                Update,
                (
                    bind_provider_rows,
                    bind_rom_storage_rows,
                    sync_rom_storage_rows,
                    finish_settings_provider_sync,
                )
                    .run_if(in_state(AppState::Settings)),
            )
            .add_systems(OnExit(AppState::Settings), reset_settings_provider_sync)
            .add_observer(save_settings_select_on_activation)
            .add_observer(handle_provider_button_activation);
    }
}

fn spawn_settings_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    input_mappings: Res<RuntimeInputMappings>,
    storage: Res<LocalStorage>,
    sync_state: Res<ProviderSyncTaskState>,
) {
    commands.spawn_scene(settings_scene(
        &assets,
        *theme,
        &input_mappings,
        &storage,
        sync_state.is_running(),
    ));
}

fn save_settings_select_on_activation(
    activated: On<Add, ActivatedUiElement>,
    mut commands: Commands,
    selects: Query<(&SettingsSelect, &UiMultiSelect)>,
    mut storage: ResMut<LocalStorage>,
    state: Res<State<AppState>>,
    mut active_theme: ResMut<ActiveTheme>,
    mut ui_scale: ResMut<UiScale>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
) {
    if *state.get() != AppState::Settings {
        return;
    }

    let Ok((settings_select, ui_select)) = selects.get(activated.entity) else {
        return;
    };

    let value = ui_select.selected as u8;
    let previous_value = match settings_select.field {
        FIELD_FORCE_BUTTON_OVERLAY => storage.data.settings.force_button_overlay,
        FIELD_EMULATION_MODEL => storage.data.settings.emulation_model,
        FIELD_SGB_OVERLAY_ENABLE => storage.data.settings.sgb_overlay_enable,
        FIELD_UPSCALING_MODE => storage.data.settings.upscaling_mode,
        FIELD_UI_SCALE => storage.data.settings.ui_scale,
        FIELD_UI_THEME => storage.data.settings.ui_theme,
        _ => return,
    };
    if previous_value == value {
        return;
    }

    match settings_select.field {
        FIELD_FORCE_BUTTON_OVERLAY => storage.data.settings.force_button_overlay = value,
        FIELD_EMULATION_MODEL => storage.data.settings.emulation_model = value,
        FIELD_SGB_OVERLAY_ENABLE => storage.data.settings.sgb_overlay_enable = value,
        FIELD_UPSCALING_MODE => storage.data.settings.upscaling_mode = value,
        FIELD_UI_SCALE => storage.data.settings.ui_scale = value,
        FIELD_UI_THEME => storage.data.settings.ui_theme = value,
        _ => return,
    }

    if let Err(error) = storage.save_settings() {
        eprintln!("failed to save settings: {error}");
        set_latest_info_message(&mut messages, SETTINGS_SAVE_ERROR_MESSAGE);
        return;
    }

    if settings_select.field == FIELD_UI_THEME {
        *active_theme = active_theme_for_setting(value);
        commands.trigger(ActiveThemeChanged);
    }
    if settings_select.field == FIELD_UI_SCALE {
        apply_ui_scale_setting(value, &mut ui_scale);
    }
}

fn handle_provider_button_activation(
    activated: On<Add, ActivatedUiElement>,
    create_buttons: Query<(), With<ProviderCreateButton>>,
    edit_buttons: Query<(), With<ProviderEditButton>>,
    delete_buttons: Query<(), With<ProviderDeleteButton>>,
    sync_buttons: Query<(), With<ProviderSyncButton>>,
    selected_provider_rows: Query<&ProviderRow, With<SelectedUiElement>>,
    mut edit_target: ResMut<RomProviderEditTarget>,
    mut storage: ResMut<LocalStorage>,
    mut sync_state: ResMut<ProviderSyncTaskState>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
) {
    if *state.get() != AppState::Settings {
        return;
    }

    let entity = activated.entity;
    if create_buttons.get(entity).is_ok() {
        edit_target.provider_index = None;
        next_state.set(AppState::RomProvider);
    } else if edit_buttons.get(entity).is_ok() {
        let Some(index) = selected_provider_index(&selected_provider_rows) else {
            set_latest_info_message(&mut messages, "Select a ROM provider to edit.");
            return;
        };
        edit_target.provider_index = Some(index);
        next_state.set(AppState::RomProvider);
    } else if delete_buttons.get(entity).is_ok() {
        let Some(index) = selected_provider_index(&selected_provider_rows) else {
            set_latest_info_message(&mut messages, "Select a ROM provider to delete.");
            return;
        };
        if storage
            .data
            .providers
            .get(index)
            .is_some_and(|provider| provider.locked)
        {
            set_latest_info_message(&mut messages, "Built-in ROM providers cannot be deleted.");
            return;
        }
        if index < storage.data.providers.len() {
            let provider_id = storage.data.providers[index].uuid;
            storage.data.providers.remove(index);
            storage
                .data
                .roms
                .retain(|rom| rom.provider_id != provider_id);
            if let Err(error) = storage.save_providers().and_then(|_| storage.save_roms()) {
                eprintln!("failed to save ROM providers: {error}");
                set_latest_info_message(&mut messages, "ROM provider could not be deleted.");
            } else {
                set_latest_info_message(&mut messages, "ROM provider deleted.");
            }
        }
    } else if sync_buttons.get(entity).is_ok() {
        let Some(index) = selected_provider_index(&selected_provider_rows) else {
            set_latest_info_message(&mut messages, "Select a ROM provider to sync.");
            return;
        };
        if sync_state.is_running() {
            set_latest_info_message(&mut messages, "ROM provider sync is already running.");
            return;
        }

        let Some(provider) = storage.data.providers.get(index).cloned() else {
            set_latest_info_message(&mut messages, "Selected ROM provider was not found.");
            return;
        };
        set_latest_info_message(&mut messages, "Syncing ROM provider...");
        sync_state.start_provider_sync(provider);
    }
}

fn finish_settings_provider_sync(
    mut sync_state: ResMut<ProviderSyncTaskState>,
    mut storage: ResMut<LocalStorage>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
) {
    let result = sync_state.poll();
    let Some(result) = result else {
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
                    &format!("Provider synced. {count} ROM item(s) found."),
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

fn reset_settings_provider_sync(mut sync_state: ResMut<ProviderSyncTaskState>) {
    sync_state.clear();
}

fn selected_provider_index(
    selected_provider_rows: &Query<&ProviderRow, With<SelectedUiElement>>,
) -> Option<usize> {
    selected_provider_rows.iter().next().map(|row| row.index)
}

fn bind_provider_rows(
    mut commands: Commands,
    lists: Query<(Entity, &Children), (With<ProviderList>, Without<ProviderRowsBound>)>,
    kinds: Query<&UiElementKind>,
    child_query: Query<&Children>,
) {
    for (list_entity, children) in &lists {
        let rows = collect_list_item_entities(children, &kinds, &child_query);
        for (index, row) in rows.into_iter().enumerate() {
            commands.entity(row).insert(ProviderRow { index });
        }
        commands.entity(list_entity).insert(ProviderRowsBound);
    }
}

fn bind_rom_storage_rows(
    mut commands: Commands,
    lists: Query<(Entity, &Children), (With<RomStorageListView>, Without<RomStorageRowsBound>)>,
    kinds: Query<&UiElementKind>,
    scroll_areas: Query<(), With<VirtualListScrollArea>>,
    scroll_contents: Query<(), With<VirtualListContent>>,
    child_query: Query<&Children>,
) {
    for (list_entity, children) in &lists {
        let rows = collect_list_item_entities(children, &kinds, &child_query);
        for (slot, row) in rows.into_iter().enumerate() {
            commands.entity(row).insert(VirtualListRow {
                slot,
                item_index: usize::MAX,
            });
        }
        for scroll_area in collect_descendants_with(children, &scroll_areas, &child_query) {
            commands.entity(scroll_area).insert(RomStorageScrollArea);
        }
        for scroll_content in collect_descendants_with(children, &scroll_contents, &child_query) {
            commands.entity(scroll_content).insert(RomStorageContent);
        }
        commands.entity(list_entity).insert(RomStorageRowsBound);
    }
}

fn sync_rom_storage_rows(
    storage: Res<LocalStorage>,
    scroll_areas: Query<&UiScrollArea, With<RomStorageScrollArea>>,
    mut virtual_contents: Query<&mut Node, With<RomStorageContent>>,
    mut rows: Query<(&mut VirtualListRow, &Children)>,
    mut cells: Query<(&mut UiListCellText, &Children)>,
    mut texts: Query<&mut Text>,
    child_query: Query<&Children>,
) {
    let window = scroll_areas
        .iter()
        .next()
        .map(virtual_list_window)
        .unwrap_or(VirtualListWindow {
            first_row: 0,
            content_offset: 0.0,
        });
    for mut node in &mut virtual_contents {
        node.top = px(-window.content_offset);
        node.height = px(virtual_list_content_height(
            storage.data.roms.len(),
            DEFAULT_VIRTUAL_ROW_POOL_SIZE,
        ));
    }
    for (mut row, children) in &mut rows {
        let rom_index = window.first_row + row.slot;
        let Some(rom) = storage.data.roms.get(rom_index) else {
            if row.item_index != usize::MAX {
                row.item_index = usize::MAX;
                set_list_row_cells(
                    &["", "", ""],
                    children,
                    &mut cells,
                    &mut texts,
                    &child_query,
                );
            }
            continue;
        };
        if row.item_index == rom_index {
            continue;
        }
        row.item_index = rom_index;
        update_rom_storage_row(rom, children, &mut cells, &mut texts, &child_query);
    }
}

fn settings_focus_nav(id: u16) -> UiFocusNavIds {
    match id {
        TARGET_OVERLAY => focus_nav_ids(
            UI_FOCUS_NONE,
            TARGET_ROM_STORAGE_LIST,
            TARGET_MODEL,
            UI_FOCUS_NONE,
        ),
        TARGET_MODEL => focus_nav_ids(
            TARGET_OVERLAY,
            TARGET_ROM_STORAGE_LIST,
            TARGET_SGB,
            UI_FOCUS_NONE,
        ),
        TARGET_SGB => focus_nav_ids(
            TARGET_MODEL,
            TARGET_ROM_STORAGE_LIST,
            TARGET_UPSCALING,
            UI_FOCUS_NONE,
        ),
        TARGET_UPSCALING => focus_nav_ids(
            TARGET_SGB,
            TARGET_ROM_STORAGE_LIST,
            TARGET_UI_SCALE,
            UI_FOCUS_NONE,
        ),
        TARGET_UI_SCALE => focus_nav_ids(
            TARGET_UPSCALING,
            TARGET_ROM_STORAGE_LIST,
            TARGET_THEME,
            UI_FOCUS_NONE,
        ),
        TARGET_THEME => focus_nav_ids(
            TARGET_UI_SCALE,
            TARGET_ROM_STORAGE_LIST,
            TARGET_PRIMARY_INPUT,
            UI_FOCUS_NONE,
        ),
        TARGET_PRIMARY_INPUT => focus_nav_ids(
            TARGET_THEME,
            TARGET_ROM_STORAGE_LIST,
            TARGET_EDIT_MAPPINGS,
            UI_FOCUS_NONE,
        ),
        TARGET_EDIT_MAPPINGS => focus_nav_ids(
            TARGET_PRIMARY_INPUT,
            TARGET_ROM_STORAGE_LIST,
            TARGET_AUDIO_PRESET,
            UI_FOCUS_NONE,
        ),
        TARGET_AUDIO_PRESET => focus_nav_ids(
            TARGET_EDIT_MAPPINGS,
            TARGET_ROM_STORAGE_LIST,
            TARGET_DELETE_MAPPING,
            UI_FOCUS_NONE,
        ),
        TARGET_DELETE_MAPPING => focus_nav_ids(
            TARGET_AUDIO_PRESET,
            TARGET_EDIT_MAPPING,
            UI_FOCUS_NONE,
            UI_FOCUS_NONE,
        ),
        TARGET_EDIT_MAPPING => focus_nav_ids(
            TARGET_AUDIO_PRESET,
            TARGET_CREATE_MAPPING,
            UI_FOCUS_NONE,
            TARGET_DELETE_MAPPING,
        ),
        TARGET_CREATE_MAPPING => focus_nav_ids(
            TARGET_AUDIO_PRESET,
            TARGET_ROM_STORAGE_LIST,
            UI_FOCUS_NONE,
            TARGET_EDIT_MAPPING,
        ),
        TARGET_ROM_STORAGE_LIST => focus_nav_ids(
            UI_FOCUS_NONE,
            UI_FOCUS_NONE,
            TARGET_STORAGE_DELETE,
            TARGET_OVERLAY,
        ),
        TARGET_STORAGE_DELETE => focus_nav_ids(
            TARGET_ROM_STORAGE_LIST,
            TARGET_STORAGE_DETAILS,
            TARGET_PROVIDER_LIST,
            TARGET_OVERLAY,
        ),
        TARGET_STORAGE_DETAILS => focus_nav_ids(
            TARGET_ROM_STORAGE_LIST,
            UI_FOCUS_NONE,
            TARGET_PROVIDER_LIST,
            TARGET_STORAGE_DELETE,
        ),
        TARGET_PROVIDER_LIST => focus_nav_ids(
            TARGET_STORAGE_DELETE,
            UI_FOCUS_NONE,
            TARGET_PROVIDER_SYNC,
            TARGET_OVERLAY,
        ),
        TARGET_PROVIDER_SYNC => focus_nav_ids(
            TARGET_PROVIDER_LIST,
            TARGET_PROVIDER_DELETE,
            UI_FOCUS_NONE,
            TARGET_OVERLAY,
        ),
        TARGET_PROVIDER_DELETE => focus_nav_ids(
            TARGET_PROVIDER_LIST,
            TARGET_PROVIDER_EDIT,
            UI_FOCUS_NONE,
            TARGET_PROVIDER_SYNC,
        ),
        TARGET_PROVIDER_EDIT => focus_nav_ids(
            TARGET_PROVIDER_LIST,
            TARGET_PROVIDER_CREATE,
            UI_FOCUS_NONE,
            TARGET_PROVIDER_DELETE,
        ),
        TARGET_PROVIDER_CREATE => focus_nav_ids(
            TARGET_PROVIDER_LIST,
            UI_FOCUS_NONE,
            UI_FOCUS_NONE,
            TARGET_PROVIDER_EDIT,
        ),
        _ => UiFocusNavIds {
            up: UI_FOCUS_NONE,
            right: UI_FOCUS_NONE,
            down: UI_FOCUS_NONE,
            left: UI_FOCUS_NONE,
        },
    }
}

fn focus_nav_ids(up: u16, right: u16, down: u16, left: u16) -> UiFocusNavIds {
    UiFocusNavIds {
        up,
        right,
        down,
        left,
    }
}

fn settings_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    input_mappings: &RuntimeInputMappings,
    storage: &LocalStorage,
    sync_running: bool,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let left_column_font = font.clone();
    let settings = storage.data.settings;
    let providers = storage.data.providers.clone();
    let roms = storage.data.roms.clone();
    let info_text = if sync_running {
        "Syncing ROM provider..."
    } else {
        ""
    };

    bsn! {
        #SettingsScene
        DespawnOnExit::<AppState>(AppState::Settings)
        Node {
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(UI_SCREEN_PADDING)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Children [
            (
                Node {
                    width: percent(100),
                    max_width: px(UI_MAX_CONTENT_WIDTH),
                    height: percent(100),
                    min_height: px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(SETTINGS_CONTENT_GAP),
                }
                Children [
                    heading(font.clone(), theme, "Settings"),
                    (
                        Node {
                            width: percent(100),
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            min_height: px(0.0),
                            flex_direction: FlexDirection::Row,
                            column_gap: px(UI_PANEL_GAP),
                        }
                        Children [
                            (
                                #LeftScrollBar
                                scroll_view(
                                    theme,
                                    #LeftScrollBar,
                                    ScrollViewConfig {
                                        width: percent(SETTINGS_LEFT_WIDTH_PERCENT),
                                        min_height: px(0.0),
                                        thumb_height: 112.0,
                                    },
                                    move |_| settings_left_column(left_column_font, theme, settings)
                                )
                            ),
                            (
                                Node {
                                    width: percent(SETTINGS_RIGHT_WIDTH_PERCENT),
                                    min_height: px(0.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(SETTINGS_RIGHT_SECTION_GAP),
                                }
                                Children [
                                    (
                                        #RomStorageList
                                        Node {
                                            width: percent(100),
                                            min_height: px(0.0),
                                            flex_direction: FlexDirection::Column,
                                            row_gap: px(14.0),
                                        }
                                        Children [
                                            description(font.clone(), theme, "ROM Storage"),
                                            (
                                                #RomStorageListView
                                                list_view(font.clone(), theme, rom_storage_list_config(&roms))
                                                RomStorageListView
                                                UiFocusId { id: TARGET_ROM_STORAGE_LIST }
                                                UiFocusNavIds { up: {settings_focus_nav(TARGET_ROM_STORAGE_LIST).up}, right: {settings_focus_nav(TARGET_ROM_STORAGE_LIST).right}, down: {settings_focus_nav(TARGET_ROM_STORAGE_LIST).down}, left: {settings_focus_nav(TARGET_ROM_STORAGE_LIST).left} }
                                            ),
                                            (
                                                Node {
                                                    width: percent(100),
                                                    justify_content: JustifyContent::FlexEnd,
                                                    column_gap: px(SETTINGS_BUTTON_ROW_GAP),
                                                }
                                                Children [
                                                    (
                                                        #StorageDelete
                                                        button(font.clone(), "Delete", theme, UiFocusNav::default())
                                                        UiFocusId { id: TARGET_STORAGE_DELETE }
                                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_STORAGE_DELETE).up}, right: {settings_focus_nav(TARGET_STORAGE_DELETE).right}, down: {settings_focus_nav(TARGET_STORAGE_DELETE).down}, left: {settings_focus_nav(TARGET_STORAGE_DELETE).left} }
                                                    ),
                                                    (
                                                        #StorageDetails
                                                        button(font.clone(), "View Details", theme, UiFocusNav::default())
                                                        UiFocusId { id: TARGET_STORAGE_DETAILS }
                                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_STORAGE_DETAILS).up}, right: {settings_focus_nav(TARGET_STORAGE_DETAILS).right}, down: {settings_focus_nav(TARGET_STORAGE_DETAILS).down}, left: {settings_focus_nav(TARGET_STORAGE_DETAILS).left} }
                                                    ),
                                                ]
                                            ),
                                        ]
                                    ),
                                    (
                                        #ProviderList
                                        Node {
                                            width: percent(100),
                                            min_height: px(0.0),
                                            flex_direction: FlexDirection::Column,
                                            row_gap: px(14.0),
                                        }
                                        Children [
                                            description(font.clone(), theme, "ROM Providers"),
                                            (
                                                #ProviderListView
                                                list_view(font.clone(), theme, provider_list_config(&providers))
                                                ProviderList
                                                UiFocusId { id: TARGET_PROVIDER_LIST }
                                                UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_LIST).up}, right: {settings_focus_nav(TARGET_PROVIDER_LIST).right}, down: {settings_focus_nav(TARGET_PROVIDER_LIST).down}, left: {settings_focus_nav(TARGET_PROVIDER_LIST).left} }
                                            ),
                                            (
                                                Node {
                                                    width: percent(100),
                                                    justify_content: JustifyContent::FlexEnd,
                                                    column_gap: px(SETTINGS_BUTTON_ROW_GAP),
                                                }
                                                Children [
                                                    (
                                                        #ProviderSync
                                                        button(font.clone(), "Sync", theme, UiFocusNav::default())
                                                        ProviderSyncButton
                                                        UiFocusId { id: TARGET_PROVIDER_SYNC }
                                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_SYNC).up}, right: {settings_focus_nav(TARGET_PROVIDER_SYNC).right}, down: {settings_focus_nav(TARGET_PROVIDER_SYNC).down}, left: {settings_focus_nav(TARGET_PROVIDER_SYNC).left} }
                                                    ),
                                                    (
                                                        #ProviderDelete
                                                        button(font.clone(), "Delete", theme, UiFocusNav::default())
                                                        ProviderDeleteButton
                                                        UiFocusId { id: TARGET_PROVIDER_DELETE }
                                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_DELETE).up}, right: {settings_focus_nav(TARGET_PROVIDER_DELETE).right}, down: {settings_focus_nav(TARGET_PROVIDER_DELETE).down}, left: {settings_focus_nav(TARGET_PROVIDER_DELETE).left} }
                                                    ),
                                                    (
                                                        #ProviderEdit
                                                        button(font.clone(), "Edit", theme, UiFocusNav::default())
                                                        ProviderEditButton
                                                        UiFocusId { id: TARGET_PROVIDER_EDIT }
                                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_EDIT).up}, right: {settings_focus_nav(TARGET_PROVIDER_EDIT).right}, down: {settings_focus_nav(TARGET_PROVIDER_EDIT).down}, left: {settings_focus_nav(TARGET_PROVIDER_EDIT).left} }
                                                    ),
                                                    (
                                                        #ProviderCreate
                                                        button(font.clone(), "Create", theme, UiFocusNav::default())
                                                        ProviderCreateButton
                                                        UiFocusId { id: TARGET_PROVIDER_CREATE }
                                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_CREATE).up}, right: {settings_focus_nav(TARGET_PROVIDER_CREATE).right}, down: {settings_focus_nav(TARGET_PROVIDER_CREATE).down}, left: {settings_focus_nav(TARGET_PROVIDER_CREATE).left} }
                                                    ),
                                                ]
                                            ),
                                        ]
                                    ),
                                ]
                            ),
                        ]
                    ),
                    info_message_text(font.clone(), theme, info_text.to_string(), false),
                    action_hints_with_labels(font, assets.icons.clone(), theme, input_mappings, "Back", "Select"),
                ]
            ),
        ]
    }
}

fn settings_left_column(
    font: Handle<Font>,
    theme: ActiveTheme,
    settings: crate::storage::data::GeneralSettings,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(SETTINGS_CONTROL_GAP),
            padding: UiRect::right(px(18.0)),
        }
        Children [
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(18.0),
                }
                Children [
                    description(font.clone(), theme, "Show Button Overlay"),
                    (
                        #OverlaySelect
                        multi_select(font.clone(), theme, button_overlay_config(settings.force_button_overlay as usize))
                        SettingsSelect { field: FIELD_FORCE_BUTTON_OVERLAY }
                        UiFocusId { id: TARGET_OVERLAY }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_OVERLAY).up}, right: {settings_focus_nav(TARGET_OVERLAY).right}, down: {settings_focus_nav(TARGET_OVERLAY).down}, left: {settings_focus_nav(TARGET_OVERLAY).left} }
                        InitialFocus { enabled: true }
                        DefaultFocusTarget
                    ),
                ]
            ),
            description(font.clone(), theme, "The overlay shows touch input zones and emulated button state. By default, it's hidden when using any non-touch input device."),
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(18.0),
                }
                Children [
                    description(font.clone(), theme, "Force Emulated Model"),
                    (
                        #ModelSelect
                        multi_select(font.clone(), theme, emulation_model_config(settings.emulation_model as usize))
                        SettingsSelect { field: FIELD_EMULATION_MODEL }
                        UiFocusId { id: TARGET_MODEL }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_MODEL).up}, right: {settings_focus_nav(TARGET_MODEL).right}, down: {settings_focus_nav(TARGET_MODEL).down}, left: {settings_focus_nav(TARGET_MODEL).left} }
                    ),
                ]
            ),
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(18.0),
                }
                Children [
                    description(font.clone(), theme, "Enable Super GameBoy Border"),
                    (
                        #SgbSelect
                        multi_select(font.clone(), theme, yes_no_config(settings.sgb_overlay_enable as usize))
                        SettingsSelect { field: FIELD_SGB_OVERLAY_ENABLE }
                        UiFocusId { id: TARGET_SGB }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_SGB).up}, right: {settings_focus_nav(TARGET_SGB).right}, down: {settings_focus_nav(TARGET_SGB).down}, left: {settings_focus_nav(TARGET_SGB).left} }
                    ),
                ]
            ),
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(18.0),
                }
                Children [
                    description(font.clone(), theme, "Image Upscaling"),
                    (
                        #UpscalingSelect
                        multi_select(font.clone(), theme, upscaling_config(settings.upscaling_mode as usize))
                        SettingsSelect { field: FIELD_UPSCALING_MODE }
                        UiFocusId { id: TARGET_UPSCALING }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_UPSCALING).up}, right: {settings_focus_nav(TARGET_UPSCALING).right}, down: {settings_focus_nav(TARGET_UPSCALING).down}, left: {settings_focus_nav(TARGET_UPSCALING).left} }
                    ),
                ]
            ),
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(18.0),
                }
                Children [
                    description(font.clone(), theme, "UI Scaling"),
                    (
                        #UiScaleSelect
                        multi_select(font.clone(), theme, ui_scale_config(settings.ui_scale as usize))
                        SettingsSelect { field: FIELD_UI_SCALE }
                        UiFocusId { id: TARGET_UI_SCALE }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_UI_SCALE).up}, right: {settings_focus_nav(TARGET_UI_SCALE).right}, down: {settings_focus_nav(TARGET_UI_SCALE).down}, left: {settings_focus_nav(TARGET_UI_SCALE).left} }
                    ),
                ]
            ),
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(18.0),
                }
                Children [
                    description(font.clone(), theme, "UI Theme"),
                    (
                        #ThemeSelect
                        multi_select(font.clone(), theme, theme_config(settings.ui_theme as usize))
                        SettingsSelect { field: FIELD_UI_THEME }
                        UiFocusId { id: TARGET_THEME }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_THEME).up}, right: {settings_focus_nav(TARGET_THEME).right}, down: {settings_focus_nav(TARGET_THEME).down}, left: {settings_focus_nav(TARGET_THEME).left} }
                    ),
                ]
            ),
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(18.0),
                }
                Children [
                    description(font.clone(), theme, "Primary Input Device"),
                    (
                        #PrimaryInputSelect
                        multi_select(font.clone(), theme, primary_input_config())
                        SettingsSelect { field: 255 }
                        UiFocusId { id: TARGET_PRIMARY_INPUT }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PRIMARY_INPUT).up}, right: {settings_focus_nav(TARGET_PRIMARY_INPUT).right}, down: {settings_focus_nav(TARGET_PRIMARY_INPUT).down}, left: {settings_focus_nav(TARGET_PRIMARY_INPUT).left} }
                    ),
                ]
            ),
            (
                #EditMappings
                button(font.clone(), "Edit Mappings", theme, UiFocusNav::default())
                UiFocusId { id: TARGET_EDIT_MAPPINGS }
                UiFocusNavIds { up: {settings_focus_nav(TARGET_EDIT_MAPPINGS).up}, right: {settings_focus_nav(TARGET_EDIT_MAPPINGS).right}, down: {settings_focus_nav(TARGET_EDIT_MAPPINGS).down}, left: {settings_focus_nav(TARGET_EDIT_MAPPINGS).left} }
            ),
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(18.0),
                }
                Children [
                    description(font.clone(), theme, "Audio Preset"),
                    (
                        #AudioPreset
                        multi_select(font.clone(), theme, audio_preset_config())
                        SettingsSelect { field: 255 }
                        UiFocusId { id: TARGET_AUDIO_PRESET }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_AUDIO_PRESET).up}, right: {settings_focus_nav(TARGET_AUDIO_PRESET).right}, down: {settings_focus_nav(TARGET_AUDIO_PRESET).down}, left: {settings_focus_nav(TARGET_AUDIO_PRESET).left} }
                    ),
                ]
            ),
            (
                Node {
                    width: percent(100),
                    column_gap: px(SETTINGS_BUTTON_ROW_GAP),
                    padding: UiRect::bottom(px(120.0)),
                }
                Children [
                    (
                        #DeleteMapping
                        button(font.clone(), "Delete", theme, UiFocusNav::default())
                        DisabledUiElement
                        UiFocusId { id: TARGET_DELETE_MAPPING }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_DELETE_MAPPING).up}, right: {settings_focus_nav(TARGET_DELETE_MAPPING).right}, down: {settings_focus_nav(TARGET_DELETE_MAPPING).down}, left: {settings_focus_nav(TARGET_DELETE_MAPPING).left} }
                    ),
                    (
                        #EditMapping
                        button(font.clone(), "Edit", theme, UiFocusNav::default())
                        UiFocusId { id: TARGET_EDIT_MAPPING }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_EDIT_MAPPING).up}, right: {settings_focus_nav(TARGET_EDIT_MAPPING).right}, down: {settings_focus_nav(TARGET_EDIT_MAPPING).down}, left: {settings_focus_nav(TARGET_EDIT_MAPPING).left} }
                    ),
                    (
                        #CreateMapping
                        button(font.clone(), "Create", theme, UiFocusNav::default())
                        UiFocusId { id: TARGET_CREATE_MAPPING }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_CREATE_MAPPING).up}, right: {settings_focus_nav(TARGET_CREATE_MAPPING).right}, down: {settings_focus_nav(TARGET_CREATE_MAPPING).down}, left: {settings_focus_nav(TARGET_CREATE_MAPPING).left} }
                    ),
                ]
            ),
        ]
    }
}

fn button_overlay_config(selected: usize) -> MultiSelectConfig {
    select_config(selected.min(1), vec!["When needed", "Always"])
}

fn emulation_model_config(selected: usize) -> MultiSelectConfig {
    select_config(
        selected.min(3),
        vec![
            "Best for ROM",
            "Game Boy",
            "Game Boy Color",
            "Super GameBoy",
        ],
    )
}

fn yes_no_config(selected: usize) -> MultiSelectConfig {
    select_config(selected.min(1), vec!["No", "Yes"])
}

fn upscaling_config(selected: usize) -> MultiSelectConfig {
    select_config(selected.min(3), vec!["None", "2x", "3x", "4x"])
}

fn ui_scale_config(selected: usize) -> MultiSelectConfig {
    select_config(
        selected.min(UI_SCALE_LABELS.len() - 1),
        UI_SCALE_LABELS.to_vec(),
    )
}

fn theme_config(selected: usize) -> MultiSelectConfig {
    select_config(
        selected.min(17),
        vec![
            "Random",
            "Minimal",
            "Forest",
            "Jungle",
            "Temple",
            "Cyber",
            "Engine room",
            "Deep sea",
            "Starry night",
            "Alien space",
            "Black hole",
            "Loneliness",
            "Cathedral",
            "Runway",
            "Swamp",
            "Fire cavern",
            "Twilight city",
            "In the clouds",
        ],
    )
}

fn primary_input_config() -> MultiSelectConfig {
    select_config(0, vec!["Keyboard", "XBOX360"])
}

fn audio_preset_config() -> MultiSelectConfig {
    select_config(0, vec!["Preset 0"])
}

fn select_config(selected: usize, options: Vec<&'static str>) -> MultiSelectConfig {
    MultiSelectConfig {
        selected,
        options,
        nav: UiFocusNav::default(),
    }
}

fn rom_storage_list_config(roms: &[RomMetadata]) -> ListViewConfig {
    ListViewConfig {
        nav: UiFocusNav::default(),
        scrollbar_nav: UiFocusNav::default(),
        columns: vec![
            ListColumn {
                heading: "Name",
                width_percent: 42.0,
            },
            ListColumn {
                heading: "Last played",
                width_percent: 34.0,
            },
            ListColumn {
                heading: "Storage Used",
                width_percent: 24.0,
            },
        ],
        rows: virtual_rom_storage_rows(roms),
        virtual_total_rows: Some(roms.len()),
    }
}

fn virtual_rom_storage_rows(roms: &[RomMetadata]) -> Vec<ListRow> {
    let order = (0..roms.len()).collect::<Vec<_>>();
    virtual_list_rows(
        roms,
        &order,
        DEFAULT_VIRTUAL_ROW_POOL_SIZE,
        ROM_STORAGE_COLUMN_COUNT,
        rom_storage_row,
    )
}

fn rom_storage_row(rom: &RomMetadata) -> ListRow {
    ListRow {
        cells: rom_storage_cells(rom),
        nav: UiFocusNav::default(),
    }
}

fn update_rom_storage_row(
    rom: &RomMetadata,
    children: &Children,
    cells: &mut Query<(&mut UiListCellText, &Children)>,
    texts: &mut Query<&mut Text>,
    child_query: &Query<&Children>,
) {
    let values = rom_storage_cells(rom);
    set_list_row_cells(
        &values.iter().map(String::as_str).collect::<Vec<_>>(),
        children,
        cells,
        texts,
        child_query,
    );
}

fn rom_storage_cells(rom: &RomMetadata) -> Vec<String> {
    vec![
        rom.friendly_name
            .clone()
            .unwrap_or_else(|| rom.file_name.clone()),
        String::new(),
        if rom.id.is_some() {
            "Known".to_string()
        } else {
            "Remote".to_string()
        },
    ]
}

fn provider_list_config(providers: &[RomProvider]) -> ListViewConfig {
    ListViewConfig {
        nav: UiFocusNav::default(),
        scrollbar_nav: UiFocusNav::default(),
        columns: vec![
            ListColumn {
                heading: "Name",
                width_percent: 42.0,
            },
            ListColumn {
                heading: "Type",
                width_percent: 30.0,
            },
            ListColumn {
                heading: "Priority",
                width_percent: 28.0,
            },
        ],
        rows: providers.iter().map(provider_row).collect(),
        virtual_total_rows: None,
    }
}

fn provider_row(provider: &RomProvider) -> ListRow {
    ListRow {
        cells: vec![
            provider.friendly_name.clone(),
            provider_type_label(provider).to_string(),
            provider.priority.to_string(),
        ],
        nav: UiFocusNav::default(),
    }
}

fn provider_type_label(provider: &RomProvider) -> &'static str {
    if provider.absolute_local_dir_path.is_some() {
        "Local Directory"
    } else if provider.remote_file_url.is_some() {
        "Remote File"
    } else if provider.remote_api.is_some() {
        "Remote API"
    } else {
        "Built-in"
    }
}
