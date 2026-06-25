use std::fs;
use std::path::PathBuf;

use bevy::asset::HandleTemplate;
use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::math::Rect;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;
use bevy::window::PrimaryWindow;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::dimensions::{
    GAMEPLAY_ERROR_ICON_SIZE, LOADING_INDICATOR_GRID_SIZE, TOUCH_OVERLAY_MARGIN, UI_BODY_FONT_SIZE,
    UI_CONTENT_GAP, UI_CONTROL_FONT_SIZE, UI_ELEMENT_HEIGHT, UI_INNER_PADDING,
    UI_MULTI_SELECT_WIDTH, UI_SCREEN_PADDING,
};
use crate::game_boy::{
    CheatCode, GameBoyCore, GameBoyEmulator, GameBoyLoadStatus, apply_save_state,
    encode_save_state, parse_cheat_code,
};
use crate::input::events::MappedInputEvent;
use crate::input::selection::PrimaryInputDevice;
use crate::storage::LocalStorage;
use crate::storage::input_mappings::InputAction;
use crate::ui_elements::action_hint::action_hints_for_actions;
use crate::ui_elements::choice_popup::{
    ChoicePopupConfig, ChoicePopupContext, ChoicePopupOption, centered_choice_popup_position,
    choice_popup_context_index, choice_popup_menu, despawn_choice_popups,
};
use crate::ui_elements::interactions::{
    ActivatedUiElement, FocusedUiElement, IgnorePicking, LastFocusedUiElement, UiElementColors,
    UiElementKind, UiElementLabel, UiFocusNav, UiTextInput,
};
use crate::ui_elements::loading_indicator::loading_indicator_scene;
use crate::ui_elements::styles::{control_fill, hover_fill, ui_border, ui_padding, ui_radius};
use crate::ui_elements::text_input::text_input_with_value_width;
use crate::ui_elements::theme::{UiElementTheme, UiThemeBorderColor, UiThemeTextColor};

const ICON_TEXTURE_SIZE: f32 = 1024.0;
const ICON_GRID_UNITS: f32 = 16.0;
const ERROR_ICON_X: f32 = 0.0;
const ERROR_ICON_Y: f32 = 8.0;
const ERROR_ICON_SIZE: f32 = 4.0;
const GAMEPLAY_MENU_BUTTON_WIDTH: f32 = 104.0;
const GAMEPLAY_MENU_POPUP_ESTIMATED_HEIGHT: f32 = 372.0;
const CHEAT_CODES_POPUP_WIDTH: f32 = UI_MULTI_SELECT_WIDTH * 2.0;
const GAMEPLAY_MENU_CONTEXT_INDEX: usize = 0;
const CHEAT_CODES_CONTEXT_INDEX: usize = 1;
const CHEAT_OPTION_ADD: usize = 0;
const CHEAT_OPTION_BACK: usize = 1;
const CHEAT_OPTION_CLOSE: usize = 2;
const FIRST_MANUAL_SAVE_SLOT: u8 = 0;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct GameplayErrorOverlay;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct GameplayErrorMessage;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct GameplayErrorIcon;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct GameplayLoadingIndicator;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct GameplayMenuButton;

#[derive(Default, Resource)]
struct GameplayMenuPauseState {
    paused_by_menu: bool,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct CheatCodePopupRoot;

#[derive(SystemParam)]
struct GameplayMenuActivationQueries<'w, 's> {
    menu_buttons: Query<'w, 's, (), With<GameplayMenuButton>>,
    popup_options: Query<'w, 's, &'static ChoicePopupOption>,
    popup_roots: Query<'w, 's, (Entity, &'static ChoicePopupContext, &'static Children)>,
    child_query: Query<'w, 's, &'static Children>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    focused: Query<'w, 's, Entity, With<FocusedUiElement>>,
    text_inputs: Query<'w, 's, &'static UiTextInput>,
    cheat_code_popup_roots: Query<'w, 's, Entity, With<CheatCodePopupRoot>>,
}

pub struct GameplayScenePlugin;

impl Plugin for GameplayScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameplayMenuPauseState>()
            .add_systems(OnEnter(AppState::Gameplay), spawn_gameplay_scene)
            .add_systems(
                Update,
                (
                    return_home_from_gameplay,
                    resume_after_gameplay_menu_dismissal,
                    update_gameplay_error_overlay,
                )
                    .run_if(in_state(AppState::Gameplay)),
            )
            .add_systems(OnExit(AppState::Gameplay), auto_save_gameplay_on_exit)
            .add_observer(handle_gameplay_activation);
    }
}

