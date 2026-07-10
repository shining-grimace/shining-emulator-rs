use std::fs;

use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;
use bevy::window::PrimaryWindow;
use bevy_midi_graph::{MidiFileSource, MidiGraphAudioContext, Sf2FileSource, WaveFileSource};

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::audio::preset_graph::{
    AudioChannelPreset, AudioPreset, apply_audio_preset_to_playback, default_audio_preset,
    load_audio_preset,
};
use crate::dimensions::{
    UI_BODY_FONT_SIZE, UI_CONTENT_GAP, UI_CONTROL_GAP, UI_INNER_PADDING, UI_PANEL_GAP,
    UI_PORTRAIT_SCREEN_PADDING, UI_SCREEN_PADDING, UI_SCROLL_CONTENT_BOTTOM_PADDING,
    UI_SECTION_GAP, UI_WIDE_CONTENT_WIDTH, UI_WIDE_PRIMARY_COLUMN_PERCENT,
    UI_WIDE_SECONDARY_COLUMN_PERCENT,
};
use crate::input::selection::PrimaryInputDevice;
use crate::settings_transition::SettingsNavigation;
use crate::storage::LocalStorage;
use crate::ui_elements::action_hint::action_hints_with_labels;
use crate::ui_elements::button::button;
use crate::ui_elements::description::description;
use crate::ui_elements::file_picker::{
    UiAudioFilePicker, UiFilePicker, UiFilePickerValue, file_picker_with_value,
};
use crate::ui_elements::info_message::{InfoMessage, info_message_text, set_latest_info_message};
use crate::ui_elements::interactions::{
    ActivatedUiElement, IgnorePicking, InitialFocus, UI_FOCUS_NONE, UiFocusId, UiFocusNav,
    UiFocusNavIds, UiMultiSelect, UiMultiSelectLabel,
};
use crate::ui_elements::multi_select::{MultiSelectConfig, multi_select};
use crate::ui_elements::responsive::{
    ResponsiveColumns, ResponsiveFieldRow, ResponsiveLandscapeOnly, ResponsivePercentWidth,
    ResponsivePortraitOnly, ResponsiveScreenPadding,
};
use crate::ui_elements::scroll_view::{ScrollViewConfig, flow_scroll_view, scroll_view};
use crate::ui_elements::settings_header::settings_header;
use crate::ui_elements::theme::UiThemeTextColor;

const OSCILLATOR_SQUARE: usize = 0;
const OSCILLATOR_BUILT_IN_SAMPLER: usize = 4;
const OSCILLATOR_CUSTOM_SAMPLER: usize = 5;
const AUDIO_OPTION_SILENCE: &str = "Silence";

const FIELD_OSCILLATOR: u8 = 0;
const FIELD_BUILT_IN_SAMPLE: u8 = 1;
const FIELD_MODULATION_A: u8 = 2;
const FIELD_MODULATION_B: u8 = 3;

const SECTION_BUILT_IN_SAMPLE: u8 = 0;
const SECTION_CUSTOM_SAMPLE: u8 = 1;

const TARGET_CH1_OSCILLATOR: u16 = 1;
const TARGET_CH1_BUILT_IN_SAMPLE: u16 = 2;
const TARGET_CH1_CUSTOM_SAMPLE: u16 = 3;
const TARGET_CH1_MOD_A: u16 = 4;
const TARGET_CH1_MOD_B: u16 = 5;
const TARGET_CH2_OSCILLATOR: u16 = 6;
const TARGET_CH2_BUILT_IN_SAMPLE: u16 = 7;
const TARGET_CH2_CUSTOM_SAMPLE: u16 = 8;
const TARGET_CH2_MOD_A: u16 = 9;
const TARGET_CH3_SAMPLE: u16 = 10;
const TARGET_CH4_OSCILLATOR: u16 = 11;
const TARGET_CH4_BUILT_IN_SAMPLE: u16 = 12;
const TARGET_CH4_CUSTOM_SAMPLE: u16 = 13;
const TARGET_SAVE: u16 = 20;
const TARGET_DELETE: u16 = 21;
const TARGET_RESTORE: u16 = 22;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct AudioSelect {
    channel: u8,
    field: u8,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct AudioSamplePicker {
    channel: u8,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct AudioConditionalSection {
    channel: u8,
    section: u8,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct AudioSaveButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct AudioDeleteButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct AudioRestoreButton;

pub struct AudioSettingsScenePlugin;

impl Plugin for AudioSettingsScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::AudioSettings), spawn_audio_settings_scene)
            .add_systems(
                Update,
                sync_audio_conditional_sections.run_if(in_state(AppState::AudioSettings)),
            )
            .add_observer(handle_audio_settings_activation);
    }
}

fn spawn_audio_settings_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    primary_input: Res<PrimaryInputDevice>,
    storage: Res<LocalStorage>,
) {
    commands.spawn_scene(audio_settings_scene(
        &assets,
        *theme,
        &primary_input,
        &storage,
    ));
}

