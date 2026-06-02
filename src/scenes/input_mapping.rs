use bevy::asset::HandleTemplate;
use bevy::input::ButtonState;
use bevy::input::gamepad::GamepadButtonStateChangedEvent;
use bevy::input::keyboard::KeyboardInput;
use bevy::math::Rect;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::input::key_ids::{gamepad_button_id, key_code_id};
use crate::input::mappings::{RuntimeInputMappings, ensure_essential_navigation_mappings};
use crate::input::selection::{InputMappingEditTarget, PrimaryInputDevice, mapping_label};
use crate::storage::LocalStorage;
use crate::storage::input_mappings::{
    InputAction, InputDeviceMapping, InputDeviceType, InputKeyId, InputMapEntry,
    default_controller_mapping, default_keyboard_mapping,
};
use crate::ui_elements::action_hint::action_hints_with_labels;
use crate::ui_elements::button::button;
use crate::ui_elements::description::description;
use crate::ui_elements::heading::heading;
use crate::ui_elements::info_message::{InfoMessage, info_message, set_latest_info_message};
use crate::ui_elements::interactions::{
    ActivatedUiElement, DefaultFocusTarget, FocusedUiElement, IgnorePicking, InitialFocus,
    UI_FOCUS_NONE, UiElementColors, UiElementKind, UiElementLabel, UiFocusId, UiFocusNav,
    UiFocusNavIds,
};
use crate::ui_elements::responsive::{
    ResponsiveColumns, ResponsiveFieldRow, ResponsivePercentWidth, ResponsiveScreenPadding,
    UI_PORTRAIT_SCREEN_PADDING,
};
use crate::ui_elements::scroll_view::{ScrollViewConfig, scroll_view};
use crate::ui_elements::styles::{
    UI_CONTROL_FONT_SIZE, UI_ELEMENT_HEIGHT, UI_SCREEN_PADDING, control_fill, hover_fill,
    ui_border, ui_radius,
};
use crate::ui_elements::theme::{
    UiElementTheme, UiThemeBorderColor, UiThemeImageColor, UiThemeTextColor,
};

const CONTENT_WIDTH: f32 = 1500.0;
const CONTENT_GAP: f32 = 18.0;
const COLUMN_GAP: f32 = 42.0;
const FORM_BOTTOM_PADDING: f32 = 72.0;
const LEFT_COLUMN_PERCENT: f32 = 40.0;
const CENTRE_COLUMN_PERCENT: f32 = 28.0;
const RIGHT_COLUMN_PERCENT: f32 = 32.0;
const GAMEBOY_COLUMNS_PERCENT: f32 = LEFT_COLUMN_PERCENT + CENTRE_COLUMN_PERCENT;
const GAMEBOY_LEFT_COLUMN_PERCENT: f32 = LEFT_COLUMN_PERCENT / GAMEBOY_COLUMNS_PERCENT * 100.0;
const GAMEBOY_CENTRE_COLUMN_PERCENT: f32 = CENTRE_COLUMN_PERCENT / GAMEBOY_COLUMNS_PERCENT * 100.0;
const ROW_GAP: f32 = 24.0;
const BUTTON_WIDTH: f32 = 208.0;
const HERO_TEXTURE_SIZE: f32 = 454.0;
const HERO_GRID_UNITS: f32 = 2.0;
const HERO_IMAGE_SIZE: f32 = 184.0;
const CONTROLLER_HERO_X: f32 = 0.0;
const CONTROLLER_HERO_Y: f32 = 0.0;

const TARGET_RESET: u16 = 1;
const TARGET_DPAD_UP: u16 = 10;
const TARGET_DPAD_DOWN: u16 = 11;
const TARGET_DPAD_LEFT: u16 = 12;
const TARGET_DPAD_RIGHT: u16 = 13;
const TARGET_A: u16 = 20;
const TARGET_B: u16 = 21;
const TARGET_START: u16 = 22;
const TARGET_SELECT: u16 = 23;
const TARGET_QUIT_ROM: u16 = 30;
const TARGET_RESET_ROM: u16 = 31;
const TARGET_SAVE_STATE: u16 = 32;
const TARGET_LOAD_STATE: u16 = 33;
const TARGET_SPEED_UP: u16 = 34;
const TARGET_SPEED_DOWN: u16 = 35;
const TARGET_PAUSE: u16 = 36;
const TARGET_QUIT_APP: u16 = 37;

