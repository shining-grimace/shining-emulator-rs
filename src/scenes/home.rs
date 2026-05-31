use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task, futures::check_ready};
use bevy::ui::UiGlobalTransform;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::input::selection::PrimaryInputDevice;
use crate::scenes::rom_data::RomDataEditTarget;
use crate::storage::LocalStorage;
use crate::storage::data::RomMetadata;
use crate::storage::provider_sync::{ProviderSyncMessages, ProviderSyncTaskResult, sync_provider};
use crate::ui_elements::action_hint::action_hints;
use crate::ui_elements::button::button;
use crate::ui_elements::choice_popup::{
    ChoicePopupConfig, ChoicePopupOption, DismissChoicePopupOnOutsideClick, choice_popup,
};
use crate::ui_elements::description::description;
use crate::ui_elements::heading::heading;
use crate::ui_elements::info_message::{
    InfoMessage, info_message, info_message_text, set_latest_info_message,
};
use crate::ui_elements::interactions::{
    ActivatedUiElement, DefaultFocusTarget, FocusedUiElement, InitialFocus, SelectedUiElement,
    SuppressFocusAutoScroll, UiElementKind, UiFocusNav, UiListCellText, UiScrollArea,
};
use crate::ui_elements::list_view::{
    ListColumn, ListRow, ListViewConfig, VirtualListContent, VirtualListRow, VirtualListScrollArea,
    VirtualListWindow, collect_list_item_entities, list_view, set_list_row_cells,
    virtual_list_content_height, virtual_list_rows, virtual_list_window,
};
use crate::ui_elements::multi_select::{MultiSelectConfig, multi_select};
use crate::ui_elements::styles::{UI_MAX_CONTENT_WIDTH, UI_PANEL_GAP, UI_SCREEN_PADDING};

const HOME_CONTENT_GAP: f32 = 32.0;
const HOME_MESSAGE_GAP: f32 = 12.0;
const HOME_SIDE_PANEL_WIDTH: f32 = 280.0;
const HOME_SIDE_TOP_GAP: f32 = 50.0;
const HOME_SIDE_GROUP_GAP: f32 = 46.0;
const HOME_SORT_LABEL_GAP: f32 = 12.0;
const HOME_SORT_GROUP_GAP: f32 = 24.0;
const AUTO_SAVE_POPUP_WIDTH: f32 = 260.0;
const AUTO_SAVE_POPUP_LEFT: f32 = 760.0;
const HOME_ROM_COLUMN_COUNT: usize = 5;
const HOME_VIRTUAL_ROW_POOL_SIZE: usize = 16;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct SettingsButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct HomeRomList;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct PrimarySortSelect;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct SecondarySortSelect;

#[derive(Clone, Copy, Component, Debug, Default)]
struct HomeRomRowsBound;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct HomePopupRoot {
    rom_index: usize,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct HomeStatusMessage;

#[derive(Resource)]
struct HomeProviderSyncState {
    started: bool,
    task: Option<Task<ProviderSyncTaskResult>>,
}

pub struct HomeScenePlugin;

impl Plugin for HomeScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HomeRomListData>()
            .init_resource::<HomeVirtualListSyncState>()
            .add_systems(OnEnter(AppState::Home), spawn_home_scene)
            .add_systems(
                Update,
                (
                    start_home_provider_sync,
                    finish_home_provider_sync,
                    bind_home_rom_rows,
                    refresh_home_status_message,
                )
                    .run_if(in_state(AppState::Home)),
            )
            .add_systems(
                PostUpdate,
                (sync_sorted_rom_rows, clear_invisible_home_row_state)
                    .run_if(in_state(AppState::Home)),
            )
            .add_observer(handle_home_activation);
    }
}

fn spawn_home_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    primary_input: Res<PrimaryInputDevice>,
    storage: Res<LocalStorage>,
    mut rom_list_data: ResMut<HomeRomListData>,
    mut virtual_list_sync: ResMut<HomeVirtualListSyncState>,
) {
    rom_list_data.roms = home_roms_from_storage(&storage);
    rom_list_data.refresh_order(SortField::LastPlayed, SortField::ProviderPriority);
    *virtual_list_sync = HomeVirtualListSyncState::default();
    commands.spawn_scene(home_scene(
        &assets,
        *theme,
        &storage,
        &primary_input,
        &rom_list_data,
    ));
    commands.insert_resource(HomeProviderSyncState {
        started: false,
        task: None,
    });
}

