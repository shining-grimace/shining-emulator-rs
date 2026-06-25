use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui::UiScale;
use bevy_midi_graph::{MidiFileSource, MidiGraphAudioContext, Sf2FileSource, WaveFileSource};
use std::fs;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::{ActiveTheme, ActiveThemeChanged, active_theme_for_setting};
use crate::app_ui_scale::{FONT_SIZE_LABELS, apply_font_size_setting};
use crate::audio::preset_graph::{
    apply_audio_preset_to_playback, default_audio_preset, load_audio_preset,
};
use crate::dimensions::{
    UI_BUTTON_ROW_GAP, UI_CONTENT_GAP, UI_CONTROL_GAP, UI_PANEL_GAP, UI_PORTRAIT_SCREEN_PADDING,
    UI_PRIMARY_COLUMN_PERCENT, UI_SCREEN_PADDING, UI_SECONDARY_COLUMN_PERCENT, UI_SECTION_GAP,
    UI_WIDE_CONTENT_WIDTH,
};
use crate::input::selection::{
    InputMappingEditTarget, PrimaryInputDevice, mapping_label, selected_mapping_index,
};
use crate::scenes::rom_provider::RomProviderEditTarget;
use crate::storage::LocalStorage;
use crate::storage::provider_sync::{ProviderSyncTaskResult, ProviderSyncTaskState};
use crate::storage::providers::RomProvider;
use crate::ui_elements::action_hint::action_hints_with_labels;
use crate::ui_elements::button::button;
use crate::ui_elements::description::description;
use crate::ui_elements::info_message::{InfoMessage, info_message_text, set_latest_info_message};
use crate::ui_elements::interactions::{
    ActivatedUiElement, DefaultFocusTarget, InitialFocus, SelectedUiElement, UI_FOCUS_NONE,
    UiElementKind, UiFocusId, UiFocusNav, UiFocusNavIds, UiListCellText, UiMultiSelect,
};
use crate::ui_elements::list_view::{
    ListColumn, ListRow, ListViewConfig, collect_list_item_entities, list_view, set_list_row_cells,
};
use crate::ui_elements::multi_select::{MultiSelectConfig, multi_select};
use crate::ui_elements::responsive::{
    ResponsiveButtonRow, ResponsiveColumns, ResponsiveFieldRow, ResponsiveLandscapeOnly,
    ResponsivePercentWidth, ResponsivePortraitOnly, ResponsiveScreenPadding,
};
use crate::ui_elements::scroll_view::{ScrollViewConfig, flow_scroll_view, scroll_view};
use crate::ui_elements::settings_header::settings_header;

const SETTINGS_SAVE_ERROR_MESSAGE: &str = "Settings could not be saved";

const FIELD_FORCE_BUTTON_OVERLAY: u8 = 0;
const FIELD_EMULATION_MODEL: u8 = 1;
const FIELD_SGB_OVERLAY_ENABLE: u8 = 2;
const FIELD_UPSCALING_MODE: u8 = 3;
const FIELD_FONT_SIZE: u8 = 4;
const FIELD_UI_THEME: u8 = 5;
const FIELD_AUDIO_PRESET: u8 = 6;
const FIELD_PRIMARY_INPUT: u8 = 255;

const EMULATION_MODEL_BEST_FOR_ROM: u8 = 0;
const EMULATION_MODEL_GAME_BOY_MONO: u8 = 1;
const EMULATION_MODEL_SUPER_GAME_BOY: u8 = 3;

const TARGET_OVERLAY: u16 = 0;
const TARGET_MODEL: u16 = 1;
const TARGET_SGB: u16 = 2;
const TARGET_UPSCALING: u16 = 3;
const TARGET_FONT_SIZE: u16 = 4;
const TARGET_THEME: u16 = 5;
const TARGET_PRIMARY_INPUT: u16 = 6;
const TARGET_EDIT_MAPPINGS: u16 = 7;
const TARGET_AUDIO_PRESET: u16 = 8;
const TARGET_DELETE_MAPPING: u16 = 9;
const TARGET_EDIT_MAPPING: u16 = 10;
const TARGET_CREATE_MAPPING: u16 = 11;
const TARGET_PROVIDER_LIST: u16 = 15;
const TARGET_PROVIDER_SYNC: u16 = 16;
const TARGET_PROVIDER_DELETE: u16 = 17;
const TARGET_PROVIDER_EDIT: u16 = 18;
const TARGET_PROVIDER_CREATE: u16 = 19;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct SettingsSelect {
    field: u8,
}

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