fn spawn_gameplay_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    storage: Res<LocalStorage>,
    primary_input: Res<PrimaryInputDevice>,
) {
    commands.spawn_scene(gameplay_scene(&assets, *theme, &storage, &primary_input));
}

fn return_home_from_gameplay(
    mut input_events: MessageReader<MappedInputEvent>,
    mut storage: ResMut<LocalStorage>,
    emulators: Query<&GameBoyCore, With<GameBoyEmulator>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if input_events
        .read()
        .any(|event| event.state == ButtonState::Pressed && event.action == InputAction::QuitRom)
    {
        auto_save_gameplay_from_query(&mut storage, &emulators);
        next_state.set(AppState::Home);
    }
}

fn update_gameplay_error_overlay(
    status: Res<GameBoyLoadStatus>,
    mut nodes: ParamSet<(
        Query<&mut Node, With<GameplayErrorOverlay>>,
        Query<&mut Node, With<GameplayLoadingIndicator>>,
        Query<&mut Node, With<GameplayErrorIcon>>,
    )>,
    mut messages: Query<&mut Text, With<GameplayErrorMessage>>,
) {
    let message = status.overlay_message();
    let display = if message.is_some() {
        Display::Flex
    } else {
        Display::None
    };

    for mut node in &mut nodes.p0() {
        node.display = display;
    }
    let loading_display = if status.is_loading() {
        Display::Flex
    } else {
        Display::None
    };
    let error_display = if message.is_some() && !status.is_loading() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut nodes.p1() {
        node.display = loading_display;
    }
    for mut node in &mut nodes.p2() {
        node.display = error_display;
    }
    let Some(message) = message else {
        return;
    };
    for mut text in &mut messages {
        if text.0 != message {
            text.0 = message.to_string();
        }
    }
}

fn handle_gameplay_activation(
    activated: On<Add, ActivatedUiElement>,
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    queries: GameplayMenuActivationQueries,
    mut emulators: Query<&mut GameBoyCore, With<GameBoyEmulator>>,
    mut storage: ResMut<LocalStorage>,
    state: Res<State<AppState>>,
    mut status: ResMut<GameBoyLoadStatus>,
    mut menu_pause: ResMut<GameplayMenuPauseState>,
    mut last_focused: ResMut<LastFocusedUiElement>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if *state.get() != AppState::Gameplay {
        return;
    }

    let entity = activated.entity;
    if queries.menu_buttons.get(entity).is_ok() {
        pause_gameplay(&mut emulators, &mut status);
        clear_gameplay_ui_focus(&mut commands, &queries.focused, &mut last_focused);
        menu_pause.paused_by_menu = true;
        despawn_choice_popups(&mut commands, &queries.popup_roots);
        let popup_position = centered_choice_popup_position(
            &queries.windows,
            CHEAT_CODES_POPUP_WIDTH,
            CHEAT_CODES_POPUP_ESTIMATED_HEIGHT,
        );
        commands.spawn_scene(gameplay_menu_popup_scene(&assets, *theme, popup_position));
        return;
    }

    let Ok(option) = queries.popup_options.get(entity) else {
        return;
    };

    let Some(context_index) =
        choice_popup_context_index(entity, &queries.popup_roots, &queries.child_query)
    else {
        return;
    };

    if context_index == CHEAT_CODES_CONTEXT_INDEX {
        match option.option_index {
            CHEAT_OPTION_ADD => {
                handle_add_cheat_code(
                    &mut commands,
                    &queries,
                    &mut emulators,
                    &assets,
                    theme,
                    &mut status,
                );
            }
            CHEAT_OPTION_BACK => {
                handle_cheat_codes_back(&mut commands, &assets, *theme, &queries);
            }
            CHEAT_OPTION_CLOSE => {
                despawn_cheat_codes_popup(&mut commands, &queries);
                resume_gameplay(&mut emulators, &mut status);
                menu_pause.paused_by_menu = false;
            }
            _ => {}
        }
        return;
    }

    if context_index != GAMEPLAY_MENU_CONTEXT_INDEX {
        return;
    }

    clear_gameplay_ui_focus(&mut commands, &queries.focused, &mut last_focused);
    despawn_choice_popups(&mut commands, &queries.popup_roots);
    menu_pause.paused_by_menu = false;
    match option.option_index {
        0 => resume_gameplay(&mut emulators, &mut status),
        1 => reboot_current_rom(&mut emulators, &storage, &mut status),
        2 => create_manual_save(&mut emulators, &mut storage, &mut status),
        3 => restore_manual_save(&mut emulators, &storage, &mut status),
        4 => {
            let codes = emulators
                .iter()
                .next()
                .map(|e| e.cheats.codes.clone())
                .unwrap_or_default();
            spawn_cheat_codes_popup(&mut commands, &assets, *theme, &queries, &codes);
            return;
        }
        5 => {
            auto_save_gameplay_from_mut_query(&mut storage, &mut emulators);
            next_state.set(AppState::Home);
        }
        _ => resume_gameplay(&mut emulators, &mut status),
    }
}