fn handle_audio_settings_activation(
    activated: On<Add, ActivatedUiElement>,
    save_buttons: Query<(), With<AudioSaveButton>>,
    delete_buttons: Query<(), With<AudioDeleteButton>>,
    restore_buttons: Query<(), With<AudioRestoreButton>>,
    mut selects: ParamSet<(
        Query<(&AudioSelect, &UiMultiSelect)>,
        Query<(&AudioSelect, &mut UiMultiSelect, Option<&Children>)>,
    )>,
    mut pickers: ParamSet<(
        Query<(&AudioSamplePicker, &UiFilePicker)>,
        Query<(&AudioSamplePicker, &mut UiFilePicker, Option<&Children>)>,
    )>,
    mut sections: Query<(&AudioConditionalSection, &mut Node)>,
    mut text_queries: ParamSet<(
        Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
        Query<&mut Text, With<UiMultiSelectLabel>>,
        Query<(Entity, &UiFilePickerValue, &mut Text)>,
    )>,
    child_query: Query<&Children>,
    storage: Res<LocalStorage>,
    asset_server: Res<AssetServer>,
    mut audio_context: ResMut<MidiGraphAudioContext>,
    midi_assets: Res<Assets<MidiFileSource>>,
    sf2_assets: Res<Assets<Sf2FileSource>>,
    wave_assets: Res<Assets<WaveFileSource>>,
    mut navigation: SettingsNavigation,
) {
    if navigation.current() != AppState::AudioSettings {
        return;
    }

    let entity = activated.entity;
    if save_buttons.get(entity).is_ok() {
        let preset = audio_preset_from_form(&selects.p0(), &pickers.p0());
        match serde_json::to_string_pretty(&preset)
            .map(|json| fs::write(current_preset_path(&storage), format!("{json}\n")))
        {
            Ok(Ok(())) => match apply_audio_preset_to_playback(
                &preset,
                &asset_server,
                &mut audio_context,
                &midi_assets,
                &sf2_assets,
                &wave_assets,
            ) {
                Ok(()) => navigation.request(AppState::Settings),
                Err(error) => {
                    eprintln!("failed to apply audio preset: {error}");
                    set_audio_preset_apply_error_message(&mut text_queries.p0(), &error);
                }
            },
            Ok(Err(error)) => {
                eprintln!("failed to save audio preset: {error}");
                set_latest_info_message(&mut text_queries.p0(), "Audio preset could not be saved.");
            }
            Err(error) => {
                eprintln!("failed to serialise audio preset: {error}");
                set_latest_info_message(&mut text_queries.p0(), "Audio preset could not be saved.");
            }
        }
    } else if delete_buttons.get(entity).is_ok() {
        if storage.data.settings.audio_preset == 0 {
            set_latest_info_message(
                &mut text_queries.p0(),
                "The default audio preset cannot be deleted.",
            );
            return;
        }
        match fs::remove_file(current_preset_path(&storage)) {
            Ok(()) => set_latest_info_message(&mut text_queries.p0(), "Audio preset deleted."),
            Err(error) => {
                eprintln!("failed to delete audio preset: {error}");
                set_latest_info_message(
                    &mut text_queries.p0(),
                    "Audio preset could not be deleted.",
                );
            }
        }
    } else if restore_buttons.get(entity).is_ok() {
        let preset = default_audio_preset();
        match serde_json::to_string_pretty(&preset)
            .map(|json| fs::write(current_preset_path(&storage), format!("{json}\n")))
        {
            Ok(Ok(())) => match apply_audio_preset_to_playback(
                &preset,
                &asset_server,
                &mut audio_context,
                &midi_assets,
                &sf2_assets,
                &wave_assets,
            ) {
                Ok(()) => {
                    apply_audio_preset_to_selects(
                        &preset,
                        &mut selects.p1(),
                        &mut text_queries.p1(),
                        &child_query,
                    );
                    apply_audio_preset_to_pickers(
                        &preset,
                        &mut pickers.p1(),
                        &mut text_queries.p2(),
                        &child_query,
                    );
                    apply_audio_conditional_sections(&selects.p0(), &mut sections);
                    set_latest_info_message(&mut text_queries.p0(), "Audio preset restored.");
                }
                Err(error) => {
                    eprintln!("failed to apply restored audio preset: {error}");
                    set_audio_preset_apply_error_message(&mut text_queries.p0(), &error);
                }
            },
            Ok(Err(error)) => {
                eprintln!("failed to restore audio preset: {error}");
                set_latest_info_message(
                    &mut text_queries.p0(),
                    "Audio preset could not be restored.",
                );
            }
            Err(error) => {
                eprintln!("failed to serialise audio preset: {error}");
                set_latest_info_message(
                    &mut text_queries.p0(),
                    "Audio preset could not be restored.",
                );
            }
        }
    } else if selects.p0().get(entity).is_ok() {
        apply_audio_conditional_sections(&selects.p0(), &mut sections);
    }
}

fn sync_audio_conditional_sections(
    windows: Query<&Window, With<PrimaryWindow>>,
    selects: Query<(&AudioSelect, &UiMultiSelect)>,
    mut nodes: ParamSet<(Query<&Node>, Query<(&AudioConditionalSection, &mut Node)>)>,
    parents: Query<&ChildOf>,
    mut navs: Query<(Entity, &UiFocusId, &mut UiFocusNav)>,
) {
    let portrait = windows
        .iter()
        .next()
        .is_some_and(|window| window.width() < window.height());
    apply_audio_conditional_sections(&selects, &mut nodes.p1());
    apply_audio_focus_nav(&selects, &nodes.p0(), &parents, &mut navs, portrait);
}

fn apply_audio_conditional_sections(
    selects: &Query<(&AudioSelect, &UiMultiSelect)>,
    sections: &mut Query<(&AudioConditionalSection, &mut Node)>,
) {
    for (section, mut node) in sections {
        let oscillator = selects
            .iter()
            .find(|(select, _)| {
                select.channel == section.channel && select.field == FIELD_OSCILLATOR
            })
            .map(|(_, select)| select.selected)
            .unwrap_or(OSCILLATOR_SQUARE);
        let visible = matches!(
            (section.section, oscillator),
            (SECTION_BUILT_IN_SAMPLE, OSCILLATOR_BUILT_IN_SAMPLER)
                | (SECTION_CUSTOM_SAMPLE, OSCILLATOR_CUSTOM_SAMPLER)
        );
        node.display = display_for(visible);
    }
}

