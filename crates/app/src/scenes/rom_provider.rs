use bevy::math::Rect;
use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task, futures::check_ready};
use bevy::window::PrimaryWindow;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::dimensions::{
    HERO_GRID_UNITS, HERO_IMAGE_SIZE, HERO_TEXTURE_SIZE, UI_CONTENT_GAP, UI_CONTROL_GAP,
    UI_FIELD_GAP, UI_FORM_LEFT_COLUMN_FLEX, UI_FORM_RIGHT_COLUMN_FLEX, UI_MULTI_SELECT_WIDTH,
    UI_PANEL_GAP, UI_PORTRAIT_SCREEN_PADDING, UI_SCREEN_PADDING, UI_SECTION_GAP,
    UI_WIDE_CONTENT_WIDTH,
};
use crate::input::selection::PrimaryInputDevice;
use crate::settings_transition::{SettingsTransition, request_or_set};
use crate::storage::LocalStorage;
use crate::storage::provider_sync::{
    ProviderSyncTaskState, test_provider as test_provider_connection,
};
use crate::storage::providers::{
    RomProvider, RomProviderFormInput, RomProviderSourceKind, new_provider,
    provider_from_form_input,
};
use crate::ui_elements::action_hint::action_hints_with_labels;
use crate::ui_elements::button::button;
use crate::ui_elements::description::description;
use crate::ui_elements::file_picker::{UiFilePicker, directory_picker_with_value};
use crate::ui_elements::info_message::{InfoMessage, info_message, set_latest_info_message};
use crate::ui_elements::interactions::{
    ActivatedUiElement, DefaultFocusTarget, IgnorePicking, InitialFocus, UI_FOCUS_NONE, UiFocusId,
    UiFocusNav, UiFocusNavIds, UiMultiSelect, UiSchedule, UiTextInput,
};
use crate::ui_elements::multi_select::{MultiSelectConfig, multi_select_with_width};
use crate::ui_elements::responsive::{
    ResponsiveButtonRow, ResponsiveColumns, ResponsiveFieldRow, ResponsiveFlexWidth,
    ResponsiveLandscapeOnly, ResponsivePortraitOnly, ResponsiveScreenPadding,
};
use crate::ui_elements::scroll_view::{ScrollViewConfig, flow_scroll_view, scroll_view};
use crate::ui_elements::settings_header::settings_header;
use crate::ui_elements::text_input::text_input_with_value_width;
use crate::ui_elements::theme::UiThemeImageColor;
const CLOUD_SYNC_HERO_X: f32 = 1.0;
const CLOUD_SYNC_HERO_Y: f32 = 0.0;

const FIELD_NAME: u8 = 0;
const FIELD_LOCAL_DIR: u8 = 2;
const FIELD_REMOTE_FILE_URL: u8 = 3;
const FIELD_API_URL: u8 = 4;
const FIELD_DOWNLOAD_URL: u8 = 5;
const FIELD_ITEMS_PATH: u8 = 6;
const FIELD_PAGE_COUNT_PATH: u8 = 7;
const FIELD_PAGE_PARAM: u8 = 8;
const FIELD_MAX_PAGES: u8 = 9;
const FIELD_ID_PATH: u8 = 10;
const FIELD_FILENAME_PATH: u8 = 11;
const FIELD_NAME_PATH: u8 = 12;
const FIELD_AUTHOR_PATH: u8 = 13;
const FIELD_LICENSE_PATH: u8 = 14;

const SELECT_STATUS: u8 = 0;
const SELECT_PRIORITY: u8 = 1;
const SELECT_TYPE: u8 = 2;
const SELECT_PAGINATION: u8 = 3;

const SECTION_LOCAL_DIR: u8 = 0;
const SECTION_REMOTE_FILE: u8 = 1;
const SECTION_REMOTE_API: u8 = 2;
const SECTION_PAGINATION: u8 = 3;

const TARGET_NAME: u16 = 0;
const TARGET_STATUS: u16 = 1;
const TARGET_PRIORITY: u16 = 2;
const TARGET_TYPE: u16 = 3;
const TARGET_LOCAL_DIR: u16 = 4;
const TARGET_REMOTE_FILE_URL: u16 = 5;
const TARGET_TEST: u16 = 6;
const TARGET_SAVE: u16 = 7;
const TARGET_API_URL: u16 = 8;
const TARGET_DOWNLOAD_URL: u16 = 9;
const TARGET_ITEMS_PATH: u16 = 10;
const TARGET_PAGINATION: u16 = 11;
const TARGET_PAGE_COUNT: u16 = 12;
const TARGET_PAGE_PARAM: u16 = 13;
const TARGET_ID_PATH: u16 = 14;
const TARGET_FILENAME_PATH: u16 = 15;
const TARGET_NAME_PATH: u16 = 16;
const TARGET_AUTHOR_PATH: u16 = 17;
const TARGET_LICENSE_PATH: u16 = 18;
const TARGET_MAX_PAGES: u16 = 19;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct ProviderTextField {
    field: u8,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct ProviderFilePicker {
    field: u8,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct ProviderSelect {
    field: u8,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct ProviderConditionalSection {
    section: u8,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct ProviderSaveButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct ProviderTestButton;

#[derive(Default, Resource)]
struct ProviderTestConnectionState {
    task: Option<Task<Result<String, String>>>,
}

#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct RomProviderEditTarget {
    pub provider_index: Option<usize>,
}

pub struct RomProviderScenePlugin;

impl Plugin for RomProviderScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RomProviderEditTarget>()
            .init_resource::<ProviderTestConnectionState>()
            .add_systems(
                OnEnter(AppState::RomProvider),
                (reset_provider_test_connection, spawn_rom_provider_scene).chain(),
            )
            .add_systems(
                Update,
                sync_provider_conditional_sections
                    .run_if(in_state(AppState::RomProvider))
                    .before(UiSchedule::Focus),
            )
            .add_systems(
                Update,
                finish_provider_test_connection.run_if(in_state(AppState::RomProvider)),
            )
            .add_systems(
                OnExit(AppState::RomProvider),
                reset_provider_test_connection,
            )
            .add_observer(handle_provider_activation);
    }
}

fn spawn_rom_provider_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    primary_input: Res<PrimaryInputDevice>,
    storage: Res<LocalStorage>,
    target: Res<RomProviderEditTarget>,
) {
    let provider = target
        .provider_index
        .and_then(|index| storage.data.providers.get(index))
        .cloned()
        .unwrap_or_else(new_provider);
    commands.spawn_scene(rom_provider_scene(
        &assets,
        *theme,
        &primary_input,
        &storage,
        &provider,
    ));
}