const CORE_LEFT_ROWS: [(InputAction, &str, u16); 4] = [
    (InputAction::Dup, "D-Pad Up", TARGET_DPAD_UP),
    (InputAction::Ddown, "D-Pad Down", TARGET_DPAD_DOWN),
    (InputAction::Dleft, "D-Pad Left", TARGET_DPAD_LEFT),
    (InputAction::Dright, "D-Pad Right", TARGET_DPAD_RIGHT),
];

const CORE_CENTRE_ROWS: [(InputAction, &str, u16); 4] = [
    (InputAction::A, "A", TARGET_A),
    (InputAction::B, "B", TARGET_B),
    (InputAction::Start, "Start", TARGET_START),
    (InputAction::Select, "Select", TARGET_SELECT),
];

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct MappingActionButton {
    action: InputAction,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct MappingButtonLabel;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct MappingResetButton;

#[derive(Clone, Copy, Debug, Default, Resource)]
struct MappingCaptureState {
    action: Option<InputAction>,
    armed: bool,
}

pub struct InputMappingScenePlugin;

impl Plugin for InputMappingScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MappingCaptureState>()
            .add_systems(OnEnter(AppState::InputMapping), spawn_input_mapping_scene)
            .add_systems(OnExit(AppState::InputMapping), clear_mapping_capture)
            .add_systems(
                Update,
                (capture_mapping_input, sync_mapping_button_labels)
                    .chain()
                    .run_if(in_state(AppState::InputMapping)),
            )
            .add_observer(handle_mapping_activation);
    }
}

fn spawn_input_mapping_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    storage: Res<LocalStorage>,
    target: Res<InputMappingEditTarget>,
    primary_input: Res<PrimaryInputDevice>,
) {
    let mapping_index = mapping_index(&target, &storage);
    let mapping = storage
        .data
        .input_mappings
        .get(mapping_index)
        .cloned()
        .unwrap_or_else(default_keyboard_mapping);
    commands.spawn_scene(input_mapping_scene(
        &assets,
        *theme,
        &storage,
        &primary_input,
        &mapping,
    ));
}

fn clear_mapping_capture(mut capture: ResMut<MappingCaptureState>) {
    *capture = MappingCaptureState::default();
}

fn handle_mapping_activation(
    activated: On<Add, ActivatedUiElement>,
    reset_buttons: Query<(), With<MappingResetButton>>,
    action_buttons: Query<&MappingActionButton>,
    mut capture: ResMut<MappingCaptureState>,
    mut storage: ResMut<LocalStorage>,
    target: Res<InputMappingEditTarget>,
    mut runtime_mappings: ResMut<RuntimeInputMappings>,
    state: Res<State<AppState>>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
) {
    if *state.get() != AppState::InputMapping {
        return;
    }

    if let Ok(button) = action_buttons.get(activated.entity) {
        capture.action = Some(button.action);
        capture.armed = false;
        return;
    }

    if reset_buttons.get(activated.entity).is_err() {
        return;
    }

    let index = mapping_index(&target, &storage);
    let Some(existing) = storage.data.input_mappings.get(index).cloned() else {
        return;
    };
    storage.data.input_mappings[index].map = default_entries(&existing);
    persist_mapping_change(
        &mut storage,
        &mut runtime_mappings,
        &mut messages,
        "Input mappings could not be reset.",
    );
}