fn display_for(visible: bool) -> Display {
    if visible {
        Display::Flex
    } else {
        Display::None
    }
}

fn apply_audio_focus_nav(
    selects: &Query<(&AudioSelect, &UiMultiSelect)>,
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
    navs: &mut Query<(Entity, &UiFocusId, &mut UiFocusNav)>,
    portrait: bool,
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

    let ch1_oscillator = selected_channel_oscillator(selects, 1);
    let ch2_oscillator = selected_channel_oscillator(selects, 2);
    let ch1_first_detail = channel_detail_target(
        ch1_oscillator,
        TARGET_CH1_BUILT_IN_SAMPLE,
        TARGET_CH1_CUSTOM_SAMPLE,
        TARGET_CH1_MOD_A,
    );
    let ch1_last_detail = channel_detail_target(
        ch1_oscillator,
        TARGET_CH1_BUILT_IN_SAMPLE,
        TARGET_CH1_CUSTOM_SAMPLE,
        TARGET_CH1_OSCILLATOR,
    );
    let ch2_first_detail = channel_detail_target(
        ch2_oscillator,
        TARGET_CH2_BUILT_IN_SAMPLE,
        TARGET_CH2_CUSTOM_SAMPLE,
        TARGET_CH2_MOD_A,
    );
    let ch2_last_detail = channel_detail_target(
        ch2_oscillator,
        TARGET_CH2_BUILT_IN_SAMPLE,
        TARGET_CH2_CUSTOM_SAMPLE,
        TARGET_CH2_OSCILLATOR,
    );
    let ch4_oscillator = selected_channel_oscillator(selects, 4);
    let ch4_first_detail = channel_detail_target(
        ch4_oscillator,
        TARGET_CH4_BUILT_IN_SAMPLE,
        TARGET_CH4_CUSTOM_SAMPLE,
        TARGET_SAVE,
    );
    let ch4_last_detail = channel_detail_target(
        ch4_oscillator,
        TARGET_CH4_BUILT_IN_SAMPLE,
        TARGET_CH4_CUSTOM_SAMPLE,
        TARGET_CH4_OSCILLATOR,
    );

    for (_, focus_id, mut nav) in navs.iter_mut() {
        let nav_ids = audio_focus_nav_for(
            focus_id.id,
            ch1_first_detail,
            ch1_last_detail,
            ch2_first_detail,
            ch2_last_detail,
            ch4_first_detail,
            ch4_last_detail,
            portrait,
        );
        *nav = UiFocusNav {
            up: target(nav_ids.up),
            right: target(nav_ids.right),
            down: target(nav_ids.down),
            left: target(nav_ids.left),
        };
    }
}

fn audio_focus_nav_for(
    id: u16,
    ch1_first_detail: u16,
    ch1_last_detail: u16,
    ch2_first_detail: u16,
    ch2_last_detail: u16,
    ch4_first_detail: u16,
    ch4_last_detail: u16,
    portrait: bool,
) -> UiFocusNavIds {
    if portrait {
        return audio_portrait_focus_nav_for(
            id,
            ch1_first_detail,
            ch1_last_detail,
            ch2_first_detail,
            ch2_last_detail,
            ch4_first_detail,
            ch4_last_detail,
        );
    }

    match id {
        TARGET_CH1_OSCILLATOR => {
            focus_nav_ids(UI_FOCUS_NONE, TARGET_SAVE, ch1_first_detail, UI_FOCUS_NONE)
        }
        TARGET_CH1_BUILT_IN_SAMPLE | TARGET_CH1_CUSTOM_SAMPLE => focus_nav_ids(
            TARGET_CH1_OSCILLATOR,
            TARGET_SAVE,
            TARGET_CH1_MOD_A,
            UI_FOCUS_NONE,
        ),
        TARGET_CH1_MOD_A => focus_nav_ids(
            ch1_last_detail,
            TARGET_SAVE,
            TARGET_CH1_MOD_B,
            UI_FOCUS_NONE,
        ),
        TARGET_CH1_MOD_B => focus_nav_ids(
            TARGET_CH1_MOD_A,
            TARGET_SAVE,
            TARGET_CH2_OSCILLATOR,
            UI_FOCUS_NONE,
        ),
        TARGET_CH2_OSCILLATOR => focus_nav_ids(
            TARGET_CH1_MOD_B,
            TARGET_SAVE,
            ch2_first_detail,
            UI_FOCUS_NONE,
        ),
        TARGET_CH2_BUILT_IN_SAMPLE | TARGET_CH2_CUSTOM_SAMPLE => focus_nav_ids(
            TARGET_CH2_OSCILLATOR,
            TARGET_SAVE,
            TARGET_CH2_MOD_A,
            UI_FOCUS_NONE,
        ),
        TARGET_CH2_MOD_A => focus_nav_ids(
            ch2_last_detail,
            TARGET_SAVE,
            TARGET_CH3_SAMPLE,
            UI_FOCUS_NONE,
        ),
        TARGET_CH3_SAMPLE => focus_nav_ids(
            TARGET_CH2_MOD_A,
            TARGET_SAVE,
            TARGET_CH4_OSCILLATOR,
            UI_FOCUS_NONE,
        ),
        TARGET_CH4_OSCILLATOR => focus_nav_ids(
            TARGET_CH3_SAMPLE,
            TARGET_SAVE,
            ch4_first_detail,
            UI_FOCUS_NONE,
        ),
        TARGET_CH4_BUILT_IN_SAMPLE | TARGET_CH4_CUSTOM_SAMPLE => focus_nav_ids(
            TARGET_CH4_OSCILLATOR,
            TARGET_SAVE,
            UI_FOCUS_NONE,
            UI_FOCUS_NONE,
        ),
        TARGET_SAVE => focus_nav_ids(
            UI_FOCUS_NONE,
            UI_FOCUS_NONE,
            TARGET_DELETE,
            TARGET_CH1_OSCILLATOR,
        ),
        TARGET_DELETE => focus_nav_ids(
            TARGET_SAVE,
            UI_FOCUS_NONE,
            TARGET_RESTORE,
            TARGET_CH1_OSCILLATOR,
        ),
        TARGET_RESTORE => focus_nav_ids(
            TARGET_DELETE,
            UI_FOCUS_NONE,
            UI_FOCUS_NONE,
            TARGET_CH1_OSCILLATOR,
        ),
        _ => focus_nav_ids(UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE),
    }
}