fn handle_provider_activation(
    activated: On<Add, ActivatedUiElement>,
    save_buttons: Query<(), With<ProviderSaveButton>>,
    test_buttons: Query<(), With<ProviderTestButton>>,
    text_fields: Query<(Entity, &ProviderTextField, &UiTextInput)>,
    file_pickers: Query<(Entity, &ProviderFilePicker, &UiFilePicker)>,
    selects: Query<(Entity, &ProviderSelect, &UiMultiSelect)>,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
    mut storage: ResMut<LocalStorage>,
    target: Res<RomProviderEditTarget>,
    state: Res<State<AppState>>,
    mut test_state: ResMut<ProviderTestConnectionState>,
    mut sync_state: ResMut<ProviderSyncTaskState>,
    mut next_state: ResMut<NextState<AppState>>,
    mut transition: ResMut<SettingsTransition>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
) {
    if *state.get() != AppState::RomProvider {
        return;
    }

    let entity = activated.entity;
    if save_buttons.get(entity).is_ok() {
        match provider_from_form(
            &text_fields,
            &file_pickers,
            &selects,
            &nodes,
            &parents,
            target.provider_index,
            &storage,
        ) {
            Ok(provider) => {
                let is_new_provider = target.provider_index.is_none();
                let provider_for_sync = provider.clone();
                if let Some(index) = target.provider_index {
                    if let Some(existing) = storage.data.providers.get_mut(index) {
                        *existing = provider;
                    }
                } else {
                    storage.data.providers.push(provider);
                }

                if let Err(error) = storage.save_providers() {
                    eprintln!("failed to save ROM providers: {error}");
                    set_latest_info_message(&mut messages, "ROM provider could not be saved.");
                    return;
                }
                if is_new_provider && !sync_state.start_provider_sync(provider_for_sync) {
                    eprintln!(
                        "ROM provider was saved, but initial sync could not start because another provider sync is already running."
                    );
                }
                request_or_set(
                    &mut transition,
                    &mut next_state,
                    AppState::RomProvider,
                    AppState::Settings,
                );
            }
            Err(error) => set_latest_info_message(&mut messages, &error.to_string()),
        }
    } else if test_buttons.get(entity).is_ok() {
        if test_state.task.is_some() {
            set_latest_info_message(&mut messages, "Test connection is already running.");
            return;
        }

        match provider_from_form(
            &text_fields,
            &file_pickers,
            &selects,
            &nodes,
            &parents,
            target.provider_index,
            &storage,
        ) {
            Ok(provider) => {
                set_latest_info_message(&mut messages, "Testing ROM provider connection...");
                test_state.task = Some(IoTaskPool::get().spawn(async move {
                    test_provider_connection(&provider)
                        .map(|result| result.message)
                        .map_err(|error| error.to_string())
                }));
            }
            Err(error) => set_latest_info_message(&mut messages, &error.to_string()),
        }
    }
}

fn finish_provider_test_connection(
    mut state: ResMut<ProviderTestConnectionState>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
) {
    let result = {
        let Some(task) = state.task.as_mut() else {
            return;
        };
        check_ready(task)
    };
    let Some(result) = result else {
        return;
    };
    state.task = None;

    match result {
        Ok(message) => set_latest_info_message(&mut messages, &message),
        Err(message) => set_latest_info_message(&mut messages, &message),
    }
}

fn reset_provider_test_connection(mut state: ResMut<ProviderTestConnectionState>) {
    state.task = None;
}

fn sync_provider_conditional_sections(
    windows: Query<&Window, With<PrimaryWindow>>,
    selects: Query<(Entity, &ProviderSelect, &UiMultiSelect)>,
    mut nodes: ParamSet<(
        Query<&Node>,
        Query<(&ProviderConditionalSection, &mut Node)>,
    )>,
    parents: Query<&ChildOf>,
    mut navs: Query<(Entity, &UiFocusId, &mut UiFocusNav)>,
) {
    let portrait = windows
        .iter()
        .next()
        .is_some_and(|window| window.width() < window.height());
    let (provider_type, pagination_enabled) = {
        let node_query = nodes.p0();
        (
            selected_value(&selects, SELECT_TYPE, &node_query, &parents),
            selected_value(&selects, SELECT_PAGINATION, &node_query, &parents) == 0,
        )
    };

    for (section, mut node) in &mut nodes.p1() {
        node.display = match section.section {
            SECTION_LOCAL_DIR => display_for(provider_type == 0),
            SECTION_REMOTE_FILE => display_for(provider_type == 1),
            SECTION_REMOTE_API => display_for(provider_type == 2),
            SECTION_PAGINATION => display_for(provider_type == 2 && pagination_enabled),
            _ => Display::Flex,
        };
    }

    apply_provider_focus_nav(
        provider_type,
        pagination_enabled,
        portrait,
        &nodes.p0(),
        &parents,
        &mut navs,
    );
}