#[derive(SystemParam)]
struct ProviderButtonQueries<'w, 's> {
    create_buttons: Query<'w, 's, (), With<ProviderCreateButton>>,
    edit_buttons: Query<'w, 's, (), With<ProviderEditButton>>,
    delete_buttons: Query<'w, 's, (), With<ProviderDeleteButton>>,
    sync_buttons: Query<'w, 's, (), With<ProviderSyncButton>>,
    selected_provider_rows: Query<'w, 's, &'static ProviderRow, With<SelectedUiElement>>,
    provider_lists: Query<'w, 's, &'static Children, With<ProviderList>>,
    kinds: Query<'w, 's, &'static UiElementKind>,
    child_query: Query<'w, 's, &'static Children>,
    cells: Query<'w, 's, (&'static mut UiListCellText, &'static Children)>,
    texts: Query<'w, 's, &'static mut Text, Without<InfoMessage>>,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct EditPrimaryMappingButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct EditAudioPresetButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct CreateAudioPresetButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct DeleteAudioPresetButton;

pub struct SettingsScenePlugin;

impl Plugin for SettingsScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Settings), spawn_settings_scene)
            .add_systems(
                Update,
                (bind_provider_rows, finish_settings_provider_sync)
                    .run_if(in_state(AppState::Settings)),
            )
            .add_systems(OnExit(AppState::Settings), reset_settings_provider_sync)
            .add_observer(save_settings_select_on_activation)
            .add_observer(handle_provider_button_activation)
            .add_observer(handle_mapping_button_activation)
            .add_observer(handle_audio_preset_button_activation);
    }
}