fn audio_portrait_focus_nav_for(
    id: u16,
    ch1_first_detail: u16,
    ch1_last_detail: u16,
    ch2_first_detail: u16,
    ch2_last_detail: u16,
    ch4_first_detail: u16,
    ch4_last_detail: u16,
) -> UiFocusNavIds {
    match id {
        TARGET_CH1_OSCILLATOR => focus_nav_ids(
            UI_FOCUS_NONE,
            UI_FOCUS_NONE,
            ch1_first_detail,
            UI_FOCUS_NONE,
        ),
        TARGET_CH1_BUILT_IN_SAMPLE | TARGET_CH1_CUSTOM_SAMPLE => focus_nav_ids(
            TARGET_CH1_OSCILLATOR,
            UI_FOCUS_NONE,
            TARGET_CH1_MOD_A,
            UI_FOCUS_NONE,
        ),
        TARGET_CH1_MOD_A => focus_nav_ids(
            ch1_last_detail,
            UI_FOCUS_NONE,
            TARGET_CH1_MOD_B,
            UI_FOCUS_NONE,
        ),
        TARGET_CH1_MOD_B => focus_nav_ids(
            TARGET_CH1_MOD_A,
            UI_FOCUS_NONE,
            TARGET_CH2_OSCILLATOR,
            UI_FOCUS_NONE,
        ),
        TARGET_CH2_OSCILLATOR => focus_nav_ids(
            TARGET_CH1_MOD_B,
            UI_FOCUS_NONE,
            ch2_first_detail,
            UI_FOCUS_NONE,
        ),
        TARGET_CH2_BUILT_IN_SAMPLE | TARGET_CH2_CUSTOM_SAMPLE => focus_nav_ids(
            TARGET_CH2_OSCILLATOR,
            UI_FOCUS_NONE,
            TARGET_CH2_MOD_A,
            UI_FOCUS_NONE,
        ),
        TARGET_CH2_MOD_A => focus_nav_ids(
            ch2_last_detail,
            UI_FOCUS_NONE,
            TARGET_CH3_SAMPLE,
            UI_FOCUS_NONE,
        ),
        TARGET_CH3_SAMPLE => focus_nav_ids(
            TARGET_CH2_MOD_A,
            UI_FOCUS_NONE,
            TARGET_CH4_OSCILLATOR,
            UI_FOCUS_NONE,
        ),
        TARGET_CH4_OSCILLATOR => focus_nav_ids(
            TARGET_CH3_SAMPLE,
            UI_FOCUS_NONE,
            ch4_first_detail,
            UI_FOCUS_NONE,
        ),
        TARGET_CH4_BUILT_IN_SAMPLE | TARGET_CH4_CUSTOM_SAMPLE => focus_nav_ids(
            TARGET_CH4_OSCILLATOR,
            UI_FOCUS_NONE,
            TARGET_SAVE,
            UI_FOCUS_NONE,
        ),
        TARGET_SAVE => focus_nav_ids(ch4_last_detail, UI_FOCUS_NONE, TARGET_DELETE, UI_FOCUS_NONE),
        TARGET_DELETE => focus_nav_ids(TARGET_SAVE, UI_FOCUS_NONE, TARGET_RESTORE, UI_FOCUS_NONE),
        TARGET_RESTORE => focus_nav_ids(TARGET_DELETE, UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE),
        _ => focus_nav_ids(UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE),
    }
}

fn selected_channel_oscillator(
    selects: &Query<(&AudioSelect, &UiMultiSelect)>,
    channel: u8,
) -> usize {
    selects
        .iter()
        .find(|(select, _)| select.channel == channel && select.field == FIELD_OSCILLATOR)
        .map(|(_, select)| select.selected)
        .unwrap_or(OSCILLATOR_SQUARE)
}

fn channel_detail_target(
    oscillator: usize,
    built_in_sample: u16,
    custom_sample: u16,
    default: u16,
) -> u16 {
    match oscillator {
        OSCILLATOR_BUILT_IN_SAMPLER => built_in_sample,
        OSCILLATOR_CUSTOM_SAMPLER => custom_sample,
        _ => default,
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

fn set_audio_preset_apply_error_message(
    messages: &mut Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
    error: &str,
) {
    set_latest_info_message(
        messages,
        &format!("Audio preset could not be applied: {error}"),
    );
}

fn audio_settings_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    primary_input: &PrimaryInputDevice,
    storage: &LocalStorage,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let body_font = font.clone();
    let landscape_font = font.clone();
    let (preset, load_message) = load_current_audio_preset(storage);
    let landscape_preset = preset.clone();
    let preset_label = format!("Preset {}", storage.data.settings.audio_preset.min(9));
    let landscape_preset_label = preset_label.clone();

    bsn! {
        DespawnOnExit::<AppState>(AppState::AudioSettings)
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
                    settings_header(font.clone(), assets.icons.clone(), theme, "Audio Preset Settings"),
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
                            audio_landscape_body(landscape_font, theme, landscape_preset, landscape_preset_label),
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
                                #AudioBodyScrollBar
                                flow_scroll_view(
                                    theme,
                                    #AudioBodyScrollBar,
                                    ScrollViewConfig {
                                        width: percent(100),
                                        min_height: px(0.0),
                                        thumb_height: 132.0,
                                    },
                                    move |_| audio_body(body_font, theme, preset, preset_label)
                                )
                            )
                        ]
                    ),
                    info_message_text(font.clone(), theme, load_message.unwrap_or_default(), false),
                    action_hints_with_labels(font, assets.icons.clone(), theme, storage, primary_input, "Back", "Select"),
                ]
            )
        ]
    }
}