fn capture_mapping_input(
    mut capture: ResMut<MappingCaptureState>,
    mut keyboard_events: MessageReader<KeyboardInput>,
    mut controller_events: MessageReader<GamepadButtonStateChangedEvent>,
    focused_buttons: Query<&MappingActionButton, With<FocusedUiElement>>,
    mut storage: ResMut<LocalStorage>,
    target: Res<InputMappingEditTarget>,
    mut runtime_mappings: ResMut<RuntimeInputMappings>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
) {
    let keyboard_key = keyboard_events.read().find_map(|event| {
        (!event.repeat && event.state == ButtonState::Pressed)
            .then(|| key_code_id(event.key_code))
            .flatten()
    });
    let controller_key = controller_events.read().find_map(|event| {
        (event.state == ButtonState::Pressed)
            .then(|| gamepad_button_id(event.button))
            .flatten()
    });

    let index = mapping_index(&target, &storage);
    let Some(mapping) = storage.data.input_mappings.get(index) else {
        return;
    };
    let pressed_key = match mapping.r#type {
        InputDeviceType::Keyboard => keyboard_key,
        InputDeviceType::Controller => controller_key,
    };
    let b_pressed = pressed_key.is_some_and(|key_id| {
        mapping
            .map
            .iter()
            .any(|entry| entry.key_id == key_id && entry.map_to == InputAction::B)
    });

    if capture.action.is_some() && !capture.armed {
        capture.armed = true;
        return;
    }

    if let Some(action) = capture.action {
        if b_pressed {
            capture.action = None;
            capture.armed = false;
            return;
        }

        let Some(key_id) = pressed_key else {
            return;
        };

        set_action_mapping(
            &mut storage.data.input_mappings[index],
            action,
            Some(key_id),
        );
        persist_mapping_change(
            &mut storage,
            &mut runtime_mappings,
            &mut messages,
            "Input mappings could not be saved.",
        );
        capture.action = None;
        capture.armed = false;
    } else if b_pressed {
        let Some(button) = focused_buttons.iter().next() else {
            return;
        };
        if index >= storage.data.input_mappings.len() {
            return;
        }
        set_action_mapping(&mut storage.data.input_mappings[index], button.action, None);
        persist_mapping_change(
            &mut storage,
            &mut runtime_mappings,
            &mut messages,
            "Input mappings could not be saved.",
        );
    }
}

fn sync_mapping_button_labels(
    storage: Res<LocalStorage>,
    target: Res<InputMappingEditTarget>,
    capture: Res<MappingCaptureState>,
    buttons: Query<(&MappingActionButton, &Children)>,
    mut labels: Query<&mut Text, With<MappingButtonLabel>>,
) {
    let index = mapping_index(&target, &storage);
    let Some(mapping) = storage.data.input_mappings.get(index) else {
        return;
    };

    for (button, children) in &buttons {
        let label = mapping_value_label(
            mapping,
            button.action,
            capture.action == Some(button.action),
        );
        for child in children {
            if let Ok(mut text) = labels.get_mut(*child) {
                text.0 = label.clone();
            }
        }
    }
}

fn input_mapping_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    storage: &LocalStorage,
    primary_input: &PrimaryInputDevice,
    mapping: &InputDeviceMapping,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let content_font = font.clone();
    let heroes = assets.heroes.clone();

    bsn! {
        #InputMappingScene
        DespawnOnExit::<AppState>(AppState::InputMapping)
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
                    max_width: px(CONTENT_WIDTH),
                    height: percent(100),
                    min_height: px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(CONTENT_GAP),
                }
                Children [
                    heading(font.clone(), theme, "Input Mapping"),
                    (
                        #InputMappingScrollBar
                        scroll_view(
                            theme,
                            #InputMappingScrollBar,
                            ScrollViewConfig {
                                width: percent(100),
                                min_height: px(0.0),
                                thumb_height: 110.0,
                            },
                            move |_| mapping_content(content_font, heroes, theme, mapping.clone())
                        )
                    ),
                    info_message(font.clone(), theme, "", false),
                    action_hints_with_labels(font, assets.icons.clone(), theme, storage, primary_input, "Back", "Select"),
                ]
            )
        ]
    }
}

fn mapping_content(
    font: Handle<Font>,
    heroes: Handle<Image>,
    theme: ActiveTheme,
    mapping: InputDeviceMapping,
) -> impl Scene {
    let left_font = font.clone();
    let centre_font = font.clone();
    let right_font = font.clone();
    let mapping_name = mapping_label(&mapping);
    let save_action = save_state_action(mapping.r#type);
    let load_action = load_state_action(mapping.r#type);

    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            column_gap: px(COLUMN_GAP),
            padding: UiRect {
                left: px(0.0),
                right: px(28.0),
                top: px(0.0),
                bottom: px(FORM_BOTTOM_PADDING),
            },
        }
        ResponsiveColumns { gap: COLUMN_GAP }
        Children [
            gameboy_mapping_panel(left_font, centre_font, heroes, theme, mapping.clone(), mapping_name),
            right_column(right_font, theme, mapping, save_action, load_action),
        ]
    }
}

