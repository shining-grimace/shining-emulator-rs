use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task, futures::check_ready};
use bevy::ui::UiGlobalTransform;
use bevy::window::PrimaryWindow;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::dimensions::{
    UI_CONTENT_GAP, UI_FIELD_GAP, UI_MULTI_SELECT_WIDTH, UI_PANEL_GAP, UI_PORTRAIT_SCREEN_PADDING,
    UI_SCREEN_PADDING, UI_SCROLL_CONTENT_BOTTOM_PADDING, UI_SIDEBAR_GROUP_GAP, UI_SIDEBAR_TOP_GAP,
    UI_SIDEBAR_WIDTH,
};
use crate::game_boy::GameBoyRomLoadRequest;
use crate::input::selection::PrimaryInputDevice;
use crate::scenes::rom_data::RomDataEditTarget;
use crate::settings_transition::SettingsNavigation;
use crate::storage::LocalStorage;
use crate::storage::data::RomMetadata;
use crate::storage::provider_sync::{ProviderSyncMessages, ProviderSyncTaskResult, sync_provider};
use crate::ui_elements::action_hint::action_hints;
use crate::ui_elements::button::button_with_width;
use crate::ui_elements::choice_popup::{
    ChoicePopupConfig, ChoicePopupContext, ChoicePopupOption, centered_choice_popup_position,
    choice_popup_context_index, choice_popup_menu, despawn_choice_popups, inside_choice_popup,
};
use crate::ui_elements::description::description;
use crate::ui_elements::heading::heading;
use crate::ui_elements::info_message::{
    InfoMessage, info_message, info_message_text, set_latest_info_message,
};
use crate::ui_elements::interactions::{
    ActivatedUiElement, DefaultFocusTarget, FocusedUiElement, InitialFocus, SelectedUiElement,
    SuppressFocusAutoScroll, UI_FOCUS_NONE, UiElementKind, UiFocusId, UiFocusNav, UiFocusNavIds,
    UiListCellText, UiSchedule, UiScrollArea,
};
use crate::ui_elements::list_view::{
    ListCellIndex, ListColumn, ListRow, ListViewConfig, VirtualListContent, VirtualListRow,
    VirtualListScrollArea, VirtualListWindow, collect_list_item_entities, list_view,
    set_list_row_cells, virtual_list_content_height, virtual_list_rows, virtual_list_window,
};
use crate::ui_elements::multi_select::{MultiSelectConfig, multi_select};
use crate::ui_elements::responsive::{ResponsiveColumns, ResponsiveScreenPadding};
use crate::ui_elements::scroll_view::{ScrollViewConfig, flow_scroll_view};

const HOME_ROM_COLUMN_COUNT: usize = 5;
const HOME_ROM_COLUMN_WIDTHS: [f32; HOME_ROM_COLUMN_COUNT] = [31.0, 20.0, 17.0, 14.0, 18.0];
const HOME_ROM_COMPACT_LIST_WIDTH: f32 = 800.0;
const HOME_ROM_COMPACT_NAME_WIDTH: f32 = 64.0;
const HOME_ROM_COMPACT_LAST_PLAYED_WIDTH: f32 = 36.0;
const HOME_CONTENT_MAX_WIDTH: f32 = 2200.0;
const HOME_PORTRAIT_SIDE_PANEL_HEIGHT: f32 = 128.0;
const HOME_VIRTUAL_ROW_POOL_SIZE: usize = 16;
const HOME_POPUP_ESTIMATED_HEIGHT: f32 = 300.0;

const TARGET_ROM_LIST: u16 = 0;
const TARGET_ALL_SETTINGS: u16 = 1;
const TARGET_PRIMARY_SORT: u16 = 2;
const TARGET_SECONDARY_SORT: u16 = 3;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct SettingsButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct HomeRomList;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct HomeRomListColumn;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct PrimarySortSelect;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct SecondarySortSelect;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct HomeSidePanelSlot;