fn start_home_provider_sync(
    state: Option<ResMut<HomeProviderSyncState>>,
    storage: Res<LocalStorage>,
) {
    let Some(mut state) = state else {
        return;
    };
    if state.started {
        return;
    }
    state.started = true;

    let providers = storage
        .data
        .providers
        .iter()
        .filter(|provider| provider.enabled)
        .cloned()
        .collect::<Vec<_>>();
    if providers.is_empty() {
        return;
    }

    state.task = Some(IoTaskPool::get().spawn(async move {
        let mut result = ProviderSyncTaskResult::default();
        for provider in providers {
            match sync_provider(&provider) {
                Ok(provider_result) => result.results.push(provider_result),
                Err(error) => result
                    .failures
                    .push(format!("{}: {error}", provider.friendly_name)),
            }
        }
        result
    }));
}

fn finish_home_provider_sync(
    mut commands: Commands,
    mut storage: ResMut<LocalStorage>,
    mut sync_messages: ResMut<ProviderSyncMessages>,
    state: Option<ResMut<HomeProviderSyncState>>,
    mut rom_list_data: ResMut<HomeRomListData>,
    mut virtual_list_sync: ResMut<HomeVirtualListSyncState>,
    mut rows: Query<(
        Entity,
        &mut VirtualListRow,
        &Children,
        Has<FocusedUiElement>,
        Has<SelectedUiElement>,
    )>,
    mut cells: Query<(&mut UiListCellText, &Children)>,
    mut texts: Query<&mut Text>,
    child_query: Query<&Children>,
) {
    let Some(mut state) = state else {
        return;
    };
    let Some(task) = state.task.as_mut() else {
        return;
    };
    let Some(result) = check_ready(task) else {
        return;
    };
    state.task = None;

    sync_messages.failures = storage.apply_provider_sync_result(result);
    rom_list_data.roms = home_roms_from_storage(&storage);
    rom_list_data.refresh_order(SortField::LastPlayed, SortField::ProviderPriority);
    refresh_home_rom_rows(
        &mut commands,
        &rom_list_data,
        0,
        &mut rows,
        &mut cells,
        &mut texts,
        &child_query,
    );
    *virtual_list_sync = HomeVirtualListSyncState::default();
}

fn handle_home_activation(
    activated: On<Add, ActivatedUiElement>,
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    settings_buttons: Query<(), With<SettingsButton>>,
    popup_options: Query<&ChoicePopupOption>,
    rom_rows: Query<(Entity, &VirtualListRow)>,
    row_nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    lists: Query<(&ComputedNode, &UiGlobalTransform), With<HomeRomList>>,
    popup_roots: Query<(Entity, &HomePopupRoot, &Children)>,
    child_query: Query<&Children>,
    focused: Query<Entity, With<FocusedUiElement>>,
    rom_list_data: Res<HomeRomListData>,
    mut rom_data_target: ResMut<RomDataEditTarget>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if *state.get() != AppState::Home {
        return;
    }

    let entity = activated.entity;
    if settings_buttons.get(entity).is_ok() {
        next_state.set(AppState::Settings);
    } else if let Ok(option) = popup_options.get(entity) {
        let popup_rom_index = popup_rom_index(entity, &popup_roots, &child_query);
        despawn_home_popups(&mut commands, &popup_roots);
        match option.option_index {
            0 => {
                set_latest_info_message(&mut messages, "Resume Auto-save selected.");
                focus_home_rom_row(&mut commands, popup_rom_index, &rom_rows, &focused);
            }
            1 => {
                set_latest_info_message(&mut messages, "Cold Boot selected.");
                focus_home_rom_row(&mut commands, popup_rom_index, &rom_rows, &focused);
            }
            2 => {
                if let Some(rom_index) = popup_rom_index {
                    rom_data_target.rom_index = Some(rom_index);
                    next_state.set(AppState::RomData);
                } else {
                    set_latest_info_message(&mut messages, "ROM data could not be opened.");
                }
            }
            _ => {
                set_latest_info_message(&mut messages, "Launch cancelled.");
                focus_home_rom_row(&mut commands, popup_rom_index, &rom_rows, &focused);
            }
        }
    } else if let Ok((_, row)) = rom_rows.get(entity) {
        if home_row_visible(entity, &row_nodes, &lists) {
            despawn_home_popups(&mut commands, &popup_roots);
            for entity in &focused {
                commands.entity(entity).remove::<FocusedUiElement>();
            }
            commands.spawn_scene(auto_save_popup_scene(
                &assets,
                *theme,
                rom_list_data.roms.get(row.item_index).cloned(),
            ));
        }
    }
}