fn gameboy_mapping_panel(
    left_font: Handle<Font>,
    centre_font: Handle<Font>,
    heroes: Handle<Image>,
    theme: ActiveTheme,
    mapping: InputDeviceMapping,
    mapping_name: String,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(GAMEBOY_COLUMNS_PERCENT),
            flex_direction: FlexDirection::Column,
            row_gap: px(54.0),
        }
        ResponsivePercentWidth { landscape: GAMEBOY_COLUMNS_PERCENT }
        Children [
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(COLUMN_GAP),
                }
                ResponsiveColumns { gap: COLUMN_GAP }
                Children [
                    mapping_intro(left_font.clone(), theme, mapping_name),
                    controller_hero_image(heroes, theme),
                ]
            ),
            (
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(COLUMN_GAP),
                }
                ResponsiveColumns { gap: COLUMN_GAP }
                Children [
                    core_left_column(left_font, theme, mapping.clone()),
                    core_centre_column(centre_font, theme, mapping),
                ]
            ),
        ]
    }
}

fn mapping_intro(font: Handle<Font>, theme: ActiveTheme, mapping_name: String) -> impl Scene {
    bsn! {
        Node {
            width: percent(GAMEBOY_LEFT_COLUMN_PERCENT),
            flex_direction: FlexDirection::Column,
        }
        ResponsivePercentWidth { landscape: GAMEBOY_LEFT_COLUMN_PERCENT }
        Children [
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(28.0),
                }
                ResponsiveFieldRow { gap: 28.0 }
                Children [
                    (
                        Node {
                            flex_grow: 1.0,
                            align_items: AlignItems::Center,
                            column_gap: px(120.0),
                        }
                        Children [
                            description(font.clone(), theme, "Name:"),
                            description(font.clone(), theme, mapping_name),
                        ]
                    ),
                    (
                        button(font.clone(), "Reset Defaults", theme, UiFocusNav::default())
                        MappingResetButton
                        UiFocusId { id: TARGET_RESET }
                        UiFocusNavIds { up: UI_FOCUS_NONE, right: TARGET_QUIT_ROM, down: TARGET_DPAD_UP, left: UI_FOCUS_NONE }
                    ),
                ]
            ),
            (
                Node {
                    width: percent(100),
                    height: px(32.0),
                }
            ),
            description(font.clone(), theme, "Map buttons on your input device to emulated GameBoy buttons. These will be used in-game as well as in menus."),
            (
                Node {
                    width: percent(100),
                    height: px(18.0),
                }
            ),
            description(font, theme, "Other actions can be mapped to buttons, or left unassigned. Use the button assigned to the A key to map the next button press to the given action (and B to cancel), or press B to unset an action."),
        ]
    }
}

fn core_left_column(
    font: Handle<Font>,
    theme: ActiveTheme,
    mapping: InputDeviceMapping,
) -> impl Scene {
    let rows = CORE_LEFT_ROWS
        .iter()
        .map(|(action, label, target)| {
            mapping_row(
                font.clone(),
                theme,
                mapping.clone(),
                *action,
                label,
                *target,
                left_nav(*target),
            )
        })
        .collect::<Vec<_>>();

    bsn! {
        Node {
            width: percent(GAMEBOY_LEFT_COLUMN_PERCENT),
            flex_direction: FlexDirection::Column,
            row_gap: px(ROW_GAP),
        }
        ResponsivePercentWidth { landscape: GAMEBOY_LEFT_COLUMN_PERCENT }
        Children [
            {rows}
        ]
    }
}

fn core_centre_column(
    font: Handle<Font>,
    theme: ActiveTheme,
    mapping: InputDeviceMapping,
) -> impl Scene {
    let rows = CORE_CENTRE_ROWS
        .iter()
        .map(|(action, label, target)| {
            mapping_row(
                font.clone(),
                theme,
                mapping.clone(),
                *action,
                label,
                *target,
                centre_nav(*target),
            )
        })
        .collect::<Vec<_>>();

    bsn! {
        Node {
            width: percent(GAMEBOY_CENTRE_COLUMN_PERCENT),
            flex_direction: FlexDirection::Column,
            row_gap: px(ROW_GAP),
        }
        ResponsivePercentWidth { landscape: GAMEBOY_CENTRE_COLUMN_PERCENT }
        Children [
            {rows}
        ]
    }
}