fn spawn_settings_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    primary_input: Res<PrimaryInputDevice>,
    storage: Res<LocalStorage>,
    sync_state: Res<ProviderSyncTaskState>,
) {
    commands.spawn_scene(settings_scene(
        &assets,
        *theme,
        &primary_input,
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
    mut font_size: ResMut<UiScale>,
    mut primary_input: ResMut<PrimaryInputDevice>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
    asset_server: Res<AssetServer>,
    mut audio_context: ResMut<MidiGraphAudioContext>,
    midi_assets: Res<Assets<MidiFileSource>>,
    sf2_assets: Res<Assets<Sf2FileSource>>,
    wave_assets: Res<Assets<WaveFileSource>>,
) {
    if *state.get() != AppState::Settings {
        return;
    }

    let Ok((settings_select, ui_select)) = selects.get(activated.entity) else {
        return;
    };

    let value = match settings_select.field {
        FIELD_AUDIO_PRESET => selected_audio_preset_number(&storage, ui_select.selected),
        FIELD_EMULATION_MODEL => selected_emulation_model_value(ui_select.selected),
        _ => ui_select.selected as u8,
    };
    if settings_select.field == FIELD_PRIMARY_INPUT {
        primary_input.mapping_index = selected_mapping_index(
            &PrimaryInputDevice {
                mapping_index: ui_select.selected,
            },
            &storage,
        );
        return;
    }

    let previous_value = match settings_select.field {
        FIELD_FORCE_BUTTON_OVERLAY => storage.data.settings.force_button_overlay,
        FIELD_EMULATION_MODEL => storage.data.settings.emulation_model,
        FIELD_SGB_OVERLAY_ENABLE => storage.data.settings.sgb_overlay_enable,
        FIELD_UPSCALING_MODE => storage.data.settings.upscaling_mode,
        FIELD_FONT_SIZE => storage.data.settings.font_size,
        FIELD_UI_THEME => storage.data.settings.ui_theme,
        FIELD_AUDIO_PRESET => storage.data.settings.audio_preset,
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
        FIELD_FONT_SIZE => storage.data.settings.font_size = value,
        FIELD_UI_THEME => storage.data.settings.ui_theme = value,
        FIELD_AUDIO_PRESET => storage.data.settings.audio_preset = value.min(9),
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
    if settings_select.field == FIELD_FONT_SIZE {
        apply_font_size_setting(value, &mut font_size);
    }
    if settings_select.field == FIELD_AUDIO_PRESET {
        apply_current_audio_preset_to_playback(
            &storage,
            &asset_server,
            &mut audio_context,
            &midi_assets,
            &sf2_assets,
            &wave_assets,
            &mut messages,
        );
    }
}

fn handle_mapping_button_activation(
    activated: On<Add, ActivatedUiElement>,
    edit_primary_buttons: Query<(), With<EditPrimaryMappingButton>>,
    mut edit_target: ResMut<InputMappingEditTarget>,
    primary_input: Res<PrimaryInputDevice>,
    storage: Res<LocalStorage>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if *state.get() != AppState::Settings || edit_primary_buttons.get(activated.entity).is_err() {
        return;
    }

    edit_target.mapping_index = selected_mapping_index(&primary_input, &storage);
    next_state.set(AppState::InputMapping);
}

fn handle_audio_preset_button_activation(
    activated: On<Add, ActivatedUiElement>,
    edit_buttons: Query<(), With<EditAudioPresetButton>>,
    create_buttons: Query<(), With<CreateAudioPresetButton>>,
    delete_buttons: Query<(), With<DeleteAudioPresetButton>>,
    state: Res<State<AppState>>,
    mut storage: ResMut<LocalStorage>,
    mut next_state: ResMut<NextState<AppState>>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
    asset_server: Res<AssetServer>,
    mut audio_context: ResMut<MidiGraphAudioContext>,
    midi_assets: Res<Assets<MidiFileSource>>,
    sf2_assets: Res<Assets<Sf2FileSource>>,
    wave_assets: Res<Assets<WaveFileSource>>,
) {
    if *state.get() != AppState::Settings {
        return;
    }

    if edit_buttons.get(activated.entity).is_ok() {
        next_state.set(AppState::AudioSettings);
    } else if create_buttons.get(activated.entity).is_ok() {
        create_audio_preset(&mut storage, &mut next_state, &mut messages);
    } else if delete_buttons.get(activated.entity).is_ok() {
        delete_audio_preset(
            &mut storage,
            &mut next_state,
            &mut messages,
            &asset_server,
            &mut audio_context,
            &midi_assets,
            &sf2_assets,
            &wave_assets,
        );
    }
}

fn apply_current_audio_preset_to_playback(
    storage: &LocalStorage,
    asset_server: &Res<AssetServer>,
    audio_context: &mut MidiGraphAudioContext,
    midi_assets: &Res<Assets<MidiFileSource>>,
    sf2_assets: &Res<Assets<Sf2FileSource>>,
    wave_assets: &Res<Assets<WaveFileSource>>,
    messages: &mut Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
) {
    let preset_path = storage
        .paths
        .audio_preset_file(storage.data.settings.audio_preset.min(9));
    let preset = match load_audio_preset(&preset_path) {
        Ok(preset) => preset,
        Err(error) => {
            eprintln!("failed to load audio preset: {error}");
            set_latest_info_message(
                messages,
                &format!("Audio preset could not be loaded: {error}"),
            );
            return;
        }
    };

    if let Err(error) = apply_audio_preset_to_playback(
        &preset,
        asset_server,
        audio_context,
        midi_assets,
        sf2_assets,
        wave_assets,
    ) {
        eprintln!("failed to apply audio preset: {error}");
        set_latest_info_message(
            messages,
            &format!("Audio preset could not be applied: {error}"),
        );
    }
}

fn delete_audio_preset(
    storage: &mut LocalStorage,
    next_state: &mut NextState<AppState>,
    messages: &mut Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
    asset_server: &Res<AssetServer>,
    audio_context: &mut MidiGraphAudioContext,
    midi_assets: &Res<Assets<MidiFileSource>>,
    sf2_assets: &Res<Assets<Sf2FileSource>>,
    wave_assets: &Res<Assets<WaveFileSource>>,
) {
    let preset_index = storage.data.settings.audio_preset.min(9);
    if preset_index == 0 {
        set_latest_info_message(messages, "The default audio preset cannot be deleted.");
        return;
    }

    let preset_path = storage.paths.audio_preset_file(preset_index);
    if let Err(error) = fs::remove_file(&preset_path) {
        eprintln!(
            "failed to delete audio preset {} at {}: {error}",
            preset_index,
            preset_path.display()
        );
        set_latest_info_message(messages, "Audio preset could not be deleted.");
        return;
    }

    storage.data.settings.audio_preset = 0;
    if let Err(error) = storage.save_settings() {
        eprintln!("failed to save audio preset selection after delete: {error}");
        set_latest_info_message(messages, SETTINGS_SAVE_ERROR_MESSAGE);
        return;
    }

    apply_current_audio_preset_to_playback(
        storage,
        asset_server,
        audio_context,
        midi_assets,
        sf2_assets,
        wave_assets,
        messages,
    );
    set_latest_info_message(messages, "Audio preset deleted.");
    next_state.set(AppState::Settings);
}

fn create_audio_preset(
    storage: &mut LocalStorage,
    next_state: &mut NextState<AppState>,
    messages: &mut Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
) {
    let Some(index) = next_available_audio_preset_index(storage) else {
        set_latest_info_message(messages, "All audio preset slots are already in use.");
        return;
    };

    let preset = default_audio_preset();
    let preset_path = storage.paths.audio_preset_file(index);
    if let Some(parent) = preset_path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        eprintln!("failed to create audio preset directory: {error}");
        set_latest_info_message(messages, "Audio preset could not be created.");
        return;
    }

    match serde_json::to_string_pretty(&preset)
        .map(|json| std::fs::write(&preset_path, format!("{json}\n")))
    {
        Ok(Ok(())) => {
            storage.data.settings.audio_preset = index;
            if let Err(error) = storage.save_settings() {
                eprintln!("failed to save audio preset selection: {error}");
                set_latest_info_message(messages, SETTINGS_SAVE_ERROR_MESSAGE);
                return;
            }
            next_state.set(AppState::AudioSettings);
        }
        Ok(Err(error)) => {
            eprintln!("failed to create audio preset {}: {error}", index);
            set_latest_info_message(messages, "Audio preset could not be created.");
        }
        Err(error) => {
            eprintln!("failed to serialise audio preset {}: {error}", index);
            set_latest_info_message(messages, "Audio preset could not be created.");
        }
    }
}

fn next_available_audio_preset_index(storage: &LocalStorage) -> Option<u8> {
    (1..=9).find(|index| !storage.paths.audio_preset_file(*index).exists())
}

fn selected_audio_preset_number(storage: &LocalStorage, selected: usize) -> u8 {
    existing_audio_preset_numbers(storage)
        .get(selected)
        .copied()
        .unwrap_or(0)
}

fn existing_audio_preset_numbers(storage: &LocalStorage) -> Vec<u8> {
    (0..=9)
        .filter(|index| *index == 0 || storage.paths.audio_preset_file(*index).exists())
        .collect()
}

fn handle_provider_button_activation(
    activated: On<Add, ActivatedUiElement>,
    mut commands: Commands,
    mut queries: ProviderButtonQueries,
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
    if queries.create_buttons.get(entity).is_ok() {
        edit_target.provider_index = None;
        next_state.set(AppState::RomProvider);
    } else if queries.edit_buttons.get(entity).is_ok() {
        let Some(index) = selected_provider_index(&queries.selected_provider_rows) else {
            set_latest_info_message(&mut messages, "Select a ROM provider to edit.");
            return;
        };
        edit_target.provider_index = Some(index);
        next_state.set(AppState::RomProvider);
    } else if queries.delete_buttons.get(entity).is_ok() {
        let Some(index) = selected_provider_index(&queries.selected_provider_rows) else {
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
                refresh_provider_lists(
                    &mut commands,
                    &storage.data.providers,
                    &queries.provider_lists,
                    &queries.kinds,
                    &queries.child_query,
                    &mut queries.cells,
                    &mut queries.texts,
                );
                set_latest_info_message(&mut messages, "ROM provider deleted.");
            }
        }
    } else if queries.sync_buttons.get(entity).is_ok() {
        let Some(index) = selected_provider_index(&queries.selected_provider_rows) else {
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

fn refresh_provider_lists(
    commands: &mut Commands,
    providers: &[RomProvider],
    provider_lists: &Query<&Children, With<ProviderList>>,
    kinds: &Query<&UiElementKind>,
    child_query: &Query<&Children>,
    cells: &mut Query<(&mut UiListCellText, &Children)>,
    texts: &mut Query<&mut Text, Without<InfoMessage>>,
) {
    let provider_rows = providers.iter().map(provider_row).collect::<Vec<_>>();
    for children in provider_lists {
        let row_entities = collect_list_item_entities(children, kinds, child_query);
        for (index, row_entity) in row_entities.into_iter().enumerate() {
            let Some(row) = provider_rows.get(index) else {
                commands.entity(row_entity).try_despawn();
                continue;
            };

            if let Ok(row_children) = child_query.get(row_entity) {
                let values = row.cells.iter().map(String::as_str).collect::<Vec<_>>();
                set_list_row_cells(&values, row_children, cells, texts, child_query);
            }
            commands
                .entity(row_entity)
                .insert(ProviderRow { index })
                .remove::<SelectedUiElement>();
        }
    }
}

fn settings_focus_nav(id: u16) -> UiFocusNavIds {
    match id {
        TARGET_OVERLAY => focus_nav_ids(
            UI_FOCUS_NONE,
            TARGET_PROVIDER_LIST,
            TARGET_MODEL,
            UI_FOCUS_NONE,
        ),
        TARGET_MODEL => focus_nav_ids(
            TARGET_OVERLAY,
            TARGET_PROVIDER_LIST,
            TARGET_SGB,
            UI_FOCUS_NONE,
        ),
        TARGET_SGB => focus_nav_ids(
            TARGET_MODEL,
            TARGET_PROVIDER_LIST,
            TARGET_UPSCALING,
            UI_FOCUS_NONE,
        ),
        TARGET_UPSCALING => focus_nav_ids(
            TARGET_SGB,
            TARGET_PROVIDER_LIST,
            TARGET_FONT_SIZE,
            UI_FOCUS_NONE,
        ),
        TARGET_FONT_SIZE => focus_nav_ids(
            TARGET_UPSCALING,
            TARGET_PROVIDER_LIST,
            TARGET_THEME,
            UI_FOCUS_NONE,
        ),
        TARGET_THEME => focus_nav_ids(
            TARGET_FONT_SIZE,
            TARGET_PROVIDER_LIST,
            TARGET_PRIMARY_INPUT,
            UI_FOCUS_NONE,
        ),
        TARGET_PRIMARY_INPUT => focus_nav_ids(
            TARGET_THEME,
            TARGET_PROVIDER_LIST,
            TARGET_EDIT_MAPPINGS,
            UI_FOCUS_NONE,
        ),
        TARGET_EDIT_MAPPINGS => focus_nav_ids(
            TARGET_PRIMARY_INPUT,
            TARGET_PROVIDER_LIST,
            TARGET_AUDIO_PRESET,
            UI_FOCUS_NONE,
        ),
        TARGET_AUDIO_PRESET => focus_nav_ids(
            TARGET_EDIT_MAPPINGS,
            TARGET_PROVIDER_LIST,
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
            TARGET_PROVIDER_LIST,
            UI_FOCUS_NONE,
            TARGET_EDIT_MAPPING,
        ),
        TARGET_PROVIDER_LIST => focus_nav_ids(
            UI_FOCUS_NONE,
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
    primary_input: &PrimaryInputDevice,
    storage: &LocalStorage,
    sync_running: bool,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let body_font = font.clone();
    let landscape_font = font.clone();
    let settings = storage.data.settings;
    let input_config = primary_input_config(storage, primary_input);
    let landscape_input_config = input_config.clone();
    let audio_config = audio_preset_config(storage, settings.audio_preset as usize);
    let landscape_audio_config = audio_config.clone();
    let providers = storage.data.providers.clone();
    let landscape_providers = providers.clone();
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
        ResponsiveScreenPadding { landscape: UI_SCREEN_PADDING, portrait: UI_PORTRAIT_SCREEN_PADDING }
        Children [
            (
                Node {
                    width: percent(100),
                    max_width: px(UI_WIDE_CONTENT_WIDTH),
                    height: percent(100),
                    min_height: px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(UI_CONTENT_GAP),
                }
                Children [
                    settings_header(font.clone(), assets.icons.clone(), theme, "Settings"),
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
                            settings_landscape_body(landscape_font, theme, settings, landscape_input_config, landscape_audio_config, landscape_providers),
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
                                #SettingsBodyScrollBar
                                flow_scroll_view(
                                    theme,
                                    #SettingsBodyScrollBar,
                                    ScrollViewConfig {
                                        width: percent(100),
                                        min_height: px(0.0),
                                        thumb_height: 112.0,
                                    },
                                    move |_| settings_body(body_font, theme, settings, input_config, audio_config, providers)
                                )
                            )
                        ]
                    ),
                    info_message_text(font.clone(), theme, info_text.to_string(), false),
                    action_hints_with_labels(font, assets.icons.clone(), theme, storage, primary_input, "Back", "Select"),
                ]
            ),
        ]
    }
}

fn settings_left_column(
    font: Handle<Font>,
    theme: ActiveTheme,
    settings: crate::storage::data::GeneralSettings,
    input_config: MultiSelectConfig,
    audio_config: MultiSelectConfig,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_CONTROL_GAP),
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
                ResponsiveFieldRow { gap: 18.0 }
                Children [
                    description(font.clone(), theme, "Show Button Overlay"),
                    (
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
                ResponsiveFieldRow { gap: 18.0 }
                Children [
                    description(font.clone(), theme, "Emulated Model"),
                    (
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
                ResponsiveFieldRow { gap: 18.0 }
                Children [
                    description(font.clone(), theme, "Enable Super GameBoy Border"),
                    (
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
                ResponsiveFieldRow { gap: 18.0 }
                Children [
                    description(font.clone(), theme, "Image Upscaling"),
                    (
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
                ResponsiveFieldRow { gap: 18.0 }
                Children [
                    description(font.clone(), theme, "Font Size"),
                    (
                        multi_select(font.clone(), theme, font_size_config(settings.font_size as usize))
                        SettingsSelect { field: FIELD_FONT_SIZE }
                        UiFocusId { id: TARGET_FONT_SIZE }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_FONT_SIZE).up}, right: {settings_focus_nav(TARGET_FONT_SIZE).right}, down: {settings_focus_nav(TARGET_FONT_SIZE).down}, left: {settings_focus_nav(TARGET_FONT_SIZE).left} }
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
                ResponsiveFieldRow { gap: 18.0 }
                Children [
                    description(font.clone(), theme, "UI Theme"),
                    (
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
                ResponsiveFieldRow { gap: 18.0 }
                Children [
                    description(font.clone(), theme, "Primary Input Device"),
                    (
                        multi_select(font.clone(), theme, input_config)
                        SettingsSelect { field: FIELD_PRIMARY_INPUT }
                        UiFocusId { id: TARGET_PRIMARY_INPUT }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PRIMARY_INPUT).up}, right: {settings_focus_nav(TARGET_PRIMARY_INPUT).right}, down: {settings_focus_nav(TARGET_PRIMARY_INPUT).down}, left: {settings_focus_nav(TARGET_PRIMARY_INPUT).left} }
                    ),
                ]
            ),
            (
                Node {
                    width: percent(100),
                    justify_content: JustifyContent::FlexEnd,
                }
                Children [
                    (
                        button(font.clone(), "Edit Mappings", theme, UiFocusNav::default())
                        EditPrimaryMappingButton
                        UiFocusId { id: TARGET_EDIT_MAPPINGS }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_EDIT_MAPPINGS).up}, right: {settings_focus_nav(TARGET_EDIT_MAPPINGS).right}, down: {settings_focus_nav(TARGET_EDIT_MAPPINGS).down}, left: {settings_focus_nav(TARGET_EDIT_MAPPINGS).left} }
                    )
                ]
            )
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(18.0),
                }
                ResponsiveFieldRow { gap: 18.0 }
                Children [
                    description(font.clone(), theme, "Audio Preset"),
                    (
                        multi_select(font.clone(), theme, audio_config)
                        SettingsSelect { field: FIELD_AUDIO_PRESET }
                        UiFocusId { id: TARGET_AUDIO_PRESET }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_AUDIO_PRESET).up}, right: {settings_focus_nav(TARGET_AUDIO_PRESET).right}, down: {settings_focus_nav(TARGET_AUDIO_PRESET).down}, left: {settings_focus_nav(TARGET_AUDIO_PRESET).left} }
                    ),
                ]
            ),
            (
                Node {
                    width: percent(100),
                    column_gap: px(UI_BUTTON_ROW_GAP),
                    padding: UiRect::bottom(px(120.0)),
                }
                ResponsiveButtonRow { gap: UI_BUTTON_ROW_GAP }
                Children [
                    (
                        button(font.clone(), "Delete", theme, UiFocusNav::default())
                        DeleteAudioPresetButton
                        UiFocusId { id: TARGET_DELETE_MAPPING }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_DELETE_MAPPING).up}, right: {settings_focus_nav(TARGET_DELETE_MAPPING).right}, down: {settings_focus_nav(TARGET_DELETE_MAPPING).down}, left: {settings_focus_nav(TARGET_DELETE_MAPPING).left} }
                    ),
                    (
                        button(font.clone(), "Edit", theme, UiFocusNav::default())
                        EditAudioPresetButton
                        UiFocusId { id: TARGET_EDIT_MAPPING }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_EDIT_MAPPING).up}, right: {settings_focus_nav(TARGET_EDIT_MAPPING).right}, down: {settings_focus_nav(TARGET_EDIT_MAPPING).down}, left: {settings_focus_nav(TARGET_EDIT_MAPPING).left} }
                    ),
                    (
                        button(font.clone(), "Create", theme, UiFocusNav::default())
                        CreateAudioPresetButton
                        UiFocusId { id: TARGET_CREATE_MAPPING }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_CREATE_MAPPING).up}, right: {settings_focus_nav(TARGET_CREATE_MAPPING).right}, down: {settings_focus_nav(TARGET_CREATE_MAPPING).down}, left: {settings_focus_nav(TARGET_CREATE_MAPPING).left} }
                    ),
                ]
            ),
        ]
    }
}