fn focus_home_rom_row(
    commands: &mut Commands,
    rom_index: Option<usize>,
    rom_rows: &Query<(Entity, &VirtualListRow)>,
    focused: &Query<Entity, With<FocusedUiElement>>,
) {
    let Some(rom_index) = rom_index else {
        return;
    };
    let Some(row_entity) = rom_rows
        .iter()
        .find_map(|(entity, row)| (row.item_index == rom_index).then_some(entity))
    else {
        return;
    };

    for entity in focused {
        if entity != row_entity {
            commands.entity(entity).remove::<FocusedUiElement>();
        }
    }
    commands.entity(row_entity).insert(FocusedUiElement);
}

fn home_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    storage: &LocalStorage,
    primary_input: &PrimaryInputDevice,
    rom_list_data: &HomeRomListData,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let initial_status = if rom_list_data.roms.is_empty() {
        "No ROMs found. Sync or add a ROM provider in Settings."
    } else {
        ""
    };

    bsn! {
        #HomeScene
        DespawnOnExit::<AppState>(AppState::Home)
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
                    row_gap: px(HOME_CONTENT_GAP),
                }
                Children [
                    heading(font.clone(), theme, "Shining Emulator"),
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
                                Node {
                                    flex_grow: 1.0,
                                    flex_shrink: 1.0,
                                    min_height: px(0.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(HOME_MESSAGE_GAP),
                                }
                                Children [
                                    (
                                        #RomList
                                        list_view(font.clone(), theme, rom_list_config(&rom_list_data.roms))
                                        HomeRomList
                                        InitialFocus { enabled: true }
                                        DefaultFocusTarget
                                        UiFocusNav { up: {Entity::PLACEHOLDER}, right: #AllSettings, down: {Entity::PLACEHOLDER}, left: {Entity::PLACEHOLDER} }
                                    ),
                                    (
                                        info_message_text(font.clone(), theme, initial_status.to_string(), false)
                                        HomeStatusMessage
                                    ),
                                    info_message(font.clone(), theme, "", false)
                                ]
                            ),
                            (
                                Node {
                                    width: px(HOME_SIDE_PANEL_WIDTH),
                                    min_height: px(0.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(HOME_SIDE_GROUP_GAP),
                                }
                                Children [
                                    (
                                        Node {
                                            width: percent(100),
                                            flex_direction: FlexDirection::Column,
                                            row_gap: px(HOME_SORT_GROUP_GAP),
                                        }
                                        Children [
                                            (
                                                #AllSettings
                                                button(font.clone(), "All Settings", theme, UiFocusNav::default())
                                                SettingsButton
                                                UiFocusNav { up: {Entity::PLACEHOLDER}, right: {Entity::PLACEHOLDER}, down: #PrimarySort, left: #RomList }
                                            ),
                                        ]
                                    ),
                                    (
                                        Node {
                                            width: percent(100),
                                            flex_direction: FlexDirection::Column,
                                            row_gap: px(HOME_SORT_LABEL_GAP),
                                            margin: UiRect::top(px(HOME_SIDE_TOP_GAP)),
                                        }
                                        Children [
                                            description(font.clone(), theme, "Sort ROMs by:"),
                                            (
                                                #PrimarySort
                                                multi_select(font.clone(), theme, sort_select_config(0))
                                                PrimarySortSelect
                                                UiFocusNav { up: #AllSettings, right: {Entity::PLACEHOLDER}, down: #SecondarySort, left: #RomList }
                                            ),
                                            description(font.clone(), theme, "Then by:"),
                                            (
                                                #SecondarySort
                                                multi_select(font.clone(), theme, sort_select_config(1))
                                                SecondarySortSelect
                                                UiFocusNav { up: #PrimarySort, right: {Entity::PLACEHOLDER}, down: {Entity::PLACEHOLDER}, left: #RomList }
                                            ),
                                        ]
                                    ),
                                ]
                            ),
                        ]
                    ),
                    action_hints(font, assets.icons.clone(), theme, storage, primary_input),
                ]
            ),
        ]
    }
}