fn right_column(
    font: Handle<Font>,
    theme: ActiveTheme,
    mapping: InputDeviceMapping,
    save_action: InputAction,
    load_action: InputAction,
) -> impl Scene {
    let rows = vec![
        mapping_row(
            font.clone(),
            theme,
            mapping.clone(),
            InputAction::QuitRom,
            "Shut down ROM",
            TARGET_QUIT_ROM,
            right_nav(TARGET_QUIT_ROM),
        ),
        mapping_row(
            font.clone(),
            theme,
            mapping.clone(),
            InputAction::ResetRom,
            "Reset ROM",
            TARGET_RESET_ROM,
            right_nav(TARGET_RESET_ROM),
        ),
        mapping_row(
            font.clone(),
            theme,
            mapping.clone(),
            save_action,
            "Save state",
            TARGET_SAVE_STATE,
            right_nav(TARGET_SAVE_STATE),
        ),
        mapping_row(
            font.clone(),
            theme,
            mapping.clone(),
            load_action,
            "Load state",
            TARGET_LOAD_STATE,
            right_nav(TARGET_LOAD_STATE),
        ),
        mapping_row(
            font.clone(),
            theme,
            mapping.clone(),
            InputAction::SpeedUp,
            "Speed up",
            TARGET_SPEED_UP,
            right_nav(TARGET_SPEED_UP),
        ),
        mapping_row(
            font.clone(),
            theme,
            mapping.clone(),
            InputAction::SpeedDown,
            "Speed down",
            TARGET_SPEED_DOWN,
            right_nav(TARGET_SPEED_DOWN),
        ),
        mapping_row(
            font.clone(),
            theme,
            mapping.clone(),
            InputAction::PauseAndResume,
            "Pause/resume",
            TARGET_PAUSE,
            right_nav(TARGET_PAUSE),
        ),
        mapping_row(
            font,
            theme,
            mapping.clone(),
            InputAction::QuitApp,
            "Quit app",
            TARGET_QUIT_APP,
            right_nav(TARGET_QUIT_APP),
        ),
    ];

    bsn! {
        Node {
            width: percent(RIGHT_COLUMN_PERCENT),
            flex_direction: FlexDirection::Column,
            row_gap: px(ROW_GAP),
            padding: UiRect::top(px(6.0)),
        }
        ResponsivePercentWidth { landscape: RIGHT_COLUMN_PERCENT }
        Children [
            {rows}
        ]
    }
}

fn mapping_row(
    font: Handle<Font>,
    theme: ActiveTheme,
    mapping: InputDeviceMapping,
    action: InputAction,
    label: &'static str,
    target: u16,
    nav: UiFocusNavIds,
) -> impl Scene {
    let value = mapping_value_label(&mapping, action, false);

    bsn! {
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(20.0),
        }
        ResponsiveFieldRow { gap: 20.0 }
        Children [
            (
                Node {
                    flex_grow: 1.0,
                    min_width: px(0.0),
                }
                Children [
                    description(font.clone(), theme, label)
                ]
            ),
            (
                mapping_button(font, theme, value)
                MappingActionButton { action: {action} }
                UiFocusId { id: target }
                UiFocusNavIds { up: {nav.up}, right: {nav.right}, down: {nav.down}, left: {nav.left} }
                InitialFocus { enabled: {target == TARGET_DPAD_UP} }
                DefaultFocusTarget
            )
        ]
    }
}