fn settings_body(
    font: Handle<Font>,
    theme: ActiveTheme,
    settings: crate::storage::data::GeneralSettings,
    input_config: MultiSelectConfig,
    audio_config: MultiSelectConfig,
    providers: Vec<RomProvider>,
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
            (
                Node {
                    width: percent(UI_PRIMARY_COLUMN_PERCENT),
                    min_height: px(0.0),
                }
                ResponsivePercentWidth { landscape: UI_PRIMARY_COLUMN_PERCENT }
                Children [
                    settings_left_column(left_font, theme, settings, input_config, audio_config),
                ]
            ),
            (
                Node {
                    width: percent(UI_SECONDARY_COLUMN_PERCENT),
                    min_height: px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(UI_SECTION_GAP),
                }
                ResponsivePercentWidth { landscape: UI_SECONDARY_COLUMN_PERCENT }
                Children [
                    (
                        Node {
                            width: percent(100),
                            min_height: px(0.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(14.0),
                        }
                        Children [
                            description(right_font.clone(), theme, "ROM Providers"),
                            (
                                list_view(right_font.clone(), theme, provider_list_config(&providers))
                                ProviderList
                                UiFocusId { id: TARGET_PROVIDER_LIST }
                                UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_LIST).up}, right: {settings_focus_nav(TARGET_PROVIDER_LIST).right}, down: {settings_focus_nav(TARGET_PROVIDER_LIST).down}, left: {settings_focus_nav(TARGET_PROVIDER_LIST).left} }
                            ),
                            (
                                Node {
                                    width: percent(100),
                                    justify_content: JustifyContent::FlexEnd,
                                    column_gap: px(UI_BUTTON_ROW_GAP),
                                }
                                ResponsiveButtonRow { gap: UI_BUTTON_ROW_GAP }
                                Children [
                                    (
                                        button(right_font.clone(), "Sync", theme, UiFocusNav::default())
                                        ProviderSyncButton
                                        UiFocusId { id: TARGET_PROVIDER_SYNC }
                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_SYNC).up}, right: {settings_focus_nav(TARGET_PROVIDER_SYNC).right}, down: {settings_focus_nav(TARGET_PROVIDER_SYNC).down}, left: {settings_focus_nav(TARGET_PROVIDER_SYNC).left} }
                                    ),
                                    (
                                        button(right_font.clone(), "Delete", theme, UiFocusNav::default())
                                        ProviderDeleteButton
                                        UiFocusId { id: TARGET_PROVIDER_DELETE }
                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_DELETE).up}, right: {settings_focus_nav(TARGET_PROVIDER_DELETE).right}, down: {settings_focus_nav(TARGET_PROVIDER_DELETE).down}, left: {settings_focus_nav(TARGET_PROVIDER_DELETE).left} }
                                    ),
                                    (
                                        button(right_font.clone(), "Edit", theme, UiFocusNav::default())
                                        ProviderEditButton
                                        UiFocusId { id: TARGET_PROVIDER_EDIT }
                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_EDIT).up}, right: {settings_focus_nav(TARGET_PROVIDER_EDIT).right}, down: {settings_focus_nav(TARGET_PROVIDER_EDIT).down}, left: {settings_focus_nav(TARGET_PROVIDER_EDIT).left} }
                                    ),
                                    (
                                        button(right_font, "Create", theme, UiFocusNav::default())
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
    }
}