fn resume_after_gameplay_menu_dismissal(
    popup_roots: Query<&ChoicePopupContext>,
    mut commands: Commands,
    mut emulators: Query<&mut GameBoyCore, With<GameBoyEmulator>>,
    mut status: ResMut<GameBoyLoadStatus>,
    mut menu_pause: ResMut<GameplayMenuPauseState>,
    focused: Query<Entity, With<FocusedUiElement>>,
    mut last_focused: ResMut<LastFocusedUiElement>,
) {
    let gameplay_menu_open = popup_roots
        .iter()
        .any(|context| context.context_index == GAMEPLAY_MENU_CONTEXT_INDEX);
    if !menu_pause.paused_by_menu || gameplay_menu_open {
        return;
    }

    menu_pause.paused_by_menu = false;
    clear_gameplay_ui_focus(&mut commands, &focused, &mut last_focused);
    resume_gameplay(&mut emulators, &mut status);
}

fn auto_save_gameplay_on_exit(
    mut storage: ResMut<LocalStorage>,
    emulators: Query<&GameBoyCore, With<GameBoyEmulator>>,
) {
    auto_save_gameplay_from_query(&mut storage, &emulators);
}

fn auto_save_gameplay_from_query(
    storage: &mut LocalStorage,
    emulators: &Query<&GameBoyCore, With<GameBoyEmulator>>,
) {
    let Some(emulator) = emulators.iter().next() else {
        return;
    };
    auto_save_loaded_game(storage, emulator);
}

fn auto_save_gameplay_from_mut_query(
    storage: &mut LocalStorage,
    emulators: &mut Query<&mut GameBoyCore, With<GameBoyEmulator>>,
) {
    let Some(emulator) = emulators.iter_mut().next() else {
        return;
    };
    auto_save_loaded_game(storage, &emulator);
}

fn auto_save_loaded_game(storage: &mut LocalStorage, emulator: &GameBoyCore) {
    if !emulator.runtime.is_running || emulator.rom.current_rom_id.is_empty() {
        return;
    }

    let rom_id = emulator.rom.current_rom_id.clone();
    let bytes = match encode_save_state(emulator) {
        Ok(bytes) => bytes,
        Err(error) => {
            warn!("failed to encode automatic save state: {error}");
            return;
        }
    };
    if let Err(error) = write_save_state_file(storage.auto_save_path(&rom_id), &bytes) {
        warn!("failed to write automatic save state: {error}");
        return;
    }
    if let Err(error) = storage.record_rom_played(&rom_id) {
        warn!("failed to save last-played timestamp after automatic save state: {error}");
    }
}