fn display_for(visible: bool) -> Display {
    if visible {
        Display::Flex
    } else {
        Display::None
    }
}

fn apply_provider_focus_nav(
    provider_type: usize,
    pagination_enabled: bool,
    portrait: bool,
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
    navs: &mut Query<(Entity, &UiFocusId, &mut UiFocusNav)>,
) {
    let target_entities = navs
        .iter()
        .filter(|(entity, _, _)| entity_visible(*entity, nodes, parents))
        .map(|(entity, focus_id, _)| (focus_id.id, entity))
        .collect::<Vec<_>>();
    let target = |id| {
        if id == UI_FOCUS_NONE {
            return Entity::PLACEHOLDER;
        }
        target_entities
            .iter()
            .find_map(|(target_id, entity)| (*target_id == id).then_some(*entity))
            .unwrap_or(Entity::PLACEHOLDER)
    };

    for (_, focus_id, mut nav) in navs.iter_mut() {
        let nav_ids =
            provider_focus_nav_for(focus_id.id, provider_type, pagination_enabled, portrait);
        *nav = UiFocusNav {
            up: target(nav_ids.up),
            right: target(nav_ids.right),
            down: target(nav_ids.down),
            left: target(nav_ids.left),
        };
    }
}

fn provider_focus_nav_for(
    id: u16,
    provider_type: usize,
    pagination_enabled: bool,
    portrait: bool,
) -> UiFocusNavIds {
    if portrait {
        return provider_portrait_focus_nav_for(id, provider_type, pagination_enabled);
    }

    let api_target = if provider_type == 2 {
        TARGET_API_URL
    } else {
        UI_FOCUS_NONE
    };
    let type_down = match provider_type {
        0 => TARGET_LOCAL_DIR,
        1 => TARGET_REMOTE_FILE_URL,
        _ => TARGET_TEST,
    };
    let pagination_down = if pagination_enabled {
        TARGET_PAGE_COUNT
    } else {
        TARGET_ID_PATH
    };
    let max_pages_down = if pagination_enabled {
        TARGET_PAGE_PARAM
    } else {
        TARGET_FILENAME_PATH
    };
    let id_path_up = if pagination_enabled {
        TARGET_PAGE_COUNT
    } else {
        TARGET_PAGINATION
    };
    let filename_path_up = if pagination_enabled {
        TARGET_PAGE_PARAM
    } else {
        TARGET_MAX_PAGES
    };

    match id {
        TARGET_NAME => focus_nav_ids(UI_FOCUS_NONE, api_target, TARGET_STATUS, UI_FOCUS_NONE),
        TARGET_STATUS => focus_nav_ids(TARGET_NAME, api_target, TARGET_PRIORITY, UI_FOCUS_NONE),
        TARGET_PRIORITY => focus_nav_ids(TARGET_STATUS, api_target, TARGET_TYPE, UI_FOCUS_NONE),
        TARGET_TYPE => focus_nav_ids(TARGET_PRIORITY, api_target, type_down, UI_FOCUS_NONE),
        TARGET_LOCAL_DIR => focus_nav_ids(TARGET_TYPE, UI_FOCUS_NONE, TARGET_SAVE, UI_FOCUS_NONE),
        TARGET_REMOTE_FILE_URL => {
            focus_nav_ids(TARGET_TYPE, UI_FOCUS_NONE, TARGET_SAVE, UI_FOCUS_NONE)
        }
        TARGET_TEST => focus_nav_ids(TARGET_TYPE, TARGET_SAVE, UI_FOCUS_NONE, UI_FOCUS_NONE),
        TARGET_SAVE => focus_nav_ids(TARGET_TYPE, api_target, UI_FOCUS_NONE, TARGET_TEST),
        TARGET_API_URL => focus_nav_ids(
            UI_FOCUS_NONE,
            UI_FOCUS_NONE,
            TARGET_DOWNLOAD_URL,
            TARGET_NAME,
        ),
        TARGET_DOWNLOAD_URL => focus_nav_ids(
            TARGET_API_URL,
            UI_FOCUS_NONE,
            TARGET_ITEMS_PATH,
            TARGET_NAME,
        ),
        TARGET_ITEMS_PATH => focus_nav_ids(
            TARGET_DOWNLOAD_URL,
            UI_FOCUS_NONE,
            TARGET_PAGINATION,
            TARGET_NAME,
        ),
        TARGET_PAGINATION => focus_nav_ids(
            TARGET_ITEMS_PATH,
            TARGET_MAX_PAGES,
            pagination_down,
            TARGET_NAME,
        ),
        TARGET_MAX_PAGES => focus_nav_ids(
            TARGET_ITEMS_PATH,
            UI_FOCUS_NONE,
            max_pages_down,
            TARGET_PAGINATION,
        ),
        TARGET_PAGE_COUNT => focus_nav_ids(
            TARGET_PAGINATION,
            TARGET_PAGE_PARAM,
            TARGET_ID_PATH,
            TARGET_NAME,
        ),
        TARGET_PAGE_PARAM => focus_nav_ids(
            TARGET_MAX_PAGES,
            UI_FOCUS_NONE,
            TARGET_FILENAME_PATH,
            TARGET_PAGE_COUNT,
        ),
        TARGET_ID_PATH => focus_nav_ids(
            id_path_up,
            TARGET_FILENAME_PATH,
            TARGET_NAME_PATH,
            TARGET_NAME,
        ),
        TARGET_FILENAME_PATH => focus_nav_ids(
            filename_path_up,
            UI_FOCUS_NONE,
            TARGET_AUTHOR_PATH,
            TARGET_ID_PATH,
        ),
        TARGET_NAME_PATH => focus_nav_ids(
            TARGET_ID_PATH,
            TARGET_AUTHOR_PATH,
            TARGET_LICENSE_PATH,
            TARGET_NAME,
        ),
        TARGET_AUTHOR_PATH => focus_nav_ids(
            TARGET_FILENAME_PATH,
            UI_FOCUS_NONE,
            TARGET_LICENSE_PATH,
            TARGET_NAME_PATH,
        ),
        TARGET_LICENSE_PATH => {
            focus_nav_ids(TARGET_NAME_PATH, UI_FOCUS_NONE, UI_FOCUS_NONE, TARGET_NAME)
        }
        _ => focus_nav_ids(UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE),
    }
}