#[derive(Clone, Copy, Component, Debug, Default)]
struct HomeRomRowsBound;

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
                PreUpdate,
                apply_home_side_panel_layout.run_if(in_state(AppState::Home)),
            )
            .add_systems(
                Update,
                apply_home_rom_list_columns
                    .before(UiSchedule::Widgets)
                    .run_if(in_state(AppState::Home)),
            )
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
                Update,
                (sync_sorted_rom_rows, clear_invisible_home_row_state)
                    .chain()
                    .after(UiSchedule::Scroll)
                    .before(UiSchedule::VisualState)
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
    popup_roots: Query<(Entity, &ChoicePopupContext, &Children)>,
    child_query: Query<&Children>,
    focused: Query<Entity, With<FocusedUiElement>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    rom_list_data: Res<HomeRomListData>,
    mut rom_load_request: ResMut<GameBoyRomLoadRequest>,
    mut rom_data_target: ResMut<RomDataEditTarget>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
    mut navigation: SettingsNavigation,
) {
    if navigation.current() != AppState::Home {
        return;
    }

    let entity = activated.entity;
    if settings_buttons.get(entity).is_ok() {
        navigation.request(AppState::Settings);
    } else if let Ok(option) = popup_options.get(entity) {
        let popup_rom_index = choice_popup_context_index(entity, &popup_roots, &child_query);
        despawn_choice_popups(&mut commands, &popup_roots);
        match option.option_index {
            0 => {
                if let Some(rom_index) = popup_rom_index {
                    rom_load_request.rom_index = Some(rom_index);
                    rom_load_request.resume_auto_save = true;
                    navigation.request(AppState::Gameplay);
                } else {
                    set_latest_info_message(&mut messages, "ROM data could not be opened.");
                }
            }
            1 => {
                if let Some(rom_index) = popup_rom_index {
                    rom_load_request.rom_index = Some(rom_index);
                    rom_load_request.resume_auto_save = false;
                    navigation.request(AppState::Gameplay);
                } else {
                    set_latest_info_message(&mut messages, "ROM data could not be opened.");
                }
            }
            2 => {
                if let Some(rom_index) = popup_rom_index {
                    rom_data_target.rom_index = Some(rom_index);
                    navigation.request(AppState::RomData);
                } else {
                    set_latest_info_message(&mut messages, "ROM data could not be opened.");
                }
            }
            _ => {
                set_latest_info_message(&mut messages, "Launch cancelled.");
                focus_home_rom_row(
                    &mut commands,
                    popup_rom_index,
                    &rom_rows,
                    &focused,
                    &popup_roots,
                    &child_query,
                );
            }
        }
    } else if let Ok((_, row)) = rom_rows.get(entity) {
        if row.item_index == usize::MAX {
            return;
        }

        let Some(rom) = rom_list_data.roms.get(row.item_index).cloned() else {
            set_latest_info_message(&mut messages, "ROM data could not be opened.");
            return;
        };

        despawn_choice_popups(&mut commands, &popup_roots);
        let popup_position = centered_choice_popup_position(
            &windows,
            UI_MULTI_SELECT_WIDTH,
            HOME_POPUP_ESTIMATED_HEIGHT,
        );
        commands.spawn_scene(auto_save_popup_scene(&assets, *theme, rom, popup_position));
    }
}

fn focus_home_rom_row(
    commands: &mut Commands,
    rom_index: Option<usize>,
    rom_rows: &Query<(Entity, &VirtualListRow)>,
    focused: &Query<Entity, With<FocusedUiElement>>,
    popup_roots: &Query<(Entity, &ChoicePopupContext, &Children)>,
    child_query: &Query<&Children>,
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
        if entity != row_entity && !inside_choice_popup(entity, popup_roots, child_query) {
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
    let side_font = font.clone();
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
        ResponsiveScreenPadding { landscape: UI_SCREEN_PADDING, portrait: UI_PORTRAIT_SCREEN_PADDING }
        Children [
            (
                Node {
                    width: percent(100),
                    max_width: px(HOME_CONTENT_MAX_WIDTH),
                    height: percent(100),
                    min_height: px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(UI_CONTENT_GAP),
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
                        ResponsiveColumns { gap: UI_PANEL_GAP }
                        Children [
                            (
                                Node {
                                    flex_grow: 1.0,
                                    flex_shrink: 1.0,
                                    min_height: px(0.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(UI_FIELD_GAP),
                                }
                                HomeRomListColumn
                                Children [
                                    (
                                        #RomList
                                        list_view(font.clone(), theme, rom_list_config(&rom_list_data.roms))
                                        HomeRomList
                                        InitialFocus { enabled: true }
                                        DefaultFocusTarget
                                        UiFocusId { id: TARGET_ROM_LIST }
                                        UiFocusNavIds { up: UI_FOCUS_NONE, right: TARGET_ALL_SETTINGS, down: UI_FOCUS_NONE, left: UI_FOCUS_NONE }
                                    ),
                                    (
                                        info_message_text(font.clone(), theme, initial_status.to_string(), false)
                                        HomeStatusMessage
                                    ),
                                    info_message(font.clone(), theme, "", false)
                                ]
                            ),
                            (
                                #HomeSideScrollBar
                                flow_scroll_view(
                                    theme,
                                    #HomeSideScrollBar,
                                    ScrollViewConfig {
                                        width: px(UI_SIDEBAR_WIDTH),
                                        min_height: px(0.0),
                                        thumb_height: 72.0,
                                    },
                                    move |_| home_side_panel(side_font, theme)
                                )
                                HomeSidePanelSlot
                            ),
                        ]
                    ),
                    action_hints(font, assets.icons.clone(), theme, storage, primary_input),
                ]
            ),
        ]
    }
}