fn settings_landscape_body(
    font: Handle<Font>,
    theme: ActiveTheme,
    settings: crate::storage::data::GeneralSettings,
    input_config: MultiSelectConfig,
    audio_config: MultiSelectConfig,
    providers: Vec<RomProvider>,
) -> impl Scene {
    let left_font = font.clone();
    let right_font = font;

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
                #LeftScrollBar
                scroll_view(
                    theme,
                    #LeftScrollBar,
                    ScrollViewConfig {
                        width: percent(UI_PRIMARY_COLUMN_PERCENT),
                        min_height: px(0.0),
                        thumb_height: 112.0,
                    },
                    move |_| settings_left_column(left_font, theme, settings, input_config, audio_config)
                )
            ),
            (
                Node {
                    width: percent(UI_SECONDARY_COLUMN_PERCENT),
                    min_height: px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(UI_SECTION_GAP),
                }
                Children [
                    settings_provider_column(right_font, theme, providers),
                ]
            ),
        ]
    }
}

fn settings_provider_column(
    font: Handle<Font>,
    theme: ActiveTheme,
    providers: Vec<RomProvider>,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            min_height: px(0.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(14.0),
        }
        Children [
            description(font.clone(), theme, "ROM Providers"),
            (
                list_view(font.clone(), theme, provider_list_config(&providers))
                ProviderList
                UiFocusId { id: TARGET_PROVIDER_LIST }
                UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_LIST).up}, right: {settings_focus_nav(TARGET_PROVIDER_LIST).right}, down: {settings_focus_nav(TARGET_PROVIDER_LIST).down}, left: {settings_focus_nav(TARGET_PROVIDER_LIST).left} }
            ),
            (
                Node {
                    width: percent(100),
                    justify_content: JustifyContent::FlexEnd,
                    column_gap: px(UI_BUTTON_ROW_GAP),
                }
                ResponsiveButtonRow { gap: UI_BUTTON_ROW_GAP }
                Children [
                    (
                        button(font.clone(), "Sync", theme, UiFocusNav::default())
                        ProviderSyncButton
                        UiFocusId { id: TARGET_PROVIDER_SYNC }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_SYNC).up}, right: {settings_focus_nav(TARGET_PROVIDER_SYNC).right}, down: {settings_focus_nav(TARGET_PROVIDER_SYNC).down}, left: {settings_focus_nav(TARGET_PROVIDER_SYNC).left} }
                    ),
                    (
                        button(font.clone(), "Delete", theme, UiFocusNav::default())
                        ProviderDeleteButton
                        UiFocusId { id: TARGET_PROVIDER_DELETE }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_DELETE).up}, right: {settings_focus_nav(TARGET_PROVIDER_DELETE).right}, down: {settings_focus_nav(TARGET_PROVIDER_DELETE).down}, left: {settings_focus_nav(TARGET_PROVIDER_DELETE).left} }
                    ),
                    (
                        button(font.clone(), "Edit", theme, UiFocusNav::default())
                        ProviderEditButton
                        UiFocusId { id: TARGET_PROVIDER_EDIT }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_EDIT).up}, right: {settings_focus_nav(TARGET_PROVIDER_EDIT).right}, down: {settings_focus_nav(TARGET_PROVIDER_EDIT).down}, left: {settings_focus_nav(TARGET_PROVIDER_EDIT).left} }
                    ),
                    (
                        button(font, "Create", theme, UiFocusNav::default())
                        ProviderCreateButton
                        UiFocusId { id: TARGET_PROVIDER_CREATE }
                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_CREATE).up}, right: {settings_focus_nav(TARGET_PROVIDER_CREATE).right}, down: {settings_focus_nav(TARGET_PROVIDER_CREATE).down}, left: {settings_focus_nav(TARGET_PROVIDER_CREATE).left} }
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
        emulation_model_selected_index(selected as u8),
        vec!["Best for ROM", "GameBoy Mono", "Super GameBoy"],
    )
}