fn provider_portrait_focus_nav_for(
    id: u16,
    provider_type: usize,
    pagination_enabled: bool,
) -> UiFocusNavIds {
    let mut order = vec![TARGET_NAME, TARGET_STATUS, TARGET_PRIORITY, TARGET_TYPE];
    match provider_type {
        0 => order.push(TARGET_LOCAL_DIR),
        1 => order.push(TARGET_REMOTE_FILE_URL),
        _ => {}
    }
    order.extend([TARGET_TEST, TARGET_SAVE]);
    if provider_type == 2 {
        order.extend([
            TARGET_API_URL,
            TARGET_DOWNLOAD_URL,
            TARGET_ITEMS_PATH,
            TARGET_PAGINATION,
            TARGET_MAX_PAGES,
        ]);
        if pagination_enabled {
            order.extend([TARGET_PAGE_COUNT, TARGET_PAGE_PARAM]);
        }
        order.extend([
            TARGET_ID_PATH,
            TARGET_FILENAME_PATH,
            TARGET_NAME_PATH,
            TARGET_AUTHOR_PATH,
            TARGET_LICENSE_PATH,
        ]);
    }

    let Some(index) = order.iter().position(|target| *target == id) else {
        return focus_nav_ids(UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE);
    };
    focus_nav_ids(
        index
            .checked_sub(1)
            .and_then(|previous| order.get(previous).copied())
            .unwrap_or(UI_FOCUS_NONE),
        UI_FOCUS_NONE,
        order.get(index + 1).copied().unwrap_or(UI_FOCUS_NONE),
        UI_FOCUS_NONE,
    )
}

fn rom_provider_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    primary_input: &PrimaryInputDevice,
    storage: &LocalStorage,
    provider: &RomProvider,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let left_font = font.clone();
    let right_font = font.clone();
    let heroes = assets.heroes.clone();

    bsn! {
        #RomProviderScene
        DespawnOnExit::<AppState>(AppState::RomProvider)
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
                    settings_header(font.clone(), assets.icons.clone(), theme, "ROM Provider"),
                    provider_form(left_font, right_font, heroes, theme, provider.clone()),
                    info_message(font.clone(), theme, "", false),
                    action_hints_with_labels(font, assets.icons.clone(), theme, storage, primary_input, "Back", "Select"),
                ]
            ),
        ]
    }
}

fn provider_form(
    left_font: Handle<Font>,
    right_font: Handle<Font>,
    heroes: Handle<Image>,
    theme: ActiveTheme,
    provider: RomProvider,
) -> impl Scene {
    let left_provider = provider.clone();
    let right_provider = provider.clone();
    let landscape_left_provider = provider.clone();
    let landscape_right_provider = provider;
    let landscape_left_font = left_font.clone();
    let landscape_right_font = right_font.clone();
    let landscape_heroes = heroes.clone();

    bsn! {
        Node {
            width: percent(100),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_height: px(0.0),
        }
        Children [
            (
                Node {
                    width: percent(100),
                    height: percent(100),
                    min_height: px(0.0),
                    display: Display::None,
                }
                ResponsiveLandscapeOnly
                Children [
                    provider_landscape_body(landscape_left_font, landscape_right_font, landscape_heroes, theme, landscape_left_provider, landscape_right_provider),
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
                        #ProviderBodyScrollBar
                        flow_scroll_view(
                            theme,
                            #ProviderBodyScrollBar,
                            ScrollViewConfig {
                                width: percent(100),
                                min_height: px(0.0),
                                thumb_height: 120.0,
                            },
                            move |_| provider_body(left_font, right_font, heroes, theme, left_provider, right_provider)
                        )
                    )
                ]
            ),
        ]
    }
}

fn provider_landscape_body(
    left_font: Handle<Font>,
    right_font: Handle<Font>,
    heroes: Handle<Image>,
    theme: ActiveTheme,
    left_provider: RomProvider,
    right_provider: RomProvider,
) -> impl Scene {
    let button_font = left_font.clone();

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
                Node {
                    width: px(0.0),
                    min_width: px(0.0),
                    min_height: px(0.0),
                    flex_grow: UI_FORM_LEFT_COLUMN_FLEX,
                    flex_shrink: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(UI_CONTROL_GAP),
                }
                Children [
                    (
                        Node {
                            width: percent(100),
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            min_height: px(0.0),
                        }
                        Children [
                            (
                                #ProviderLeftScrollBar
                                scroll_view(
                                    theme,
                                    #ProviderLeftScrollBar,
                                    ScrollViewConfig {
                                        width: percent(100),
                                        min_height: px(0.0),
                                        thumb_height: 120.0,
                                    },
                                    move |_| provider_left_column(left_font, heroes, theme, left_provider)
                                )
                            ),
                        ]
                    ),
                    provider_action_buttons(button_font, theme),
                ]
            ),
            (
                Node {
                    width: px(0.0),
                    min_width: px(0.0),
                    min_height: px(0.0),
                    flex_grow: UI_FORM_RIGHT_COLUMN_FLEX,
                    flex_shrink: 1.0,
                }
                Children [
                    (
                        #ProviderRightScrollBar
                        scroll_view(
                            theme,
                            #ProviderRightScrollBar,
                            ScrollViewConfig {
                                width: percent(100),
                                min_height: px(0.0),
                                thumb_height: 120.0,
                            },
                            move |_| provider_right_column(right_font, theme, right_provider)
                        )
                    ),
                ]
            ),
        ]
    }
}