fn mapping_button(font: Handle<Font>, theme: ActiveTheme, label: String) -> impl Scene {
    let background = control_fill(&theme);
    let hover_background = hover_fill(&theme);
    let nav = UiFocusNav::default();

    bsn! {
        Node {
            width: px(BUTTON_WIDTH),
            height: px(UI_ELEMENT_HEIGHT),
            flex_shrink: 0.0,
            border: ui_border(),
            border_radius: ui_radius(),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::horizontal(px(12.0)),
        }
        Button
        BorderColor::all(theme.primary)
        UiThemeBorderColor::Primary
        BackgroundColor({background})
        UiFocusNav { up: {nav.up}, right: {nav.right}, down: {nav.down}, left: {nav.left} }
        UiElementKind::Button
        UiElementTheme::Control
        UiElementColors { primary: {theme.primary}, secondary: {theme.secondary}, tertiary: {theme.tertiary}, fill: {background}, hover_fill: {hover_background} }
        Children [
            (
                Text({label})
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_CONTROL_FONT_SIZE),
                }
                TextColor({theme.primary})
                UiThemeTextColor::Primary
                UiElementLabel
                MappingButtonLabel
                IgnorePicking
                TextLayout::new(Justify::Center, LineBreak::NoWrap)
            )
        ]
    }
}