fn sort_select_config(selected: usize) -> MultiSelectConfig {
    MultiSelectConfig {
        selected,
        options: vec![
            "Last played".to_string(),
            "Provider priority".to_string(),
            "A-Z".to_string(),
        ],
        nav: UiFocusNav::default(),
    }
}

fn rom_list_config(roms: &[HomeRom]) -> ListViewConfig {
    ListViewConfig {
        nav: UiFocusNav::default(),
        scrollbar_nav: UiFocusNav::default(),
        columns: vec![
            ListColumn {
                heading: "Name",
                width_percent: 31.0,
            },
            ListColumn {
                heading: "Origin",
                width_percent: 20.0,
            },
            ListColumn {
                heading: "Author",
                width_percent: 17.0,
            },
            ListColumn {
                heading: "License",
                width_percent: 14.0,
            },
            ListColumn {
                heading: "Last played",
                width_percent: 18.0,
            },
        ],
        rows: virtual_rom_rows(roms),
        virtual_total_rows: Some(roms.len()),
    }
}

fn virtual_rom_rows(roms: &[HomeRom]) -> Vec<ListRow> {
    let order = sorted_rom_indices(roms, SortField::LastPlayed, SortField::ProviderPriority);
    virtual_list_rows(
        roms,
        &order,
        HOME_VIRTUAL_ROW_POOL_SIZE,
        HOME_ROM_COLUMN_COUNT,
        rom_row,
    )
}

fn rom_row(rom: &HomeRom) -> ListRow {
    ListRow {
        cells: vec![
            rom.name.clone(),
            rom.origin.clone(),
            rom.author.clone(),
            rom.license.clone(),
            rom.last_played_label.clone(),
        ],
        nav: UiFocusNav::default(),
    }
}

fn bind_home_rom_rows(
    mut commands: Commands,
    lists: Query<(Entity, &Children), (With<HomeRomList>, Without<HomeRomRowsBound>)>,
    kinds: Query<&UiElementKind>,
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
        commands.entity(list_entity).insert(HomeRomRowsBound);
    }
}

fn sync_sorted_rom_rows(
    mut commands: Commands,
    primary: Query<&crate::ui_elements::interactions::UiMultiSelect, With<PrimarySortSelect>>,
    secondary: Query<&crate::ui_elements::interactions::UiMultiSelect, With<SecondarySortSelect>>,
    scroll_areas: Query<&UiScrollArea, With<VirtualListScrollArea>>,
    mut virtual_contents: Query<&mut Node, With<VirtualListContent>>,
    mut rows: Query<(
        Entity,
        &mut VirtualListRow,
        &Children,
        Has<FocusedUiElement>,
        Has<SelectedUiElement>,
    )>,
    mut cells: Query<(&mut UiListCellText, &Children)>,
    mut texts: Query<&mut Text>,
    child_query: Query<&Children>,
    mut rom_list_data: ResMut<HomeRomListData>,
    mut sync_state: ResMut<HomeVirtualListSyncState>,
) {
    let primary_sort = primary
        .single()
        .map(|select| SortField::from_index(select.selected))
        .unwrap_or(SortField::LastPlayed);
    let secondary_sort = secondary
        .single()
        .map(|select| SortField::from_index(select.selected))
        .unwrap_or(SortField::ProviderPriority);
    let order_changed = rom_list_data.refresh_order(primary_sort, secondary_sort);
    let window = scroll_areas
        .iter()
        .next()
        .map(virtual_list_window)
        .unwrap_or(VirtualListWindow {
            first_row: 0,
            content_offset: 0.0,
        });
    let content_height =
        virtual_list_content_height(rom_list_data.roms.len(), HOME_VIRTUAL_ROW_POOL_SIZE);
    let content_moved = (sync_state.content_offset - window.content_offset).abs() > f32::EPSILON;
    let visible_window_changed = sync_state.first_row != window.first_row || order_changed;
    let content_height_changed = (sync_state.content_height - content_height).abs() > f32::EPSILON;

    if !content_moved && !visible_window_changed && !content_height_changed {
        return;
    }

    for mut node in &mut virtual_contents {
        let top = px(-window.content_offset);
        if node.top != top {
            node.top = top;
        }
        let height = px(content_height);
        if node.height != height {
            node.height = height;
        }
    }

    if visible_window_changed || content_height_changed {
        refresh_home_rom_rows(
            &mut commands,
            &rom_list_data,
            window.first_row,
            &mut rows,
            &mut cells,
            &mut texts,
            &child_query,
        );
    }

    *sync_state = HomeVirtualListSyncState {
        first_row: window.first_row,
        content_offset: window.content_offset,
        content_height,
    };
}