fn provider_body(
    left_font: Handle<Font>,
    right_font: Handle<Font>,
    heroes: Handle<Image>,
    theme: ActiveTheme,
    left_provider: RomProvider,
    right_provider: RomProvider,
) -> impl Scene {
    let button_font = left_font.clone();

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
                    width: px(0.0),
                    min_width: px(0.0),
                    min_height: px(0.0),
                    flex_grow: UI_FORM_LEFT_COLUMN_FLEX,
                    flex_shrink: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(UI_CONTROL_GAP),
                }
                ResponsiveFlexWidth { landscape: UI_FORM_LEFT_COLUMN_FLEX }
                Children [
                    provider_left_column(left_font, heroes, theme, left_provider),
                    provider_action_buttons(button_font, theme),
                ]
            ),
            (
                Node {
                    width: px(0.0),
                    min_width: px(0.0),
                    min_height: px(0.0),
                    flex_grow: UI_FORM_RIGHT_COLUMN_FLEX,
                    flex_shrink: 1.0,
                }
                ResponsiveFlexWidth { landscape: UI_FORM_RIGHT_COLUMN_FLEX }
                Children [
                    provider_right_column(right_font, theme, right_provider),
                ]
            ),
        ]
    }
}

fn provider_left_column(
    font: Handle<Font>,
    heroes: Handle<Image>,
    theme: ActiveTheme,
    provider: RomProvider,
) -> impl Scene {
    let friendly_name = provider.friendly_name.clone();
    let local_dir = local_dir_value(&provider);
    let remote_file_url = provider.remote_file_url.clone().unwrap_or_default();

    bsn! {
        Node {
            width: percent(100),
            min_width: px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_CONTROL_GAP),
            padding: UiRect {
                left: px(0.0),
                right: px(22.0),
                top: px(0.0),
                bottom: px(12.0),
            },
        }
        Children [
            labeled_input(font.clone(), theme, "Display name", "Enter your text here...", friendly_name, FIELD_NAME, TARGET_NAME, true),
            labeled_select(font.clone(), theme, "Status", status_config(provider.enabled), SELECT_STATUS, TARGET_STATUS),
            labeled_select(font.clone(), theme, "Priority", priority_config(provider.priority), SELECT_PRIORITY, TARGET_PRIORITY),
            labeled_select(font.clone(), theme, "Type", provider_type_config(&provider), SELECT_TYPE, TARGET_TYPE),
            (
                ProviderConditionalSection { section: SECTION_LOCAL_DIR }
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                }
                Children [
                    labeled_directory_picker(font.clone(), theme, "Local directory", "Choose directory...", local_dir, FIELD_LOCAL_DIR, TARGET_LOCAL_DIR),
                ]
            ),
            (
                ProviderConditionalSection { section: SECTION_REMOTE_FILE }
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                }
                Children [
                    labeled_input(font.clone(), theme, "Remote file URL", "Enter URL...", remote_file_url, FIELD_REMOTE_FILE_URL, TARGET_REMOTE_FILE_URL, false),
                ]
            ),
            provider_hero_image(heroes, theme),
        ]
    }
}

fn provider_action_buttons(font: Handle<Font>, theme: ActiveTheme) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            column_gap: px(UI_SECTION_GAP),
            padding: UiRect::right(px(22.0)),
        }
        ResponsiveButtonRow { gap: UI_SECTION_GAP }
        Children [
            (
                button(font.clone(), "Test Connection", theme, UiFocusNav::default())
                ProviderTestButton
                UiFocusId { id: TARGET_TEST }
                UiFocusNavIds { up: TARGET_REMOTE_FILE_URL, right: TARGET_SAVE, down: UI_FOCUS_NONE, left: UI_FOCUS_NONE }
            ),
            (
                button(font, "Save", theme, UiFocusNav::default())
                ProviderSaveButton
                UiFocusId { id: TARGET_SAVE }
                UiFocusNavIds { up: TARGET_REMOTE_FILE_URL, right: TARGET_API_URL, down: UI_FOCUS_NONE, left: TARGET_TEST }
            ),
        ]
    }
}