fn controller_hero_image(image: Handle<Image>, theme: ActiveTheme) -> impl Scene {
    bsn! {
        Node {
            width: percent(GAMEBOY_CENTRE_COLUMN_PERCENT),
            min_height: px(196.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::vertical(px(6.0)),
        }
        ResponsivePercentWidth { landscape: GAMEBOY_CENTRE_COLUMN_PERCENT }
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
                    rect: {Some(hero_grid_rect(CONTROLLER_HERO_X, CONTROLLER_HERO_Y))},
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

fn persist_mapping_change(
    storage: &mut LocalStorage,
    runtime_mappings: &mut RuntimeInputMappings,
    messages: &mut Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
    error_message: &str,
) {
    if let Err(error) = storage.save_input_mappings() {
        eprintln!("failed to save input mappings: {error}");
        set_latest_info_message(messages, error_message);
        return;
    }
    runtime_mappings.rebuild(&storage.data.input_mappings);
}

fn set_action_mapping(
    mapping: &mut InputDeviceMapping,
    action: InputAction,
    key_id: Option<InputKeyId>,
) {
    mapping
        .map
        .retain(|entry| entry.map_to != action && Some(entry.key_id) != key_id);
    if let Some(key_id) = key_id {
        mapping.map.push(InputMapEntry {
            key_id,
            map_to: action,
        });
    }
    ensure_essential_navigation_mappings(mapping);
}

fn mapping_index(target: &InputMappingEditTarget, storage: &LocalStorage) -> usize {
    if storage.data.input_mappings.is_empty() {
        0
    } else {
        target
            .mapping_index
            .min(storage.data.input_mappings.len().saturating_sub(1))
    }
}

fn mapping_value_label(
    mapping: &InputDeviceMapping,
    action: InputAction,
    listening: bool,
) -> String {
    if listening {
        return "Listening...".to_string();
    }

    mapping
        .map
        .iter()
        .find_map(|entry| (entry.map_to == action).then_some(key_id_label(entry.key_id)))
        .unwrap_or_else(|| "(Unset)".to_string())
}

fn key_id_label(key_id: InputKeyId) -> String {
    match key_id {
        InputKeyId::ArrowLeft => "Left Arrow",
        InputKeyId::ArrowRight => "Right Arrow",
        InputKeyId::ArrowUp => "Up Arrow",
        InputKeyId::ArrowDown => "Down Arrow",
        InputKeyId::KeyA => "A",
        InputKeyId::KeyB => "B",
        InputKeyId::KeyC => "C",
        InputKeyId::KeyD => "D",
        InputKeyId::KeyE => "E",
        InputKeyId::KeyF => "F",
        InputKeyId::KeyG => "G",
        InputKeyId::KeyH => "H",
        InputKeyId::KeyI => "I",
        InputKeyId::KeyJ => "J",
        InputKeyId::KeyK => "K",
        InputKeyId::KeyL => "L",
        InputKeyId::KeyM => "M",
        InputKeyId::KeyN => "N",
        InputKeyId::KeyO => "O",
        InputKeyId::KeyP => "P",
        InputKeyId::KeyQ => "Q",
        InputKeyId::KeyR => "R",
        InputKeyId::KeyS => "S",
        InputKeyId::KeyT => "T",
        InputKeyId::KeyU => "U",
        InputKeyId::KeyV => "V",
        InputKeyId::KeyW => "W",
        InputKeyId::KeyZ => "Z",
        InputKeyId::KeyX => "X",
        InputKeyId::KeyY => "Y",
        InputKeyId::Enter => "Enter",
        InputKeyId::ShiftRight => "Right Shift",
        InputKeyId::ShiftLeft => "Left Shift",
        InputKeyId::Escape => "Escape",
        InputKeyId::ControlLeft => "Left Ctrl",
        InputKeyId::ControlRight => "Right Ctrl",
        InputKeyId::Space => "Space",
        InputKeyId::Tab => "Tab",
        InputKeyId::Backspace => "Backspace",
        InputKeyId::Digit0 => "0",
        InputKeyId::Digit1 => "1",
        InputKeyId::Digit2 => "2",
        InputKeyId::Digit3 => "3",
        InputKeyId::Digit4 => "4",
        InputKeyId::Digit5 => "5",
        InputKeyId::Digit6 => "6",
        InputKeyId::Digit7 => "7",
        InputKeyId::Digit8 => "8",
        InputKeyId::Digit9 => "9",
        InputKeyId::South => "A",
        InputKeyId::East => "B",
        InputKeyId::North => "Y",
        InputKeyId::West => "X",
        InputKeyId::C => "C",
        InputKeyId::Z => "Z",
        InputKeyId::LeftTrigger => "Left shoulder",
        InputKeyId::LeftTrigger2 => "Left trigger",
        InputKeyId::RightTrigger => "Right shoulder",
        InputKeyId::RightTrigger2 => "Right trigger",
        InputKeyId::Select => "SEL",
        InputKeyId::Start => "START",
        InputKeyId::Mode => "Home",
        InputKeyId::LeftThumb => "Left stick",
        InputKeyId::RightThumb => "Right stick",
        InputKeyId::DPadUp => "DUP",
        InputKeyId::DPadDown => "DDOWN",
        InputKeyId::DPadLeft => "DLEFT",
        InputKeyId::DPadRight => "DRIGHT",
    }
    .to_string()
}

fn save_state_action(device_type: InputDeviceType) -> InputAction {
    match device_type {
        InputDeviceType::Keyboard => InputAction::SaveStateModifier,
        InputDeviceType::Controller => InputAction::SaveState0,
    }
}

fn load_state_action(device_type: InputDeviceType) -> InputAction {
    match device_type {
        InputDeviceType::Keyboard => InputAction::LoadStateModifier,
        InputDeviceType::Controller => InputAction::LoadState0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyboard_mapping(entries: Vec<InputMapEntry>) -> InputDeviceMapping {
        InputDeviceMapping {
            r#type: InputDeviceType::Keyboard,
            controller_model_id: None,
            map: entries,
        }
    }

    fn entry(key_id: InputKeyId, map_to: InputAction) -> InputMapEntry {
        InputMapEntry { key_id, map_to }
    }

    #[test]
    fn setting_special_action_removes_existing_mapping_for_same_key() {
        let mut mapping = keyboard_mapping(vec![
            entry(InputKeyId::KeyP, InputAction::A),
            entry(InputKeyId::KeyQ, InputAction::QuitRom),
            entry(InputKeyId::KeyZ, InputAction::B),
        ]);

        set_action_mapping(&mut mapping, InputAction::QuitRom, Some(InputKeyId::KeyP));

        assert!(
            mapping
                .map
                .iter()
                .any(|entry| entry.key_id == InputKeyId::KeyP
                    && entry.map_to == InputAction::QuitRom)
        );
        assert!(
            !mapping
                .map
                .iter()
                .any(|entry| entry.key_id == InputKeyId::KeyP && entry.map_to == InputAction::A)
        );
        assert!(
            !mapping
                .map
                .iter()
                .any(|entry| entry.key_id == InputKeyId::KeyQ
                    && entry.map_to == InputAction::QuitRom)
        );
    }

    #[test]
    fn setting_game_boy_action_removes_existing_mapping_for_same_key() {
        let mut mapping = keyboard_mapping(vec![
            entry(InputKeyId::KeyP, InputAction::QuitRom),
            entry(InputKeyId::KeyQ, InputAction::A),
            entry(InputKeyId::KeyZ, InputAction::B),
        ]);

        set_action_mapping(&mut mapping, InputAction::A, Some(InputKeyId::KeyP));

        assert!(
            mapping
                .map
                .iter()
                .any(|entry| entry.key_id == InputKeyId::KeyP && entry.map_to == InputAction::A)
        );
        assert!(
            !mapping
                .map
                .iter()
                .any(|entry| entry.key_id == InputKeyId::KeyP
                    && entry.map_to == InputAction::QuitRom)
        );
        assert!(
            !mapping
                .map
                .iter()
                .any(|entry| entry.key_id == InputKeyId::KeyQ && entry.map_to == InputAction::A)
        );
    }
}

fn left_nav(target: u16) -> UiFocusNavIds {
    match target {
        TARGET_DPAD_UP => nav(TARGET_RESET, TARGET_A, TARGET_DPAD_DOWN, UI_FOCUS_NONE),
        TARGET_DPAD_DOWN => nav(TARGET_DPAD_UP, TARGET_B, TARGET_DPAD_LEFT, UI_FOCUS_NONE),
        TARGET_DPAD_LEFT => nav(
            TARGET_DPAD_DOWN,
            TARGET_START,
            TARGET_DPAD_RIGHT,
            UI_FOCUS_NONE,
        ),
        TARGET_DPAD_RIGHT => nav(
            TARGET_DPAD_LEFT,
            TARGET_SELECT,
            UI_FOCUS_NONE,
            UI_FOCUS_NONE,
        ),
        _ => nav(UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE),
    }
}

fn centre_nav(target: u16) -> UiFocusNavIds {
    match target {
        TARGET_A => nav(TARGET_RESET, TARGET_QUIT_ROM, TARGET_B, TARGET_DPAD_UP),
        TARGET_B => nav(TARGET_A, TARGET_RESET_ROM, TARGET_START, TARGET_DPAD_DOWN),
        TARGET_START => nav(TARGET_B, TARGET_SAVE_STATE, TARGET_SELECT, TARGET_DPAD_LEFT),
        TARGET_SELECT => nav(
            TARGET_START,
            TARGET_LOAD_STATE,
            UI_FOCUS_NONE,
            TARGET_DPAD_RIGHT,
        ),
        _ => nav(UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE),
    }
}

fn right_nav(target: u16) -> UiFocusNavIds {
    match target {
        TARGET_QUIT_ROM => nav(UI_FOCUS_NONE, UI_FOCUS_NONE, TARGET_RESET_ROM, TARGET_A),
        TARGET_RESET_ROM => nav(TARGET_QUIT_ROM, UI_FOCUS_NONE, TARGET_SAVE_STATE, TARGET_B),
        TARGET_SAVE_STATE => nav(
            TARGET_RESET_ROM,
            UI_FOCUS_NONE,
            TARGET_LOAD_STATE,
            TARGET_START,
        ),
        TARGET_LOAD_STATE => nav(
            TARGET_SAVE_STATE,
            UI_FOCUS_NONE,
            TARGET_SPEED_UP,
            TARGET_SELECT,
        ),
        TARGET_SPEED_UP => nav(
            TARGET_LOAD_STATE,
            UI_FOCUS_NONE,
            TARGET_SPEED_DOWN,
            TARGET_SELECT,
        ),
        TARGET_SPEED_DOWN => nav(TARGET_SPEED_UP, UI_FOCUS_NONE, TARGET_PAUSE, TARGET_SELECT),
        TARGET_PAUSE => nav(
            TARGET_SPEED_DOWN,
            UI_FOCUS_NONE,
            TARGET_QUIT_APP,
            TARGET_SELECT,
        ),
        TARGET_QUIT_APP => nav(TARGET_PAUSE, UI_FOCUS_NONE, UI_FOCUS_NONE, TARGET_SELECT),
        _ => nav(UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE, UI_FOCUS_NONE),
    }
}

fn nav(up: u16, right: u16, down: u16, left: u16) -> UiFocusNavIds {
    UiFocusNavIds {
        up,
        right,
        down,
        left,
    }
}

fn default_entries(mapping: &InputDeviceMapping) -> Vec<InputMapEntry> {
    match mapping.r#type {
        InputDeviceType::Keyboard => default_keyboard_mapping().map,
        InputDeviceType::Controller => {
            default_controller_mapping(
                mapping
                    .controller_model_id
                    .clone()
                    .unwrap_or_else(|| "Controller".to_string()),
            )
            .map
        }
    }
}