fn refresh_home_rom_rows(
    commands: &mut Commands,
    rom_list_data: &HomeRomListData,
    first_visible: usize,
    rows: &mut Query<(
        Entity,
        &mut VirtualListRow,
        &Children,
        Has<FocusedUiElement>,
        Has<SelectedUiElement>,
    )>,
    cells: &mut Query<(&mut UiListCellText, &Children)>,
    texts: &mut Query<&mut Text>,
    child_query: &Query<&Children>,
) {
    let focused_item_index = rows
        .iter()
        .find_map(|(_, row, _, focused, _)| focused.then_some(row.item_index))
        .filter(|item_index| *item_index != usize::MAX);
    let current_focused_entity = rows
        .iter()
        .find_map(|(entity, _, _, focused, _)| focused.then_some(entity));
    let selected_item_index = rows
        .iter()
        .find_map(|(_, row, _, _, selected)| selected.then_some(row.item_index))
        .filter(|item_index| *item_index != usize::MAX);
    let mut next_focused_entity = None;
    let mut next_selected_entity = None;

    for (entity, mut row, children, _, _) in &mut *rows {
        let Some(rom_index) = rom_list_data.order.get(first_visible + row.slot).copied() else {
            if row.item_index != usize::MAX {
                row.item_index = usize::MAX;
                set_list_row_cells(
                    &["", "", "", "", ""],
                    children,
                    &mut *cells,
                    &mut *texts,
                    &child_query,
                );
            }
            continue;
        };
        if focused_item_index == Some(rom_index) {
            next_focused_entity = Some(entity);
        }
        if selected_item_index == Some(rom_index) {
            next_selected_entity = Some(entity);
        }
        if row.item_index == rom_index {
            continue;
        }
        row.item_index = rom_index;
        let Some(rom) = rom_list_data.roms.get(rom_index) else {
            continue;
        };
        update_row_cells(rom, children, &mut *cells, &mut *texts, &child_query);
    }

    for (entity, _, _, focused, selected) in rows {
        if focused && Some(entity) != next_focused_entity {
            commands.entity(entity).remove::<FocusedUiElement>();
        }
        if selected && Some(entity) != next_selected_entity {
            commands.entity(entity).remove::<SelectedUiElement>();
        }
    }
    if let Some(next_focused_entity) = next_focused_entity {
        let mut entity_commands = commands.entity(next_focused_entity);
        entity_commands.insert(FocusedUiElement);
        if Some(next_focused_entity) != current_focused_entity {
            entity_commands.insert(SuppressFocusAutoScroll);
        }
    }
    if let Some(next_selected_entity) = next_selected_entity {
        commands
            .entity(next_selected_entity)
            .insert(SelectedUiElement);
    }
}

fn update_row_cells(
    rom: &HomeRom,
    children: &Children,
    cells: &mut Query<(&mut UiListCellText, &Children)>,
    texts: &mut Query<&mut Text>,
    child_query: &Query<&Children>,
) {
    let values = [
        rom.name.as_str(),
        rom.origin.as_str(),
        rom.author.as_str(),
        rom.license.as_str(),
        rom.last_played_label.as_str(),
    ];
    set_list_row_cells(&values, children, cells, texts, child_query);
}