fn provider_right_column(
    font: Handle<Font>,
    theme: ActiveTheme,
    provider: RomProvider,
) -> impl Scene {
    let remote_api = provider.remote_api.as_ref();
    let pagination = remote_api.and_then(|api| api.pagination.as_ref());
    let response_items = remote_api.map(|api| &api.response_items);
    let api_url = remote_api
        .map(|api| api.get_url.clone())
        .unwrap_or_default();
    let download_url = remote_api
        .map(|api| api.download_url.clone())
        .unwrap_or_default();
    let items_path = response_items
        .map(|items| items.items_json_path.clone())
        .unwrap_or_else(|| "$".to_string());
    let page_count_path = pagination
        .map(|value| value.page_count_json_path.clone())
        .unwrap_or_default();
    let page_param = pagination
        .map(|value| value.query_page.clone())
        .unwrap_or_default();
    let max_pages = pagination
        .and_then(|value| value.max_pages)
        .map(|value| value.to_string())
        .unwrap_or_default();
    let id_path = response_items
        .map(|items| items.item_id_json_path.clone())
        .unwrap_or_default();
    let filename_path = response_items
        .map(|items| items.item_filename_json_path.clone())
        .unwrap_or_default();
    let name_path = response_items
        .and_then(|items| items.item_name_json_path.clone())
        .unwrap_or_default();
    let author_path = response_items
        .and_then(|items| items.item_author_json_path.clone())
        .unwrap_or_default();
    let license_path = response_items
        .and_then(|items| items.item_license_json_path.clone())
        .unwrap_or_default();

    bsn! {
        Node {
            width: percent(100),
            min_width: px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_CONTROL_GAP),
            padding: UiRect {
                left: px(0.0),
                right: px(22.0),
                top: px(0.0),
                bottom: px(80.0),
            },
        }
        Children [
            (
                ProviderConditionalSection { section: SECTION_REMOTE_API }
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(UI_CONTROL_GAP),
                }
                Children [
                    labeled_input(font.clone(), theme, "API URL", "Enter URL...", api_url, FIELD_API_URL, TARGET_API_URL, false),
                    labeled_input(font.clone(), theme, "Download URL (placeholders may include {id} or {filename})", "Enter URL...", download_url, FIELD_DOWNLOAD_URL, TARGET_DOWNLOAD_URL, false),
                    description(font.clone(), theme, "For remaining properties, path items use JSONPath syntax. For ROM item properties, include the [*] marker."),
                    labeled_input(font.clone(), theme, "ROM items path", "Enter path...", items_path, FIELD_ITEMS_PATH, TARGET_ITEMS_PATH, false),
                    pagination_controls(
                        font.clone(),
                        theme,
                        pagination_config(pagination.is_some()),
                        max_pages,
                    ),
                    (
                        ProviderConditionalSection { section: SECTION_PAGINATION }
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                        }
                        Children [
                            two_column_fields(
                                font.clone(),
                                theme,
                                "Pagination count path",
                                "Enter path...",
                                page_count_path,
                                FIELD_PAGE_COUNT_PATH,
                                TARGET_PAGE_COUNT,
                                "Pagination page param",
                                "Enter param...",
                                page_param,
                                FIELD_PAGE_PARAM,
                                TARGET_PAGE_PARAM,
                            ),
                        ]
                    ),
                    two_column_fields(
                        font.clone(),
                        theme,
                        "ROM Item ID path",
                        "Enter path...",
                        id_path,
                        FIELD_ID_PATH,
                        TARGET_ID_PATH,
                        "ROM file name path",
                        "Enter path...",
                        filename_path,
                        FIELD_FILENAME_PATH,
                        TARGET_FILENAME_PATH,
                    ),
                    two_column_fields(
                        font.clone(),
                        theme,
                        "ROM name path",
                        "Enter path...",
                        name_path,
                        FIELD_NAME_PATH,
                        TARGET_NAME_PATH,
                        "ROM author path",
                        "Enter path...",
                        author_path,
                        FIELD_AUTHOR_PATH,
                        TARGET_AUTHOR_PATH,
                    ),
                    labeled_input(font.clone(), theme, "ROM license path", "Enter path...", license_path, FIELD_LICENSE_PATH, TARGET_LICENSE_PATH, false),
                ]
            ),
        ]
    }
}

fn labeled_input(
    font: Handle<Font>,
    theme: ActiveTheme,
    label: &'static str,
    placeholder: &'static str,
    value: String,
    field: u8,
    target: u16,
    initial_focus: bool,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_FIELD_GAP),
        }
        Children [
            description(font.clone(), theme, label),
            (
                text_input_with_value_width(font, placeholder, value, theme, UiFocusNav::default(), percent(100))
                ProviderTextField { field: {field} }
                UiFocusId { id: target }
                UiFocusNavIds { up: {provider_focus_nav(target).up}, right: {provider_focus_nav(target).right}, down: {provider_focus_nav(target).down}, left: {provider_focus_nav(target).left} }
                InitialFocus { enabled: initial_focus }
                DefaultFocusTarget
            ),
        ]
    }
}

fn labeled_directory_picker(
    font: Handle<Font>,
    theme: ActiveTheme,
    label: &'static str,
    placeholder: &'static str,
    value: String,
    field: u8,
    target: u16,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_FIELD_GAP),
        }
        Children [
            description(font.clone(), theme, label),
            (
                directory_picker_with_value(font, placeholder, value, theme, UiFocusNav::default())
                ProviderFilePicker { field: {field} }
                UiFocusId { id: target }
                UiFocusNavIds { up: {provider_focus_nav(target).up}, right: {provider_focus_nav(target).right}, down: {provider_focus_nav(target).down}, left: {provider_focus_nav(target).left} }
            ),
        ]
    }
}

fn labeled_select(
    font: Handle<Font>,
    theme: ActiveTheme,
    label: &'static str,
    config: MultiSelectConfig,
    field: u8,
    target: u16,
) -> impl Scene {
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
                multi_select_with_width(font, theme, config, px(UI_MULTI_SELECT_WIDTH))
                ProviderSelect { field: {field} }
                UiFocusId { id: target }
                UiFocusNavIds { up: {provider_focus_nav(target).up}, right: {provider_focus_nav(target).right}, down: {provider_focus_nav(target).down}, left: {provider_focus_nav(target).left} }
            ),
        ]
    }
}

fn pagination_controls(
    font: Handle<Font>,
    theme: ActiveTheme,
    pagination_config: MultiSelectConfig,
    max_pages: String,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            column_gap: px(UI_PANEL_GAP),
        }
        ResponsiveColumns { gap: UI_PANEL_GAP }
        Children [
            (
                Node {
                    width: px(0.0),
                    min_width: px(0.0),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                }
                ResponsiveFlexWidth { landscape: 1.0 }
                Children [
                    (
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(UI_FIELD_GAP),
                        }
                        Children [
                            description(font.clone(), theme, "Pagination"),
                            (
                                multi_select_with_width(font.clone(), theme, pagination_config, percent(100))
                                ProviderSelect { field: SELECT_PAGINATION }
                                UiFocusId { id: TARGET_PAGINATION }
                                UiFocusNavIds { up: {provider_focus_nav(TARGET_PAGINATION).up}, right: {provider_focus_nav(TARGET_PAGINATION).right}, down: {provider_focus_nav(TARGET_PAGINATION).down}, left: {provider_focus_nav(TARGET_PAGINATION).left} }
                            ),
                        ]
                    ),
                ]
            ),
            (
                Node {
                    width: px(0.0),
                    min_width: px(0.0),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                }
                ResponsiveFlexWidth { landscape: 1.0 }
                Children [
                    labeled_input(font, theme, "Max pages", "Enter number...", max_pages, FIELD_MAX_PAGES, TARGET_MAX_PAGES, false),
                ]
            ),
        ]
    }
}