fn apply_home_side_panel_layout(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut panels: Query<&mut Node, With<HomeSidePanelSlot>>,
    mut list_columns: Query<&mut Node, (With<HomeRomListColumn>, Without<HomeSidePanelSlot>)>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let portrait = window.width() < window.height();

    for mut node in &mut panels {
        node.min_height = px(0.0);
        if portrait {
            node.width = percent(100);
            node.height = px(HOME_PORTRAIT_SIDE_PANEL_HEIGHT);
            node.flex_grow = 0.0;
            node.flex_shrink = 0.0;
        } else {
            node.width = px(UI_SIDEBAR_WIDTH);
            node.height = percent(100);
            node.flex_grow = 0.0;
            node.flex_shrink = 0.0;
        }
    }

    for mut node in &mut list_columns {
        node.min_height = px(0.0);
        node.min_width = px(0.0);
        node.flex_grow = 1.0;
        node.flex_shrink = 1.0;
        if portrait {
            node.width = percent(100);
            node.height = px(0.0);
        } else {
            node.width = px(0.0);
            node.height = percent(100);
        }
    }
}

fn home_side_panel(font: Handle<Font>, theme: ActiveTheme) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            min_height: px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_SIDEBAR_GROUP_GAP),
            padding: UiRect {
                left: px(0.0),
                right: px(18.0),
                top: px(0.0),
                bottom: px(UI_SCROLL_CONTENT_BOTTOM_PADDING),
            },
        }
        Children [
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(UI_CONTENT_GAP),
                }
                Children [
                    (
                        #AllSettings
                        button_with_width(font.clone(), "All Settings", theme, UiFocusNav::default(), px(UI_MULTI_SELECT_WIDTH))
                        SettingsButton
                        UiFocusId { id: TARGET_ALL_SETTINGS }
                        UiFocusNavIds { up: UI_FOCUS_NONE, right: UI_FOCUS_NONE, down: TARGET_PRIMARY_SORT, left: TARGET_ROM_LIST }
                    ),
                ]
            ),
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(UI_FIELD_GAP),
                    margin: UiRect::top(px(UI_SIDEBAR_TOP_GAP)),
                }
                Children [
                    description(font.clone(), theme, "Sort ROMs by:"),
                    (
                        #PrimarySort
                        multi_select(font.clone(), theme, sort_select_config(0))
                        PrimarySortSelect
                        UiFocusId { id: TARGET_PRIMARY_SORT }
                        UiFocusNavIds { up: TARGET_ALL_SETTINGS, right: UI_FOCUS_NONE, down: TARGET_SECONDARY_SORT, left: TARGET_ROM_LIST }
                    ),
                    description(font.clone(), theme, "Then by:"),
                    (
                        #SecondarySort
                        multi_select(font, theme, sort_select_config(1))
                        SecondarySortSelect
                        UiFocusId { id: TARGET_SECONDARY_SORT }
                        UiFocusNavIds { up: TARGET_PRIMARY_SORT, right: UI_FOCUS_NONE, down: UI_FOCUS_NONE, left: TARGET_ROM_LIST }
                    ),
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
                width_percent: HOME_ROM_COLUMN_WIDTHS[0],
            },
            ListColumn {
                heading: "Origin",
                width_percent: HOME_ROM_COLUMN_WIDTHS[1],
            },
            ListColumn {
                heading: "Author",
                width_percent: HOME_ROM_COLUMN_WIDTHS[2],
            },
            ListColumn {
                heading: "License",
                width_percent: HOME_ROM_COLUMN_WIDTHS[3],
            },
            ListColumn {
                heading: "Last Played",
                width_percent: HOME_ROM_COLUMN_WIDTHS[4],
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

fn apply_home_rom_list_columns(
    lists: Query<(Entity, &ComputedNode), With<HomeRomList>>,
    mut cells: Query<(Entity, &ListCellIndex, &mut Node)>,
    parents: Query<&ChildOf>,
) {
    let Some((list_entity, list_node)) = lists.iter().next() else {
        return;
    };
    let list_width = list_node.size().x;
    let compact = list_width > 0.0 && list_width < HOME_ROM_COMPACT_LIST_WIDTH;

    for (entity, column, mut node) in &mut cells {
        if !entity_has_ancestor(entity, list_entity, &parents) {
            continue;
        }

        let (display, width_percent) = home_rom_column_layout(column.index, compact);
        let width = percent(width_percent);
        if node.display != display {
            node.display = display;
        }
        if node.width != width {
            node.width = width;
        }
    }
}

fn home_rom_column_layout(column: usize, compact: bool) -> (Display, f32) {
    if compact {
        match column {
            0 => (Display::Flex, HOME_ROM_COMPACT_NAME_WIDTH),
            4 => (Display::Flex, HOME_ROM_COMPACT_LAST_PLAYED_WIDTH),
            _ => (
                Display::None,
                HOME_ROM_COLUMN_WIDTHS
                    .get(column)
                    .copied()
                    .unwrap_or_default(),
            ),
        }
    } else {
        (
            Display::Flex,
            HOME_ROM_COLUMN_WIDTHS
                .get(column)
                .copied()
                .unwrap_or_default(),
        )
    }
}

fn entity_has_ancestor(entity: Entity, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    let mut current = entity;
    while let Ok(parent) = parents.get(current) {
        if parent.0 == ancestor {
            return true;
        }
        current = parent.0;
    }
    false
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
                row_index: slot,
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
    let mut next_focused_entity = None;

    for (entity, mut row, children, _, _) in &mut *rows {
        let row_index = first_visible + row.slot;
        if row.row_index != row_index {
            row.row_index = row_index;
        }

        let Some(rom_index) = rom_list_data.order.get(row_index).copied() else {
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
        if selected {
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
    rom: HomeRom,
    position: Vec2,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();

    bsn! {
        #AutoSavePopup
        DespawnOnExit::<AppState>(AppState::Home)
        choice_popup_menu(
            font,
            theme,
            ChoicePopupConfig {
                title: rom.name,
                width: UI_MULTI_SELECT_WIDTH,
                options: vec!["Resume Auto-save", "Cold Boot", "ROM Data", "Cancel"],
            },
            position,
            rom.rom_index,
        )
    }
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
        return String::new();
    }

    let Some(played) = local_timestamp(timestamp) else {
        return String::new();
    };
    let Some(now) = current_local_timestamp() else {
        return played.date_label();
    };

    if played.date == now.date {
        format!("Today, {}", played.time_label())
    } else if played.date.days_since_unix_epoch() + 1 == now.date.days_since_unix_epoch() {
        format!("Yesterday, {}", played.time_label())
    } else {
        played.date_label()
    }
}

fn current_local_timestamp() -> Option<LocalTimestamp> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| local_timestamp(duration.as_secs()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LocalTimestamp {
    date: CalendarDate,
    hour: u8,
    minute: u8,
}

impl LocalTimestamp {
    fn time_label(self) -> String {
        let suffix = if self.hour < 12 { "AM" } else { "PM" };
        let hour = match self.hour % 12 {
            0 => 12,
            hour => hour,
        };
        format!("{hour:02}:{:02} {suffix}", self.minute)
    }

    fn date_label(self) -> String {
        self.date.label()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CalendarDate {
    year: i32,
    month: u8,
    day: u8,
}

impl CalendarDate {
    fn label(self) -> String {
        let month = month_name(self.month);
        let suffix = ordinal_suffix(self.day);
        format!("{month} {}{suffix}, {}", self.day, self.year)
    }

    fn days_since_unix_epoch(self) -> i64 {
        days_from_civil(self.year, u32::from(self.month), u32::from(self.day))
    }
}

fn month_name(month: u8) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    }
}

fn ordinal_suffix(day: u8) -> &'static str {
    match day % 100 {
        11..=13 => "th",
        _ => match day % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    }
}

#[cfg(any(unix, target_os = "android"))]
fn local_timestamp(timestamp: u64) -> Option<LocalTimestamp> {
    let timestamp = libc::time_t::try_from(timestamp).ok()?;
    let mut time = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: `localtime_r` writes to the provided `tm` pointer when it returns non-null.
    let time = unsafe {
        if libc::localtime_r(&timestamp, time.as_mut_ptr()).is_null() {
            return None;
        }
        time.assume_init()
    };

    Some(LocalTimestamp {
        date: CalendarDate {
            year: time.tm_year + 1900,
            month: u8::try_from(time.tm_mon + 1).ok()?,
            day: u8::try_from(time.tm_mday).ok()?,
        },
        hour: u8::try_from(time.tm_hour).ok()?,
        minute: u8::try_from(time.tm_min).ok()?,
    })
}

#[cfg(not(any(unix, target_os = "android")))]
fn local_timestamp(timestamp: u64) -> Option<LocalTimestamp> {
    utc_timestamp(timestamp)
}

#[cfg(not(any(unix, target_os = "android")))]
fn utc_timestamp(timestamp: u64) -> Option<LocalTimestamp> {
    let days = i64::try_from(timestamp / 86_400).ok()?;
    let seconds = timestamp % 86_400;
    let (year, month, day) = civil_from_days(days);
    Some(LocalTimestamp {
        date: CalendarDate {
            year,
            month: u8::try_from(month).ok()?,
            day: u8::try_from(day).ok()?,
        },
        hour: u8::try_from(seconds / 3_600).ok()?,
        minute: u8::try_from(seconds % 3_600 / 60).ok()?,
    })
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day = i64::from(day);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(not(any(unix, target_os = "android")))]
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);

    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_home_rom_columns_show_name_and_last_played_only() {
        let layouts = (0..HOME_ROM_COLUMN_COUNT)
            .map(|column| home_rom_column_layout(column, true))
            .collect::<Vec<_>>();

        assert_eq!(layouts[0], (Display::Flex, HOME_ROM_COMPACT_NAME_WIDTH));
        assert_eq!(layouts[1].0, Display::None);
        assert_eq!(layouts[2].0, Display::None);
        assert_eq!(layouts[3].0, Display::None);
        assert_eq!(
            layouts[4],
            (Display::Flex, HOME_ROM_COMPACT_LAST_PLAYED_WIDTH)
        );
    }

    #[test]
    fn wide_home_rom_columns_show_all_columns() {
        for column in 0..HOME_ROM_COLUMN_COUNT {
            assert_eq!(
                home_rom_column_layout(column, false),
                (Display::Flex, HOME_ROM_COLUMN_WIDTHS[column])
            );
        }
    }

    #[test]
    fn last_played_time_label_uses_twelve_hour_clock() {
        assert_eq!(
            LocalTimestamp {
                date: CalendarDate {
                    year: 2026,
                    month: 6,
                    day: 26
                },
                hour: 0,
                minute: 5,
            }
            .time_label(),
            "12:05 AM"
        );
        assert_eq!(
            LocalTimestamp {
                date: CalendarDate {
                    year: 2026,
                    month: 6,
                    day: 26
                },
                hour: 15,
                minute: 42,
            }
            .time_label(),
            "03:42 PM"
        );
    }

    #[test]
    fn older_last_played_date_uses_user_friendly_date_label() {
        assert_eq!(
            CalendarDate {
                year: 2026,
                month: 6,
                day: 26
            }
            .label(),
            "June 26th, 2026"
        );
    }

    #[test]
    fn older_last_played_date_uses_correct_ordinal_suffixes() {
        let day_label = |day| {
            CalendarDate {
                year: 2026,
                month: 1,
                day,
            }
            .label()
        };

        assert_eq!(day_label(1), "January 1st, 2026");
        assert_eq!(day_label(2), "January 2nd, 2026");
        assert_eq!(day_label(3), "January 3rd, 2026");
        assert_eq!(day_label(11), "January 11th, 2026");
        assert_eq!(day_label(12), "January 12th, 2026");
        assert_eq!(day_label(13), "January 13th, 2026");
        assert_eq!(day_label(21), "January 21st, 2026");
    }

    #[test]
    fn calendar_day_numbers_are_consecutive_across_month_boundary() {
        let june_30 = CalendarDate {
            year: 2026,
            month: 6,
            day: 30,
        }
        .days_since_unix_epoch();
        let july_1 = CalendarDate {
            year: 2026,
            month: 7,
            day: 1,
        }
        .days_since_unix_epoch();

        assert_eq!(june_30 + 1, july_1);
    }
}