fn refresh_home_status_message(
    rom_list_data: Res<HomeRomListData>,
    sync_messages: Res<ProviderSyncMessages>,
    state: Option<Res<HomeProviderSyncState>>,
    mut messages: Query<&mut Text, With<HomeStatusMessage>>,
) {
    let Ok(mut text) = messages.single_mut() else {
        return;
    };

    let next_text = if !sync_messages.failures.is_empty() {
        sync_messages
            .failures
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    } else if rom_list_data.roms.is_empty()
        && state.as_ref().is_some_and(|state| state.task.is_some())
    {
        "Syncing ROM providers...".to_string()
    } else if rom_list_data.roms.is_empty() {
        "No ROMs found. Sync or add a ROM provider in Settings.".to_string()
    } else {
        String::new()
    };
    if text.0 != next_text {
        text.0 = next_text;
    }
}

fn auto_save_popup_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    rom: Option<HomeRom>,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let rom_name = rom
        .as_ref()
        .map(|rom| rom.name.clone())
        .unwrap_or_else(|| "ROM".to_string());
    let rom_index = rom.map(|rom| rom.rom_index).unwrap_or(usize::MAX);

    bsn! {
        #AutoSavePopup
        DespawnOnExit::<AppState>(AppState::Home)
        Node {
            position_type: PositionType::Absolute,
            left: px(AUTO_SAVE_POPUP_LEFT),
            bottom: px(126.0),
        }
        HomePopupRoot { rom_index: {rom_index} }
        DismissChoicePopupOnOutsideClick
        Children [
            choice_popup(font, theme, ChoicePopupConfig {
                title: rom_name,
                width: AUTO_SAVE_POPUP_WIDTH,
                options: ["Resume Auto-save", "Cold Boot", "ROM Data", "Cancel"],
            })
        ]
    }
}

fn despawn_home_popups(
    commands: &mut Commands,
    popup_roots: &Query<(Entity, &HomePopupRoot, &Children)>,
) {
    for (popup, _, _) in popup_roots {
        commands.entity(popup).despawn();
    }
}

fn popup_rom_index(
    option_entity: Entity,
    popup_roots: &Query<(Entity, &HomePopupRoot, &Children)>,
    child_query: &Query<&Children>,
) -> Option<usize> {
    popup_roots
        .iter()
        .find(|(_, _, children)| contains_entity_recursive(children, option_entity, child_query))
        .map(|(_, popup, _)| popup.rom_index)
        .filter(|index| *index != usize::MAX)
}

fn contains_entity_recursive(
    children: &Children,
    target: Entity,
    child_query: &Query<&Children>,
) -> bool {
    children.iter().any(|child| {
        child == target
            || child_query
                .get(child)
                .is_ok_and(|children| contains_entity_recursive(children, target, child_query))
    })
}

fn clear_invisible_home_row_state(
    mut commands: Commands,
    rows: Query<
        (
            Entity,
            Has<SelectedUiElement>,
            Has<FocusedUiElement>,
            Has<ActivatedUiElement>,
        ),
        With<VirtualListRow>,
    >,
    row_nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    lists: Query<(&ComputedNode, &UiGlobalTransform), With<HomeRomList>>,
) {
    for (entity, selected, focused, activated) in &rows {
        if home_row_visible(entity, &row_nodes, &lists) {
            continue;
        }

        let mut entity_commands = commands.entity(entity);
        if selected {
            entity_commands.remove::<SelectedUiElement>();
        }
        if focused {
            entity_commands.remove::<FocusedUiElement>();
        }
        if activated {
            entity_commands.remove::<ActivatedUiElement>();
        }
    }
}