fn two_column_fields(
    font: Handle<Font>,
    theme: ActiveTheme,
    left_label: &'static str,
    left_placeholder: &'static str,
    left_value: String,
    left_field: u8,
    left_target: u16,
    right_label: &'static str,
    right_placeholder: &'static str,
    right_value: String,
    right_field: u8,
    right_target: u16,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            column_gap: px(UI_PANEL_GAP),
        }
        ResponsiveColumns { gap: UI_PANEL_GAP }
        Children [
            (
                Node {
                    width: px(0.0),
                    min_width: px(0.0),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                }
                ResponsiveFlexWidth { landscape: 1.0 }
                Children [
                    labeled_input(font.clone(), theme, left_label, left_placeholder, left_value, left_field, left_target, false),
                ]
            ),
            (
                Node {
                    width: px(0.0),
                    min_width: px(0.0),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                }
                ResponsiveFlexWidth { landscape: 1.0 }
                Children [
                    labeled_input(font, theme, right_label, right_placeholder, right_value, right_field, right_target, false),
                ]
            ),
        ]
    }
}

fn provider_hero_image(image: Handle<Image>, theme: ActiveTheme) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            min_height: px(HERO_IMAGE_SIZE),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::vertical(px(12.0)),
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
                    rect: {Some(hero_grid_rect(CLOUD_SYNC_HERO_X, CLOUD_SYNC_HERO_Y))},
                }
                UiThemeImageColor::Primary
                IgnorePicking
            ),
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