fn audio_body(
    font: Handle<Font>,
    theme: ActiveTheme,
    preset: AudioPreset,
    preset_label: String,
) -> impl Scene {
    let controls_font = font.clone();
    let buttons_font = font;

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
                    width: percent(UI_WIDE_PRIMARY_COLUMN_PERCENT),
                    min_height: px(0.0),
                }
                ResponsivePercentWidth { landscape: UI_WIDE_PRIMARY_COLUMN_PERCENT }
                Children [
                    audio_controls(controls_font, theme, preset, preset_label),
                ]
            ),
            audio_buttons(buttons_font, theme),
        ]
    }
}

fn audio_landscape_body(
    font: Handle<Font>,
    theme: ActiveTheme,
    preset: AudioPreset,
    preset_label: String,
) -> impl Scene {
    let controls_font = font.clone();
    let buttons_font = font;

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
                #AudioScrollBar
                scroll_view(
                    theme,
                    #AudioScrollBar,
                    ScrollViewConfig {
                        width: percent(UI_WIDE_PRIMARY_COLUMN_PERCENT),
                        min_height: px(0.0),
                        thumb_height: 132.0,
                    },
                    move |_| audio_controls(controls_font, theme, preset, preset_label)
                )
            ),
            audio_buttons(buttons_font, theme),
        ]
    }
}

fn audio_controls(
    font: Handle<Font>,
    theme: ActiveTheme,
    preset: AudioPreset,
    preset_label: String,
) -> impl Scene {
    let ch1 = preset_channel(&preset, 0);
    let ch2 = preset_channel(&preset, 1);
    let ch3 = preset_channel(&preset, 2);
    let ch4 = preset_channel(&preset, 3);
    let ch1_oscillator = selected_oscillator(&ch1.oscillator);
    let ch2_oscillator = selected_oscillator(&ch2.oscillator);
    let ch4_oscillator = selected_oscillator(&ch4.oscillator);

    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_CONTROL_GAP),
            padding: UiRect {
                left: px(0.0),
                right: px(18.0),
                top: px(0.0),
                bottom: px(UI_SCROLL_CONTENT_BOTTOM_PADDING),
            },
        }
        Children [
            preset_label_row(font.clone(), theme, preset_label),
            channel_one_controls(font.clone(), theme, ch1, ch1_oscillator),
            channel_two_controls(font.clone(), theme, ch2, ch2_oscillator),
            channel_three_controls(font.clone(), theme, ch3),
            channel_four_controls(font, theme, ch4, ch4_oscillator),
        ]
    }
}

fn channel_one_controls(
    font: Handle<Font>,
    theme: ActiveTheme,
    preset: AudioChannelPreset,
    oscillator: usize,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_INNER_PADDING),
        }
        Children [
            description(font.clone(), theme, "Channel 1"),
            audio_select_row(font.clone(), theme, "Oscillator", 1, FIELD_OSCILLATOR, oscillator_config(oscillator), TARGET_CH1_OSCILLATOR, UI_FOCUS_NONE, channel_detail_target(oscillator, TARGET_CH1_BUILT_IN_SAMPLE, TARGET_CH1_CUSTOM_SAMPLE, TARGET_CH1_MOD_A), TARGET_SAVE, UI_FOCUS_NONE, true),
            built_in_sample_row(font.clone(), theme, 1, preset.built_in_sample.clone(), oscillator == OSCILLATOR_BUILT_IN_SAMPLER, TARGET_CH1_BUILT_IN_SAMPLE, TARGET_CH1_OSCILLATOR, TARGET_CH1_MOD_A, TARGET_SAVE, UI_FOCUS_NONE),
            custom_sample_row(font.clone(), theme, 1, preset.custom_sample_path.clone(), oscillator == OSCILLATOR_CUSTOM_SAMPLER, TARGET_CH1_CUSTOM_SAMPLE, TARGET_CH1_OSCILLATOR, TARGET_CH1_MOD_A, TARGET_SAVE, UI_FOCUS_NONE),
            audio_select_row(font.clone(), theme, "Modulation 1", 1, FIELD_MODULATION_A, modulation_config(selected_modulation(&preset.modulation_a)), TARGET_CH1_MOD_A, channel_detail_target(oscillator, TARGET_CH1_BUILT_IN_SAMPLE, TARGET_CH1_CUSTOM_SAMPLE, TARGET_CH1_OSCILLATOR), TARGET_CH1_MOD_B, TARGET_SAVE, UI_FOCUS_NONE, false),
            audio_select_row(font, theme, "Modulation 2", 1, FIELD_MODULATION_B, modulation_config(selected_modulation(preset.modulation_b.as_deref().unwrap_or("Pitch Envelope"))), TARGET_CH1_MOD_B, TARGET_CH1_MOD_A, TARGET_CH2_OSCILLATOR, TARGET_SAVE, UI_FOCUS_NONE, false),
        ]
    }
}

