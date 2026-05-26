use bevy::prelude::*;
use bevy::ui::UiScale;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::{ActiveTheme, ActiveThemeChanged, active_theme_for_setting};
use crate::app_ui_scale::{UI_SCALE_LABELS, apply_ui_scale_setting};
use crate::input::mappings::RuntimeInputMappings;
use crate::storage::LocalStorage;
use crate::ui_elements::action_hint::action_hints_with_labels;
use crate::ui_elements::button::button;
use crate::ui_elements::description::description;
use crate::ui_elements::heading::heading;
use crate::ui_elements::info_message::{InfoMessage, info_message, set_latest_info_message};
use crate::ui_elements::interactions::{
    ActivatedUiElement, DefaultFocusTarget, DisabledUiElement, InitialFocus, UI_FOCUS_NONE,
    UiFocusId, UiFocusNav, UiFocusNavIds, UiMultiSelect,
};
use crate::ui_elements::list_view::{ListColumn, ListRow, ListViewConfig, list_view};
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

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct SettingsSelect {
    field: u8,
}

pub struct SettingsScenePlugin;

impl Plugin for SettingsScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Settings), spawn_settings_scene)
            .add_observer(save_settings_select_on_activation);
    }
}

fn spawn_settings_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    input_mappings: Res<RuntimeInputMappings>,
    storage: Res<LocalStorage>,
) {
    commands.spawn_scene(settings_scene(
        &assets,
        *theme,
        &input_mappings,
        &storage.data.settings,
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
    settings: &crate::storage::data::GeneralSettings,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let left_column_font = font.clone();

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
                                                list_view(font.clone(), theme, rom_storage_list_config())
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
                                                list_view(font.clone(), theme, provider_list_config())
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
                                                        UiFocusId { id: TARGET_PROVIDER_SYNC }
                                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_SYNC).up}, right: {settings_focus_nav(TARGET_PROVIDER_SYNC).right}, down: {settings_focus_nav(TARGET_PROVIDER_SYNC).down}, left: {settings_focus_nav(TARGET_PROVIDER_SYNC).left} }
                                                    ),
                                                    (
                                                        #ProviderDelete
                                                        button(font.clone(), "Delete", theme, UiFocusNav::default())
                                                        UiFocusId { id: TARGET_PROVIDER_DELETE }
                                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_DELETE).up}, right: {settings_focus_nav(TARGET_PROVIDER_DELETE).right}, down: {settings_focus_nav(TARGET_PROVIDER_DELETE).down}, left: {settings_focus_nav(TARGET_PROVIDER_DELETE).left} }
                                                    ),
                                                    (
                                                        #ProviderEdit
                                                        button(font.clone(), "Edit", theme, UiFocusNav::default())
                                                        UiFocusId { id: TARGET_PROVIDER_EDIT }
                                                        UiFocusNavIds { up: {settings_focus_nav(TARGET_PROVIDER_EDIT).up}, right: {settings_focus_nav(TARGET_PROVIDER_EDIT).right}, down: {settings_focus_nav(TARGET_PROVIDER_EDIT).down}, left: {settings_focus_nav(TARGET_PROVIDER_EDIT).left} }
                                                    ),
                                                    (
                                                        #ProviderCreate
                                                        button(font.clone(), "Create", theme, UiFocusNav::default())
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
                    info_message(font.clone(), theme, "", false),
                    action_hints_with_labels(font, assets.icons.clone(), theme, input_mappings, "Back", "Select"),
                ]
            ),
        ]
    }
}

fn settings_left_column(
    font: Handle<Font>,
    theme: ActiveTheme,
    settings: &crate::storage::data::GeneralSettings,
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

fn rom_storage_list_config() -> ListViewConfig {
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
        rows: vec![
            list_row(vec!["ALIEN BARRAGE", "Yesterday 1:32PM", "16 KB"]),
            list_row(vec!["Cabbage Dodge", "Mar 21, 2026", "2 MB"]),
            list_row(vec![
                "Extremely Frustrating Gauntlet",
                "Mar 10, 2025",
                "1072 KB",
            ]),
            list_row(vec![
                "Extremely Frustrating Gauntlet 2",
                "Mar 10, 2025",
                "1072 KB",
            ]),
        ],
    }
}

fn provider_list_config() -> ListViewConfig {
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
        rows: vec![
            list_row(vec!["Homebrew Hub (Built-in)", "Remote", "3"]),
            list_row(vec!["My ROMs", "Local Directory", "1"]),
            list_row(vec!["Validation ROMs", "Remote", "5"]),
        ],
    }
}

fn list_row(cells: Vec<&'static str>) -> ListRow {
    ListRow {
        cells,
        nav: UiFocusNav::default(),
    }
}