fn clear_gameplay_ui_focus(
    commands: &mut Commands,
    focused: &Query<Entity, With<FocusedUiElement>>,
    last_focused: &mut LastFocusedUiElement,
) {
    for entity in focused {
        commands.entity(entity).try_remove::<FocusedUiElement>();
    }
    last_focused.clear();
}

fn gameplay_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    storage: &LocalStorage,
    primary_input: &PrimaryInputDevice,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let hint_font = font.clone();

    bsn! {
        #GameplayScene
        DespawnOnExit::<AppState>(AppState::Gameplay)
        Node {
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(UI_SCREEN_PADDING)),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_CONTENT_GAP),
        }
        Children [
            gameplay_menu_button(font.clone(), theme),
            (
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_height: px(0.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                Children [
                    (
                        Node {
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            flex_direction: FlexDirection::Column,
                            row_gap: px(UI_CONTENT_GAP),
                        }
                        GameplayErrorOverlay
                        Children [
                            (
                                Node {
                                    width: px(GAMEPLAY_ERROR_ICON_SIZE),
                                    height: px(GAMEPLAY_ERROR_ICON_SIZE),
                                }
                                GameplayErrorIcon
                                ImageNode {
                                    image: {HandleTemplate::Handle(assets.icons.clone())},
                                    color: {theme.secondary},
                                    rect: {Some(error_icon_rect())},
                                }
                            ),
                            (
                                Node {
                                    display: Display::None,
                                    width: px(LOADING_INDICATOR_GRID_SIZE),
                                    height: px(LOADING_INDICATOR_GRID_SIZE),
                                }
                                GameplayLoadingIndicator
                                Children [
                                    {bsn_list![loading_indicator_scene(theme)]}
                                ]
                            ),
                            (
                                Text("Loading ROM...")
                                GameplayErrorMessage
                                TextFont {
                                    font: {FontSourceTemplate::Handle(HandleTemplate::Handle(font.clone()))},
                                    font_size: px(UI_BODY_FONT_SIZE),
                                }
                                TextColor({theme.secondary})
                                TextLayout::new(Justify::Center, LineBreak::WordBoundary)
                            )
                        ]
                    )
                ]
            ),
            action_hints_for_actions(
                hint_font,
                assets.icons.clone(),
                theme,
                storage,
                primary_input,
                (InputAction::QuitRom, "Quit"),
                (InputAction::PauseAndResume, "Pause"),
            ),
        ]
    }
}

fn gameplay_menu_button(font: Handle<Font>, theme: ActiveTheme) -> impl Scene {
    let background = control_fill(&theme);
    let hover_background = hover_fill(&theme);

    bsn! {
        Node {
            position_type: PositionType::Absolute,
            right: px(TOUCH_OVERLAY_MARGIN),
            top: px(TOUCH_OVERLAY_MARGIN),
            width: px(GAMEPLAY_MENU_BUTTON_WIDTH),
            height: px(UI_ELEMENT_HEIGHT),
            border: ui_border(),
            border_radius: ui_radius(),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        GlobalZIndex(110)
        Button
        GameplayMenuButton
        BorderColor::all(theme.primary)
        UiThemeBorderColor::Primary
        BackgroundColor({background})
        UiFocusNav {
            up: Entity::PLACEHOLDER,
            right: Entity::PLACEHOLDER,
            down: Entity::PLACEHOLDER,
            left: Entity::PLACEHOLDER,
        }
        UiElementKind::Button
        UiElementTheme::Control
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: {background}, hover_fill: {hover_background} }
        Children [
            (
                Text("Menu")
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_CONTROL_FONT_SIZE),
                }
                TextColor({theme.primary})
                UiThemeTextColor::Primary
                UiElementLabel
                IgnorePicking
                TextLayout::new(Justify::Center, LineBreak::NoWrap)
            )
        ]
    }
}