fn channel_two_controls(
    font: Handle<Font>,
    theme: ActiveTheme,
    preset: AudioChannelPreset,
    oscillator: usize,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_INNER_PADDING),
        }
        Children [
            description(font.clone(), theme, "Channel 2"),
            audio_select_row(font.clone(), theme, "Oscillator", 2, FIELD_OSCILLATOR, oscillator_config(oscillator), TARGET_CH2_OSCILLATOR, TARGET_CH1_MOD_B, channel_detail_target(oscillator, TARGET_CH2_BUILT_IN_SAMPLE, TARGET_CH2_CUSTOM_SAMPLE, TARGET_CH2_MOD_A), TARGET_SAVE, UI_FOCUS_NONE, false),
            built_in_sample_row(font.clone(), theme, 2, preset.built_in_sample.clone(), oscillator == OSCILLATOR_BUILT_IN_SAMPLER, TARGET_CH2_BUILT_IN_SAMPLE, TARGET_CH2_OSCILLATOR, TARGET_CH2_MOD_A, TARGET_SAVE, UI_FOCUS_NONE),
            custom_sample_row(font.clone(), theme, 2, preset.custom_sample_path.clone(), oscillator == OSCILLATOR_CUSTOM_SAMPLER, TARGET_CH2_CUSTOM_SAMPLE, TARGET_CH2_OSCILLATOR, TARGET_CH2_MOD_A, TARGET_SAVE, UI_FOCUS_NONE),
            audio_select_row(font, theme, "Modulation", 2, FIELD_MODULATION_A, modulation_config(selected_modulation(&preset.modulation_a)), TARGET_CH2_MOD_A, channel_detail_target(oscillator, TARGET_CH2_BUILT_IN_SAMPLE, TARGET_CH2_CUSTOM_SAMPLE, TARGET_CH2_OSCILLATOR), TARGET_CH3_SAMPLE, TARGET_SAVE, UI_FOCUS_NONE, false),
        ]
    }
}

fn channel_three_controls(
    font: Handle<Font>,
    theme: ActiveTheme,
    preset: AudioChannelPreset,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_INNER_PADDING),
        }
        Children [
            description(font.clone(), theme, "Channel 3"),
            audio_select_row(font, theme, "Wave Sample", 3, FIELD_BUILT_IN_SAMPLE, sample_config(selected_sample(&preset.built_in_sample)), TARGET_CH3_SAMPLE, TARGET_CH2_MOD_A, TARGET_CH4_OSCILLATOR, TARGET_SAVE, UI_FOCUS_NONE, false),
        ]
    }
}

fn channel_four_controls(
    font: Handle<Font>,
    theme: ActiveTheme,
    preset: AudioChannelPreset,
    oscillator: usize,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_INNER_PADDING),
        }
        Children [
            description(font.clone(), theme, "Channel 4"),
            audio_select_row(font.clone(), theme, "Oscillator", 4, FIELD_OSCILLATOR, oscillator_config(oscillator), TARGET_CH4_OSCILLATOR, TARGET_CH3_SAMPLE, TARGET_CH4_BUILT_IN_SAMPLE, TARGET_SAVE, UI_FOCUS_NONE, false),
            built_in_sample_row(font.clone(), theme, 4, preset.built_in_sample.clone(), oscillator == OSCILLATOR_BUILT_IN_SAMPLER, TARGET_CH4_BUILT_IN_SAMPLE, TARGET_CH4_OSCILLATOR, TARGET_CH4_CUSTOM_SAMPLE, TARGET_SAVE, UI_FOCUS_NONE),
            custom_sample_row(font, theme, 4, preset.custom_sample_path, oscillator == OSCILLATOR_CUSTOM_SAMPLER, TARGET_CH4_CUSTOM_SAMPLE, TARGET_CH4_BUILT_IN_SAMPLE, UI_FOCUS_NONE, TARGET_SAVE, UI_FOCUS_NONE),
        ]
    }
}

fn audio_buttons(font: Handle<Font>, theme: ActiveTheme) -> impl Scene {
    bsn! {
        Node {
            width: percent(UI_WIDE_SECONDARY_COLUMN_PERCENT),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_SECTION_GAP),
            padding: UiRect::top(px(58.0)),
        }
        ResponsivePercentWidth { landscape: UI_WIDE_SECONDARY_COLUMN_PERCENT }
        Children [
            (
                button(font.clone(), "Save", theme, UiFocusNav::default())
                AudioSaveButton
                UiFocusId { id: TARGET_SAVE }
                UiFocusNavIds { up: UI_FOCUS_NONE, right: UI_FOCUS_NONE, down: TARGET_DELETE, left: TARGET_CH1_OSCILLATOR }
            ),
            (
                button(font.clone(), "Delete", theme, UiFocusNav::default())
                AudioDeleteButton
                UiFocusId { id: TARGET_DELETE }
                UiFocusNavIds { up: TARGET_SAVE, right: UI_FOCUS_NONE, down: TARGET_RESTORE, left: TARGET_CH1_OSCILLATOR }
            ),
            (
                button(font, "Restore Defaults", theme, UiFocusNav::default())
                AudioRestoreButton
                UiFocusId { id: TARGET_RESTORE }
                UiFocusNavIds { up: TARGET_DELETE, right: UI_FOCUS_NONE, down: UI_FOCUS_NONE, left: TARGET_CH1_OSCILLATOR }
            ),
        ]
    }
}

fn preset_label_row(font: Handle<Font>, theme: ActiveTheme, preset_name: String) -> impl Scene {
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
            description(font, theme, "Preset:"),
            (
                Text(preset_name)
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(value_font)),
                    font_size: px(UI_BODY_FONT_SIZE),
                }
                TextColor({theme.primary})
                UiThemeTextColor::Primary
                IgnorePicking
                TextLayout::new(Justify::Right, LineBreak::NoWrap)
            )
        ]
    }
}