fn provider_focus_nav(id: u16) -> UiFocusNavIds {
    match id {
        TARGET_NAME => focus_nav_ids(UI_FOCUS_NONE, TARGET_API_URL, TARGET_STATUS, UI_FOCUS_NONE),
        TARGET_STATUS => focus_nav_ids(TARGET_NAME, TARGET_API_URL, TARGET_PRIORITY, UI_FOCUS_NONE),
        TARGET_PRIORITY => focus_nav_ids(TARGET_STATUS, TARGET_API_URL, TARGET_TYPE, UI_FOCUS_NONE),
        TARGET_TYPE => focus_nav_ids(
            TARGET_PRIORITY,
            TARGET_API_URL,
            TARGET_LOCAL_DIR,
            UI_FOCUS_NONE,
        ),
        TARGET_LOCAL_DIR => focus_nav_ids(
            TARGET_TYPE,
            TARGET_API_URL,
            TARGET_REMOTE_FILE_URL,
            UI_FOCUS_NONE,
        ),
        TARGET_REMOTE_FILE_URL => {
            focus_nav_ids(TARGET_LOCAL_DIR, TARGET_API_URL, TARGET_SAVE, UI_FOCUS_NONE)
        }
        TARGET_TEST => focus_nav_ids(
            TARGET_REMOTE_FILE_URL,
            TARGET_SAVE,
            UI_FOCUS_NONE,
            UI_FOCUS_NONE,
        ),
        TARGET_SAVE => focus_nav_ids(
            TARGET_REMOTE_FILE_URL,
            TARGET_API_URL,
            UI_FOCUS_NONE,
            TARGET_TEST,
        ),
        TARGET_API_URL => focus_nav_ids(
            UI_FOCUS_NONE,
            UI_FOCUS_NONE,
            TARGET_DOWNLOAD_URL,
            TARGET_NAME,
        ),
        TARGET_DOWNLOAD_URL => focus_nav_ids(
            TARGET_API_URL,
            UI_FOCUS_NONE,
            TARGET_ITEMS_PATH,
            TARGET_NAME,
        ),
        TARGET_ITEMS_PATH => focus_nav_ids(
            TARGET_DOWNLOAD_URL,
            UI_FOCUS_NONE,
            TARGET_PAGINATION,
            TARGET_NAME,
        ),
        TARGET_PAGINATION => focus_nav_ids(
            TARGET_ITEMS_PATH,
            TARGET_MAX_PAGES,
            TARGET_PAGE_COUNT,
            TARGET_NAME,
        ),
        TARGET_MAX_PAGES => focus_nav_ids(
            TARGET_ITEMS_PATH,
            UI_FOCUS_NONE,
            TARGET_PAGE_PARAM,
            TARGET_PAGINATION,
        ),
        TARGET_PAGE_COUNT => focus_nav_ids(
            TARGET_PAGINATION,
            TARGET_PAGE_PARAM,
            TARGET_ID_PATH,
            TARGET_NAME,
        ),
        TARGET_PAGE_PARAM => focus_nav_ids(
            TARGET_MAX_PAGES,
            UI_FOCUS_NONE,
            TARGET_FILENAME_PATH,
            TARGET_PAGE_COUNT,
        ),
        TARGET_ID_PATH => focus_nav_ids(
            TARGET_PAGE_COUNT,
            TARGET_FILENAME_PATH,
            TARGET_NAME_PATH,
            TARGET_NAME,
        ),
        TARGET_FILENAME_PATH => focus_nav_ids(
            TARGET_PAGE_PARAM,
            UI_FOCUS_NONE,
            TARGET_AUTHOR_PATH,
            TARGET_ID_PATH,
        ),
        TARGET_NAME_PATH => focus_nav_ids(
            TARGET_ID_PATH,
            TARGET_AUTHOR_PATH,
            TARGET_LICENSE_PATH,
            TARGET_NAME,
        ),
        TARGET_AUTHOR_PATH => focus_nav_ids(
            TARGET_FILENAME_PATH,
            UI_FOCUS_NONE,
            TARGET_LICENSE_PATH,
            TARGET_NAME_PATH,
        ),
        TARGET_LICENSE_PATH => {
            focus_nav_ids(TARGET_NAME_PATH, UI_FOCUS_NONE, UI_FOCUS_NONE, TARGET_NAME)
        }
        _ => focus_nav_ids(UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE),
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

fn provider_from_form(
    text_fields: &Query<(Entity, &ProviderTextField, &UiTextInput)>,
    file_pickers: &Query<(Entity, &ProviderFilePicker, &UiFilePicker)>,
    selects: &Query<(Entity, &ProviderSelect, &UiMultiSelect)>,
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
    provider_index: Option<usize>,
    storage: &LocalStorage,
) -> Result<RomProvider, crate::storage::errors::StorageError> {
    let existing = provider_index.and_then(|index| storage.data.providers.get(index));
    provider_from_form_input(
        existing,
        RomProviderFormInput {
            friendly_name: field_value(text_fields, FIELD_NAME, nodes, parents),
            priority: (selected_value(selects, SELECT_PRIORITY, nodes, parents) + 1).to_string(),
            enabled: selected_value(selects, SELECT_STATUS, nodes, parents) == 0,
            source_kind: match selected_value(selects, SELECT_TYPE, nodes, parents) {
                0 => RomProviderSourceKind::LocalDirectory,
                1 => RomProviderSourceKind::RemoteFile,
                _ => RomProviderSourceKind::RemoteApi,
            },
            local_dir_path: picker_value(file_pickers, FIELD_LOCAL_DIR, nodes, parents),
            remote_file_url: field_value(text_fields, FIELD_REMOTE_FILE_URL, nodes, parents),
            api_url: field_value(text_fields, FIELD_API_URL, nodes, parents),
            download_url: field_value(text_fields, FIELD_DOWNLOAD_URL, nodes, parents),
            items_json_path: field_value(text_fields, FIELD_ITEMS_PATH, nodes, parents),
            pagination_enabled: selected_value(selects, SELECT_PAGINATION, nodes, parents) == 0,
            page_count_json_path: field_value(text_fields, FIELD_PAGE_COUNT_PATH, nodes, parents),
            query_page: field_value(text_fields, FIELD_PAGE_PARAM, nodes, parents),
            max_pages: field_value(text_fields, FIELD_MAX_PAGES, nodes, parents),
            item_id_json_path: field_value(text_fields, FIELD_ID_PATH, nodes, parents),
            item_name_json_path: field_value(text_fields, FIELD_NAME_PATH, nodes, parents),
            item_author_json_path: field_value(text_fields, FIELD_AUTHOR_PATH, nodes, parents),
            item_license_json_path: field_value(text_fields, FIELD_LICENSE_PATH, nodes, parents),
            item_filename_json_path: field_value(text_fields, FIELD_FILENAME_PATH, nodes, parents),
        },
    )
}

fn picker_value(
    file_pickers: &Query<(Entity, &ProviderFilePicker, &UiFilePicker)>,
    field: u8,
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> String {
    file_pickers
        .iter()
        .find_map(|(entity, field_marker, picker)| {
            (field_marker.field == field && entity_visible(entity, nodes, parents))
                .then(|| picker.value.clone())
        })
        .unwrap_or_default()
}

fn field_value(
    text_fields: &Query<(Entity, &ProviderTextField, &UiTextInput)>,
    field: u8,
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> String {
    text_fields
        .iter()
        .find_map(|(entity, text_field, input)| {
            (text_field.field == field && entity_visible(entity, nodes, parents))
                .then(|| input.value.clone())
        })
        .unwrap_or_default()
}

fn selected_value(
    selects: &Query<(Entity, &ProviderSelect, &UiMultiSelect)>,
    field: u8,
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> usize {
    selects
        .iter()
        .find_map(|(entity, select, multi_select)| {
            (select.field == field && entity_visible(entity, nodes, parents))
                .then_some(multi_select.selected)
        })
        .unwrap_or_default()
}

fn entity_visible(entity: Entity, nodes: &Query<&Node>, parents: &Query<&ChildOf>) -> bool {
    let mut current = Some(entity);
    while let Some(entity) = current {
        if nodes
            .get(entity)
            .is_ok_and(|node| node.display == Display::None)
        {
            return false;
        }
        current = parents.get(entity).ok().map(|parent| parent.0);
    }
    true
}

fn status_config(enabled: bool) -> MultiSelectConfig {
    select_config(if enabled { 0 } else { 1 }, vec!["Enabled", "Disabled"])
}

fn priority_config(priority: u8) -> MultiSelectConfig {
    let selected = priority.saturating_sub(1).min(4) as usize;
    select_config(selected, vec!["1", "2", "3", "4", "5"])
}

fn provider_type_config(provider: &RomProvider) -> MultiSelectConfig {
    let selected = if provider.remote_file_url.is_some() {
        1
    } else if provider.remote_api.is_some() {
        2
    } else {
        0
    };
    select_config(
        selected,
        vec!["Local Directory", "Remote File", "Remote API"],
    )
}

fn pagination_config(enabled: bool) -> MultiSelectConfig {
    select_config(if enabled { 0 } else { 1 }, vec!["Enabled", "Disabled"])
}

fn select_config(selected: usize, options: Vec<&'static str>) -> MultiSelectConfig {
    MultiSelectConfig {
        selected,
        options: options.into_iter().map(str::to_string).collect(),
        nav: UiFocusNav::default(),
    }
}

fn local_dir_value(provider: &RomProvider) -> String {
    provider
        .absolute_local_dir_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}