fn gameplay_menu_popup_scene(assets: &AppAssets, theme: ActiveTheme, position: Vec2) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();

    bsn! {
        DespawnOnExit::<AppState>(AppState::Gameplay)
        GlobalZIndex(120)
        choice_popup_menu(
            font,
            theme,
            ChoicePopupConfig {
                title: "Emulation Menu".to_string(),
                width: UI_MULTI_SELECT_WIDTH,
                options: vec![
                    "Resume",
                    "Reboot",
                    "Save Slot 0",
                    "Restore Slot 0",
                    "Cheat Codes",
                    "Quit ROM",
                ],
            },
            position,
            GAMEPLAY_MENU_CONTEXT_INDEX,
        )
    }
}

fn pause_gameplay(
    emulators: &mut Query<&mut GameBoyCore, With<GameBoyEmulator>>,
    status: &mut GameBoyLoadStatus,
) {
    let Some(mut emulator) = emulators.iter_mut().next() else {
        status.set_error_message("The emulator is not available.");
        return;
    };
    emulator.runtime.is_paused = true;
}

fn resume_gameplay(
    emulators: &mut Query<&mut GameBoyCore, With<GameBoyEmulator>>,
    status: &mut GameBoyLoadStatus,
) {
    let Some(mut emulator) = emulators.iter_mut().next() else {
        status.set_error_message("The emulator is not available.");
        return;
    };
    emulator.runtime.is_paused = false;
    if emulator.runtime.is_running {
        status.set_ready();
    }
}

fn reboot_current_rom(
    emulators: &mut Query<&mut GameBoyCore, With<GameBoyEmulator>>,
    storage: &LocalStorage,
    status: &mut GameBoyLoadStatus,
) {
    let Some(mut emulator) = emulators.iter_mut().next() else {
        status.set_error_message("The emulator is not available.");
        return;
    };
    let rom_id = emulator.rom.current_rom_id.clone();
    if rom_id.is_empty() {
        status.set_error_message("No ROM is loaded.");
        return;
    }

    let properties = emulator.rom.properties;
    let opened_file = emulator.rom.current_opened_file.clone();
    let rom_len = usize::try_from(properties.size_bytes)
        .ok()
        .filter(|size_bytes| *size_bytes > 0)
        .unwrap_or(emulator.memory.rom.len())
        .min(emulator.memory.rom.len());
    let Some(rom_bytes) = emulator
        .memory
        .rom
        .get(..rom_len)
        .map(|bytes| bytes.to_vec())
    else {
        status.set_error_message("The loaded ROM data could not be read.");
        return;
    };

    if !emulator.reset_for_rom_load(properties, rom_id.clone(), opened_file, &rom_bytes) {
        emulator.runtime.is_running = false;
        status.set_error_message("The ROM could not be rebooted.");
        return;
    }
    if let Some(size_bytes) = emulator.sram.persistence_len() {
        match storage.load_sram(&rom_id, size_bytes) {
            Ok(saved_data) => emulator.sram.load_save_data(&saved_data),
            Err(error) => {
                emulator.runtime.is_running = false;
                status
                    .set_error_message(format!("battery-backed SRAM could not be loaded: {error}"));
                return;
            }
        }
    }
    let clock_frequency_hz = emulator.cpu_timing.clock_frequency_hz;
    emulator.audio_unit.reset_for_rom_load(clock_frequency_hz);
    status.set_ready();
}

fn create_manual_save(
    emulators: &mut Query<&mut GameBoyCore, With<GameBoyEmulator>>,
    storage: &mut LocalStorage,
    status: &mut GameBoyLoadStatus,
) {
    let Some(mut emulator) = emulators.iter_mut().next() else {
        status.set_error_message("The emulator is not available.");
        return;
    };
    let rom_id = emulator.rom.current_rom_id.clone();
    if rom_id.is_empty() {
        status.set_error_message("No ROM is loaded.");
        return;
    }

    let bytes = match encode_save_state(&emulator) {
        Ok(bytes) => bytes,
        Err(error) => {
            status.set_error_message(format!("Save state could not be created: {error}"));
            return;
        }
    };
    let Some(manual_path) = storage.manual_save_path(&rom_id, FIRST_MANUAL_SAVE_SLOT) else {
        status.set_error_message("Save state slot 0 is unavailable.");
        return;
    };
    if let Err(error) = write_save_state_file(manual_path, &bytes) {
        status.set_error_message(format!("Save state could not be created: {error}"));
        return;
    }
    if let Err(error) = storage.record_rom_played(&rom_id) {
        status.set_error_message(format!("Last-played timestamp could not be saved: {error}"));
        return;
    }

    emulator.runtime.is_paused = false;
    status.set_ready();
}