fn audio_select_row(
    font: Handle<Font>,
    theme: ActiveTheme,
    label: &'static str,
    channel: u8,
    field: u8,
    config: MultiSelectConfig,
    id: u16,
    up: u16,
    down: u16,
    right: u16,
    left: u16,
    initial_focus: bool,
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
                multi_select(font, theme, config)
                AudioSelect { channel: {channel}, field: {field} }
                UiFocusId { id: {id} }
                UiFocusNavIds { up: {up}, right: {right}, down: {down}, left: {left} }
                InitialFocus { enabled: {initial_focus} }
            ),
        ]
    }
}

fn built_in_sample_row(
    font: Handle<Font>,
    theme: ActiveTheme,
    channel: u8,
    sample: String,
    visible: bool,
    id: u16,
    up: u16,
    down: u16,
    right: u16,
    left: u16,
) -> impl Scene {
    bsn! {
        (
            Node {
                width: percent(100),
                display: {display_for(visible)},
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(18.0),
            }
            ResponsiveFieldRow { gap: 18.0 }
            DespawnOnExit::<AppState>(AppState::AudioSettings)
            AudioConditionalSection { channel: {channel}, section: SECTION_BUILT_IN_SAMPLE }
            Children [
                description(font.clone(), theme, "Built-in Sample"),
                (
                    multi_select(font, theme, sample_config(selected_sample(&sample)))
                    AudioSelect { channel: {channel}, field: FIELD_BUILT_IN_SAMPLE }
                    UiFocusId { id: {id} }
                    UiFocusNavIds { up: {up}, right: {right}, down: {down}, left: {left} }
                ),
            ]
        )
    }
}

fn custom_sample_row(
    font: Handle<Font>,
    theme: ActiveTheme,
    channel: u8,
    value: String,
    visible: bool,
    id: u16,
    up: u16,
    down: u16,
    right: u16,
    left: u16,
) -> impl Scene {
    bsn! {
        (
            Node {
                width: percent(100),
                display: {display_for(visible)},
                justify_content: JustifyContent::FlexEnd,
            }
            DespawnOnExit::<AppState>(AppState::AudioSettings)
            AudioConditionalSection { channel: {channel}, section: SECTION_CUSTOM_SAMPLE }
            Children [
                (
                    file_picker_with_value(font, "Choose a WAV sample...", value, theme, UiFocusNav::default())
                    DespawnOnExit::<AppState>(AppState::AudioSettings)
                    UiAudioFilePicker
                    AudioSamplePicker { channel: {channel} }
                    UiFocusId { id: {id} }
                    UiFocusNavIds { up: {up}, right: {right}, down: {down}, left: {left} }
                )
            ]
        )
    }
}

fn oscillator_config(selected: usize) -> MultiSelectConfig {
    select_config(
        selected.min(6),
        vec![
            "Square Wave",
            "Triangle Wave",
            "Sawtooth Wave",
            "LFSR Noise",
            "Built-in Sampler",
            "Custom Sampler",
            AUDIO_OPTION_SILENCE,
        ],
    )
}

fn modulation_config(selected: usize) -> MultiSelectConfig {
    select_config(
        selected.min(4),
        vec![
            "Duty Cycle",
            "Low-pass Cutoff",
            "High-pass Cutoff",
            "Notch Filter Frequency",
            "Vibrato",
        ],
    )
}

fn sample_config(selected: usize) -> MultiSelectConfig {
    select_config(
        selected.min(4),
        vec!["Piano", "Guitar", "Bass", "Bell", AUDIO_OPTION_SILENCE],
    )
}

fn select_config(selected: usize, options: Vec<&'static str>) -> MultiSelectConfig {
    MultiSelectConfig {
        selected,
        options: options.into_iter().map(str::to_string).collect(),
        nav: UiFocusNav::default(),
    }
}

fn current_preset_path(storage: &LocalStorage) -> std::path::PathBuf {
    storage
        .paths
        .audio_preset_file(storage.data.settings.audio_preset.min(9))
}

fn load_current_audio_preset(storage: &LocalStorage) -> (AudioPreset, Option<String>) {
    match load_audio_preset(&current_preset_path(storage)) {
        Ok(preset) => (preset, None),
        Err(error) => {
            eprintln!("failed to load audio preset: {error}");
            (
                default_audio_preset(),
                Some(format!("Audio preset could not be loaded: {error}")),
            )
        }
    }
}

fn audio_preset_from_form(
    selects: &Query<(&AudioSelect, &UiMultiSelect)>,
    pickers: &Query<(&AudioSamplePicker, &UiFilePicker)>,
) -> AudioPreset {
    let default = default_audio_preset();
    let channels = (1..=4)
        .map(|channel| {
            let mut channel_preset = preset_channel(&default, (channel - 1) as usize).clone();
            if matches!(channel, 1 | 2 | 4) {
                channel_preset.oscillator =
                    oscillator_label(select_value(selects, channel, FIELD_OSCILLATOR, 0))
                        .to_string();
            }
            channel_preset.built_in_sample =
                sample_label(select_value(selects, channel, FIELD_BUILT_IN_SAMPLE, 0)).to_string();
            if channel == 3 && channel_preset.built_in_sample == AUDIO_OPTION_SILENCE {
                channel_preset.oscillator = AUDIO_OPTION_SILENCE.to_string();
            }
            channel_preset.custom_sample_path = pickers
                .iter()
                .find(|(picker, _)| picker.channel == channel)
                .map(|(_, picker)| picker.value.clone())
                .unwrap_or_default();
            if matches!(channel, 1 | 2) {
                channel_preset.modulation_a =
                    modulation_label(select_value(selects, channel, FIELD_MODULATION_A, 0))
                        .to_string();
            }
            if channel == 1 {
                channel_preset.modulation_b = Some(
                    modulation_label(select_value(selects, channel, FIELD_MODULATION_B, 1))
                        .to_string(),
                );
            }
            channel_preset
        })
        .collect();
    AudioPreset { channels }
}