fn selected_emulation_model_value(selected: usize) -> u8 {
    match selected {
        1 => EMULATION_MODEL_GAME_BOY_MONO,
        2 => EMULATION_MODEL_SUPER_GAME_BOY,
        _ => EMULATION_MODEL_BEST_FOR_ROM,
    }
}

fn emulation_model_selected_index(value: u8) -> usize {
    match value {
        EMULATION_MODEL_GAME_BOY_MONO => 1,
        EMULATION_MODEL_SUPER_GAME_BOY => 2,
        _ => 0,
    }
}

fn yes_no_config(selected: usize) -> MultiSelectConfig {
    select_config(selected.min(1), vec!["No", "Yes"])
}

fn upscaling_config(selected: usize) -> MultiSelectConfig {
    select_config(selected.min(3), vec!["None", "2x", "3x", "4x"])
}

fn font_size_config(selected: usize) -> MultiSelectConfig {
    select_config(
        selected.min(FONT_SIZE_LABELS.len() - 1),
        FONT_SIZE_LABELS.to_vec(),
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

fn primary_input_config(
    storage: &LocalStorage,
    primary_input: &PrimaryInputDevice,
) -> MultiSelectConfig {
    let selected = selected_mapping_index(primary_input, storage);
    let options = storage
        .data
        .input_mappings
        .iter()
        .map(mapping_label)
        .collect::<Vec<_>>();
    MultiSelectConfig {
        selected,
        options: if options.is_empty() {
            vec!["Keyboard".to_string()]
        } else {
            options
        },
        nav: UiFocusNav::default(),
    }
}

fn audio_preset_config(storage: &LocalStorage, selected: usize) -> MultiSelectConfig {
    let numbers = existing_audio_preset_numbers(storage);
    let options = numbers
        .iter()
        .map(|index| format!("Preset {index}"))
        .collect::<Vec<_>>();

    let selected_label = format!("Preset {}", selected.min(9));
    let selected_index = options
        .iter()
        .position(|option| option == &selected_label)
        .unwrap_or(0);

    MultiSelectConfig {
        selected: selected_index,
        options,
        nav: UiFocusNav::default(),
    }
}

fn select_config(selected: usize, options: Vec<&'static str>) -> MultiSelectConfig {
    MultiSelectConfig {
        selected,
        options: options.into_iter().map(str::to_string).collect(),
        nav: UiFocusNav::default(),
    }
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