fn restore_manual_save(
    emulators: &mut Query<&mut GameBoyCore, With<GameBoyEmulator>>,
    storage: &LocalStorage,
    status: &mut GameBoyLoadStatus,
) {
    let Some(mut emulator) = emulators.iter_mut().next() else {
        status.set_error_message("The emulator is not available.");
        return;
    };
    let rom_id = emulator.rom.current_rom_id.clone();
    if rom_id.is_empty() {
        status.set_error_message("No ROM is loaded.");
        return;
    }

    let Some(path) = storage.manual_save_path(&rom_id, FIRST_MANUAL_SAVE_SLOT) else {
        status.set_error_message("Save state slot 0 is unavailable.");
        return;
    };
    if !path.is_file() {
        status.set_error_message("No save state exists in slot 0 for this ROM.");
        return;
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            status.set_error_message(format!(
                "Save state could not be read: {}: {error}",
                path.display()
            ));
            return;
        }
    };
    if let Err(error) = apply_save_state(&mut emulator, &bytes) {
        status.set_error_message(format!("Save state could not be restored: {error}"));
        return;
    }

    emulator.runtime.is_paused = false;
    status.set_ready();
}

fn write_save_state_file(path: PathBuf, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(&path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn error_icon_rect() -> Rect {
    let unit = ICON_TEXTURE_SIZE / ICON_GRID_UNITS;
    Rect {
        min: Vec2::new(ERROR_ICON_X * unit, ERROR_ICON_Y * unit),
        max: Vec2::new(
            (ERROR_ICON_X + ERROR_ICON_SIZE) * unit,
            (ERROR_ICON_Y + ERROR_ICON_SIZE) * unit,
        ),
    }
}

fn spawn_cheat_codes_popup(
    commands: &mut Commands,
    assets: &AppAssets,
    theme: ActiveTheme,
    queries: &GameplayMenuActivationQueries,
    cheat_codes: &[CheatCode],
) {
    let popup_position = centered_choice_popup_position(
        &queries.windows,
        UI_MULTI_SELECT_WIDTH,
        CHEAT_CODES_POPUP_ESTIMATED_HEIGHT,
    );
    commands.spawn_scene(cheat_codes_popup_scene(
        assets,
        theme,
        popup_position,
        cheat_codes,
    ));
}

const CHEAT_CODES_POPUP_ESTIMATED_HEIGHT: f32 = 480.0;

fn despawn_cheat_codes_popup(commands: &mut Commands, queries: &GameplayMenuActivationQueries) {
    for root in &queries.cheat_code_popup_roots {
        commands.entity(root).try_despawn();
    }
}

fn handle_add_cheat_code(
    commands: &mut Commands,
    queries: &GameplayMenuActivationQueries,
    emulators: &mut Query<&mut GameBoyCore, With<GameBoyEmulator>>,
    assets: &AppAssets,
    theme: Res<ActiveTheme>,
    status: &mut GameBoyLoadStatus,
) {
    let Some(input) = queries.text_inputs.iter().next() else {
        return;
    };
    let code_text = input.value.trim().to_string();
    if code_text.is_empty() {
        return;
    }

    let description = format!("Cheat {}", queries.text_inputs.iter().count() + 1);
    let Some(cheat_code) = parse_cheat_code(&code_text, description) else {
        status.set_error_message(format!("Could not parse cheat code: {code_text}"));
        return;
    };

    let Some(mut emulator) = emulators.iter_mut().next() else {
        status.set_error_message("The emulator is not available.");
        return;
    };

    emulator.cheats.codes.push(cheat_code);
    despawn_cheat_codes_popup(commands, queries);
    status.set_ready();

    let cheat_codes = emulator.cheats.codes.clone();
    let popup_position = centered_choice_popup_position(
        &queries.windows,
        CHEAT_CODES_POPUP_WIDTH,
        CHEAT_CODES_POPUP_ESTIMATED_HEIGHT,
    );
    commands.spawn_scene(cheat_codes_popup_scene(
        assets,
        *theme,
        popup_position,
        &cheat_codes,
    ));
}

fn handle_cheat_codes_back(
    commands: &mut Commands,
    assets: &AppAssets,
    theme: ActiveTheme,
    queries: &GameplayMenuActivationQueries,
) {
    despawn_cheat_codes_popup(commands, queries);
    let popup_position = centered_choice_popup_position(
        &queries.windows,
        UI_MULTI_SELECT_WIDTH,
        GAMEPLAY_MENU_POPUP_ESTIMATED_HEIGHT,
    );
    commands.spawn_scene(gameplay_menu_popup_scene(assets, theme, popup_position));
}

fn cheat_codes_popup_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    position: Vec2,
    codes: &[CheatCode],
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();

    bsn! {
        DespawnOnExit::<AppState>(AppState::Gameplay)
        GlobalZIndex(120)
        Node {
            position_type: PositionType::Absolute,
            left: px(position.x),
            top: px(position.y),
            width: px(CHEAT_CODES_POPUP_WIDTH),
            border: ui_border(),
            border_radius: ui_radius(),
            padding: ui_padding(),
            flex_direction: FlexDirection::Column,
            row_gap: px(12.0),
        }
        BorderColor::all(theme.secondary)
        UiThemeBorderColor::Secondary
        BackgroundColor(Color::BLACK)
        ChoicePopupContext { context_index: {CHEAT_CODES_CONTEXT_INDEX} }
        CheatCodePopupRoot
        Children [
            cheat_codes_popup_label(font.clone(), theme, "Cheat Codes"),
            text_input_with_value_width(
                font.clone(),
                "Enter code (e.g. 004-BCE or 010238CD)",
                String::new(),
                theme,
                UiFocusNav::default(),
                px(CHEAT_CODES_POPUP_WIDTH - 2.0 * UI_INNER_PADDING),
            ),
            cheat_codes_list_container(font.clone(), theme, codes),
            cheat_codes_button_row(font.clone(), theme),
        ]
    }
}

fn cheat_codes_list_container(
    font: Handle<Font>,
    theme: ActiveTheme,
    codes: &[CheatCode],
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(6.0),
            max_height: px(200.0),
            overflow: Overflow::clip(),
        }
        Children [
            {cheat_codes_list(font, theme, codes)}
        ]
    }
}