fn home_row_visible(
    row: Entity,
    row_nodes: &Query<(&ComputedNode, &UiGlobalTransform)>,
    lists: &Query<(&ComputedNode, &UiGlobalTransform), With<HomeRomList>>,
) -> bool {
    let Ok((row_node, row_transform)) = row_nodes.get(row) else {
        return false;
    };
    let Some((list_node, list_transform)) = lists.iter().next() else {
        return true;
    };

    let (_, _, row_center) = row_transform.to_scale_angle_translation();
    let (_, _, list_center) = list_transform.to_scale_angle_translation();
    let row_half_height = row_node.size().y * 0.5;
    let list_half_height = list_node.size().y * 0.5;
    let list_top = list_center.y - list_half_height;
    let list_bottom = list_center.y + list_half_height;
    let row_top = row_center.y - row_half_height;
    let row_bottom = row_center.y + row_half_height;

    row_bottom > list_top && row_top < list_bottom
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SortField {
    LastPlayed,
    ProviderPriority,
    Alphabetical,
}

impl Default for SortField {
    fn default() -> Self {
        Self::LastPlayed
    }
}

impl SortField {
    fn from_index(index: usize) -> Self {
        match index {
            1 => Self::ProviderPriority,
            2 => Self::Alphabetical,
            _ => Self::LastPlayed,
        }
    }
}

fn sorted_rom_indices(roms: &[HomeRom], primary: SortField, secondary: SortField) -> Vec<usize> {
    let mut indices = (0..roms.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| compare_roms(&roms[*left], &roms[*right], primary, secondary));
    indices
}

fn compare_roms(
    left: &HomeRom,
    right: &HomeRom,
    primary: SortField,
    secondary: SortField,
) -> std::cmp::Ordering {
    compare_field(left, right, primary)
        .then_with(|| compare_field(left, right, secondary))
        .then_with(|| compare_field(left, right, SortField::Alphabetical))
}

fn compare_field(left: &HomeRom, right: &HomeRom, field: SortField) -> std::cmp::Ordering {
    match field {
        SortField::LastPlayed => right.last_played_rank.cmp(&left.last_played_rank),
        SortField::ProviderPriority => left
            .provider_priority
            .cmp(&right.provider_priority)
            .then_with(|| left.origin.cmp(&right.origin)),
        SortField::Alphabetical => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
    }
}

#[derive(Clone, Debug, Default, Resource)]
struct HomeRomListData {
    roms: Vec<HomeRom>,
    order: Vec<usize>,
    primary_sort: SortField,
    secondary_sort: SortField,
}

impl HomeRomListData {
    fn refresh_order(&mut self, primary_sort: SortField, secondary_sort: SortField) -> bool {
        if self.primary_sort == primary_sort
            && self.secondary_sort == secondary_sort
            && self.order.len() == self.roms.len()
        {
            return false;
        }

        self.primary_sort = primary_sort;
        self.secondary_sort = secondary_sort;
        self.order = sorted_rom_indices(&self.roms, primary_sort, secondary_sort);
        true
    }
}

#[derive(Clone, Copy, Debug, Default, Resource)]
struct HomeVirtualListSyncState {
    first_row: usize,
    content_offset: f32,
    content_height: f32,
}

#[derive(Clone, Debug)]
struct HomeRom {
    rom_index: usize,
    name: String,
    origin: String,
    author: String,
    license: String,
    last_played_rank: u64,
    last_played_label: String,
    provider_priority: u8,
}

fn home_roms_from_storage(storage: &LocalStorage) -> Vec<HomeRom> {
    storage
        .data
        .roms
        .iter()
        .enumerate()
        .filter_map(|(index, rom)| home_rom_from_metadata(index, rom, storage))
        .collect()
}

fn home_rom_from_metadata(
    rom_index: usize,
    rom: &RomMetadata,
    storage: &LocalStorage,
) -> Option<HomeRom> {
    let provider = storage
        .data
        .providers
        .iter()
        .find(|provider| provider.uuid == rom.provider_id)?;
    if !provider.enabled {
        return None;
    }
    let last_played_rank = rom
        .id
        .as_ref()
        .and_then(|id| {
            storage
                .data
                .timestamps
                .last_played
                .iter()
                .find(|timestamp| &timestamp.id == id)
                .map(|timestamp| timestamp.timestamp)
        })
        .unwrap_or_default();

    Some(HomeRom {
        rom_index,
        name: rom
            .friendly_name
            .clone()
            .unwrap_or_else(|| rom.file_name.clone()),
        origin: provider.friendly_name.clone(),
        author: rom.author.clone().unwrap_or_default(),
        license: rom.license.clone().unwrap_or_default(),
        last_played_rank,
        last_played_label: last_played_label(last_played_rank),
        provider_priority: provider.priority,
    })
}

fn last_played_label(timestamp: u64) -> String {
    if timestamp == 0 {
        String::new()
    } else {
        format!("Played {timestamp}")
    }
}