fn apply_audio_preset_to_selects(
    preset: &AudioPreset,
    selects: &mut Query<(&AudioSelect, &mut UiMultiSelect, Option<&Children>)>,
    select_labels: &mut Query<&mut Text, With<UiMultiSelectLabel>>,
    child_query: &Query<&Children>,
) {
    for (select, mut ui_select, children) in selects {
        let channel = preset_channel(preset, select.channel.saturating_sub(1) as usize);
        let selected = match select.field {
            FIELD_OSCILLATOR => selected_oscillator(&channel.oscillator),
            FIELD_BUILT_IN_SAMPLE => selected_sample(&channel.built_in_sample),
            FIELD_MODULATION_A => selected_modulation(&channel.modulation_a),
            FIELD_MODULATION_B => {
                selected_modulation(channel.modulation_b.as_deref().unwrap_or("Duty Cycle"))
            }
            _ => ui_select.selected,
        };

        ui_select.selected = selected;
        if let Some(children) = children {
            update_multi_select_label(
                children,
                select_label(select.field, selected),
                select_labels,
                child_query,
            );
        }
    }
}

fn apply_audio_preset_to_pickers(
    preset: &AudioPreset,
    pickers: &mut Query<(&AudioSamplePicker, &mut UiFilePicker, Option<&Children>)>,
    picker_labels: &mut Query<(Entity, &UiFilePickerValue, &mut Text)>,
    child_query: &Query<&Children>,
) {
    for (picker, mut ui_picker, children) in pickers {
        let channel = preset_channel(preset, picker.channel.saturating_sub(1) as usize);
        ui_picker.value = channel.custom_sample_path;
        if let Some(children) = children {
            update_file_picker_label(
                children,
                "Choose a WAV sample...",
                picker_labels,
                child_query,
            );
        }
    }
}

fn update_multi_select_label(
    children: &Children,
    label: &'static str,
    labels: &mut Query<&mut Text, With<UiMultiSelectLabel>>,
    child_query: &Query<&Children>,
) {
    for child in children {
        if let Ok(mut text) = labels.get_mut(*child) {
            text.0 = label.to_string();
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            update_multi_select_label(grandchildren, label, labels, child_query);
        }
    }
}

fn update_file_picker_label(
    children: &Children,
    label: &'static str,
    labels: &mut Query<(Entity, &UiFilePickerValue, &mut Text)>,
    child_query: &Query<&Children>,
) {
    for (entity, _, mut text) in labels {
        if contains_descendant(children, entity, child_query) {
            text.0 = label.to_string();
        }
    }
}

fn contains_descendant(
    children: &Children,
    target: Entity,
    child_query: &Query<&Children>,
) -> bool {
    for child in children {
        if *child == target {
            return true;
        }
        if child_query
            .get(*child)
            .is_ok_and(|grandchildren| contains_descendant(grandchildren, target, child_query))
        {
            return true;
        }
    }
    false
}

fn select_label(field: u8, selected: usize) -> &'static str {
    match field {
        FIELD_OSCILLATOR => oscillator_label(selected),
        FIELD_BUILT_IN_SAMPLE => sample_label(selected),
        FIELD_MODULATION_A | FIELD_MODULATION_B => modulation_label(selected),
        _ => "",
    }
}

fn select_value(
    selects: &Query<(&AudioSelect, &UiMultiSelect)>,
    channel: u8,
    field: u8,
    default: usize,
) -> usize {
    selects
        .iter()
        .find(|(select, _)| select.channel == channel && select.field == field)
        .map(|(_, select)| select.selected)
        .unwrap_or(default)
}

fn preset_channel(preset: &AudioPreset, index: usize) -> AudioChannelPreset {
    preset
        .channels
        .get(index)
        .cloned()
        .unwrap_or_else(|| default_audio_preset().channels[index].clone())
}

fn selected_oscillator(value: &str) -> usize {
    oscillator_options()
        .iter()
        .position(|option| *option == value)
        .unwrap_or(OSCILLATOR_SQUARE)
}

fn selected_modulation(value: &str) -> usize {
    modulation_options()
        .iter()
        .position(|option| *option == value)
        .unwrap_or(0)
}

fn selected_sample(value: &str) -> usize {
    sample_options()
        .iter()
        .position(|option| *option == value)
        .unwrap_or(0)
}

fn oscillator_label(index: usize) -> &'static str {
    oscillator_options()
        .get(index)
        .copied()
        .unwrap_or("Square Wave")
}

fn modulation_label(index: usize) -> &'static str {
    modulation_options()
        .get(index)
        .copied()
        .unwrap_or("Duty Cycle")
}

fn sample_label(index: usize) -> &'static str {
    sample_options().get(index).copied().unwrap_or("Piano")
}

fn oscillator_options() -> [&'static str; 7] {
    [
        "Square Wave",
        "Triangle Wave",
        "Sawtooth Wave",
        "LFSR Noise",
        "Built-in Sampler",
        "Custom Sampler",
        AUDIO_OPTION_SILENCE,
    ]
}

fn modulation_options() -> [&'static str; 5] {
    [
        "Duty Cycle",
        "Low-pass Cutoff",
        "High-pass Cutoff",
        "Notch Filter Frequency",
        "Vibrato",
    ]
}

fn sample_options() -> [&'static str; 5] {
    ["Piano", "Guitar", "Bass", "Bell", AUDIO_OPTION_SILENCE]
}