fn cheat_codes_button_row(font: Handle<Font>, theme: ActiveTheme) -> impl Scene {
    let background = control_fill(&theme);
    let hover_background = hover_fill(&theme);

    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            column_gap: px(8.0),
        }
        Children [
            cheat_codes_add_button(font.clone(), theme, background, hover_background),
            cheat_codes_back_button(font.clone(), theme, background, hover_background),
            cheat_codes_close_button(font, theme, background, hover_background),
        ]
    }
}

fn cheat_codes_add_button(
    font: Handle<Font>,
    theme: ActiveTheme,
    background: Color,
    hover_background: Color,
) -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: px(0.0),
            width: percent(100),
            height: px(UI_ELEMENT_HEIGHT),
            border: ui_border(),
            border_radius: ui_radius(),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Button
        ChoicePopupOption { option_index: {CHEAT_OPTION_ADD} }
        BorderColor::all(theme.primary)
        UiThemeBorderColor::Primary
        BackgroundColor({background})
        UiFocusNav { up: Entity::PLACEHOLDER, right: Entity::PLACEHOLDER, down: Entity::PLACEHOLDER, left: Entity::PLACEHOLDER }
        UiElementKind::Button
        UiElementTheme::Control
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: {background}, hover_fill: {hover_background} }
        Children [
            (
                Text("Add Code")
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_CONTROL_FONT_SIZE),
                }
                TextColor({theme.primary})
                UiThemeTextColor::Primary
                UiElementLabel
                IgnorePicking
                TextLayout::new(Justify::Center, LineBreak::NoWrap)
            ),
        ]
    }
}

fn cheat_codes_back_button(
    font: Handle<Font>,
    theme: ActiveTheme,
    background: Color,
    hover_background: Color,
) -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: px(0.0),
            width: percent(100),
            height: px(UI_ELEMENT_HEIGHT),
            border: ui_border(),
            border_radius: ui_radius(),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Button
        ChoicePopupOption { option_index: {CHEAT_OPTION_BACK} }
        BorderColor::all(theme.primary)
        UiThemeBorderColor::Primary
        BackgroundColor({background})
        UiFocusNav { up: Entity::PLACEHOLDER, right: Entity::PLACEHOLDER, down: Entity::PLACEHOLDER, left: Entity::PLACEHOLDER }
        UiElementKind::Button
        UiElementTheme::Control
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: {background}, hover_fill: {hover_background} }
        Children [
            (
                Text("Back")
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_CONTROL_FONT_SIZE),
                }
                TextColor({theme.primary})
                UiThemeTextColor::Primary
                UiElementLabel
                IgnorePicking
                TextLayout::new(Justify::Center, LineBreak::NoWrap)
            ),
        ]
    }
}

fn cheat_codes_close_button(
    font: Handle<Font>,
    theme: ActiveTheme,
    background: Color,
    hover_background: Color,
) -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: px(0.0),
            width: percent(100),
            height: px(UI_ELEMENT_HEIGHT),
            border: ui_border(),
            border_radius: ui_radius(),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Button
        ChoicePopupOption { option_index: {CHEAT_OPTION_CLOSE} }
        BorderColor::all(theme.primary)
        UiThemeBorderColor::Primary
        BackgroundColor({background})
        UiFocusNav { up: Entity::PLACEHOLDER, right: Entity::PLACEHOLDER, down: Entity::PLACEHOLDER, left: Entity::PLACEHOLDER }
        UiElementKind::Button
        UiElementTheme::Control
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: {background}, hover_fill: {hover_background} }
        Children [
            (
                Text("Close")
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_CONTROL_FONT_SIZE),
                }
                TextColor({theme.primary})
                UiThemeTextColor::Primary
                UiElementLabel
                IgnorePicking
                TextLayout::new(Justify::Center, LineBreak::NoWrap)
            ),
        ]
    }
}

fn cheat_codes_list(
    font: Handle<Font>,
    theme: ActiveTheme,
    codes: &[CheatCode],
) -> Vec<Box<dyn SceneList>> {
    if codes.is_empty() {
        return vec![Box::new(bsn_list![
            Text("No cheat codes entered.")
            TextFont {
                font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                font_size: px(UI_BODY_FONT_SIZE),
            }
            TextColor({theme.tertiary})
            UiThemeTextColor::Tertiary
            TextLayout::new(Justify::Center, LineBreak::WordBoundary)
        ]) as Box<dyn SceneList>];
    }

    codes
        .iter()
        .enumerate()
        .map(|(_i, code)| {
            let font = font.clone();
            let status = if code.enabled { "ON" } else { "OFF" };
            let label_text = format!(
                "[{status}] {:04X}:{:02X}{}",
                code.address,
                code.value,
                if code.compare.is_some() { " (cmp)" } else { "" }
            );
            Box::new(bsn_list![
                Text({label_text})
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_CONTROL_FONT_SIZE),
                }
                TextColor({theme.primary})
                UiThemeTextColor::Primary
                TextLayout::new(Justify::Left, LineBreak::NoWrap)
            ]) as Box<dyn SceneList>
        })
        .collect()
}

fn cheat_codes_popup_label(font: Handle<Font>, theme: ActiveTheme, text: &str) -> impl Scene {
    let label_text = text.to_string();
    bsn! {
        Text({label_text})
        TextFont {
            font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
            font_size: px(UI_BODY_FONT_SIZE),
        }
        TextColor({theme.primary})
        UiThemeTextColor::Primary
        TextLayout::new(Justify::Left, LineBreak::WordBoundary)
    }
}
