use bevy::asset::HandleTemplate;
use bevy::math::Rect;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::dimensions::{ACTION_HINT_GAP, ACTION_HINT_ICON_SIZE, UI_BODY_FONT_SIZE};
use crate::input::controller::ConnectedControllers;
use crate::input::selection::{
    PrimaryInputDevice, selected_mapping, selected_mapping_has_available_device,
};
use crate::storage::LocalStorage;
use crate::storage::input_mappings::{
    InputAction, InputDeviceMapping, InputDeviceType, InputKeyId,
};
use crate::ui_elements::interactions::IgnorePicking;
use crate::ui_elements::theme::UiThemeTextColor;

const ICON_TEXTURE_SIZE: f32 = 1024.0;
const ICON_GRID_UNITS: f32 = 16.0;
const GENERIC_BUTTON_ICON_X: f32 = 12.0;
const GENERIC_BUTTON_ICON_Y: f32 = 0.0;
const GENERIC_BUTTON_ICON_SIZE: f32 = 4.0;
const XBOX_BUTTON_X_X: f32 = 8.0;
const XBOX_BUTTON_X_Y: f32 = 8.0;
const XBOX_BUTTON_Y_X: f32 = 10.0;
const XBOX_BUTTON_Y_Y: f32 = 8.0;
const XBOX_BUTTON_A_X: f32 = 8.0;
const XBOX_BUTTON_A_Y: f32 = 10.0;
const XBOX_BUTTON_B_X: f32 = 10.0;
const XBOX_BUTTON_B_Y: f32 = 10.0;
const PLAYSTATION_BUTTON_SQUARE_X: f32 = 12.0;
const PLAYSTATION_BUTTON_SQUARE_Y: f32 = 8.0;
const PLAYSTATION_BUTTON_TRIANGLE_X: f32 = 14.0;
const PLAYSTATION_BUTTON_TRIANGLE_Y: f32 = 8.0;
const PLAYSTATION_BUTTON_CROSS_X: f32 = 12.0;
const PLAYSTATION_BUTTON_CROSS_Y: f32 = 10.0;
const PLAYSTATION_BUTTON_CIRCLE_X: f32 = 14.0;
const PLAYSTATION_BUTTON_CIRCLE_Y: f32 = 10.0;
const PLATFORM_FACE_BUTTON_ICON_SIZE: f32 = 2.0;
const TRIGGER_ICON_X: f32 = 8.0;
const TRIGGER_ICON_Y: f32 = 4.0;
const SHOULDER_ICON_X: f32 = 12.0;
const SHOULDER_ICON_Y: f32 = 4.0;
const SELECT_ICON_X: f32 = 8.0;
const SELECT_ICON_Y: f32 = 0.0;
const START_ICON_X: f32 = 8.0;
const START_ICON_Y: f32 = 2.0;
const SMALL_ICON_SIZE: f32 = 4.0;

pub fn action_hints(
    font: Handle<Font>,
    icons: Handle<Image>,
    theme: ActiveTheme,
    storage: &LocalStorage,
    primary_input: &PrimaryInputDevice,
) -> impl Scene {
    action_hints_with_labels(font, icons, theme, storage, primary_input, "Quit", "Select")
}

pub fn action_hints_with_labels(
    font: Handle<Font>,
    icons: Handle<Image>,
    theme: ActiveTheme,
    storage: &LocalStorage,
    primary_input: &PrimaryInputDevice,
    back_label: &'static str,
    action_label: &'static str,
) -> impl Scene {
    action_hints_for_actions(
        font,
        icons,
        theme,
        storage,
        primary_input,
        (InputAction::B, back_label),
        (InputAction::A, action_label),
    )
}

pub fn action_hints_for_actions(
    font: Handle<Font>,
    icons: Handle<Image>,
    theme: ActiveTheme,
    storage: &LocalStorage,
    primary_input: &PrimaryInputDevice,
    first: (InputAction, &'static str),
    second: (InputAction, &'static str),
) -> impl Scene {
    let mapping = selected_mapping(primary_input, storage);
    let first_key = action_key_hint(mapping, first.0);
    let second_key = action_key_hint(mapping, second.0);

    bsn! {
        Node {
            width: percent(100),
            justify_content: JustifyContent::FlexEnd,
            column_gap: px(ACTION_HINT_GAP),
        }
        ActionHintRoot
        Children [
            action_hint(font.clone(), icons.clone(), theme, first.0, first_key, first.1),
            action_hint(font, icons, theme, second.0, second_key, second.1)
        ]
    }
}

pub struct ActionHintPlugin;

impl Plugin for ActionHintPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_action_hints);
    }
}

fn action_hint(
    font: Handle<Font>,
    icons: Handle<Image>,
    theme: ActiveTheme,
    action: InputAction,
    key: ActionKeyHint,
    label: &'static str,
) -> impl Scene {
    let key_font = font.clone();
    let background_icons = icons.clone();
    let foreground_icons = icons;
    let key_label = key.label;
    let background_rect = key.background_rect;
    let background_tint = key.background_tint.resolve(&theme);
    let foreground_rect = key.foreground_rect.unwrap_or_else(generic_button_rect);
    let foreground_tint = key.foreground_tint.resolve(&theme);
    let foreground_display = if key.foreground_rect.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    let text_display = if key_label.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    bsn! {
        Node {
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Row,
            column_gap: px(18.0),
        }
        IgnorePicking
        Children [
            (
                Node {
                    width: px(ACTION_HINT_ICON_SIZE),
                    height: px(ACTION_HINT_ICON_SIZE),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                ActionHintIcon { action: {action} }
                IgnorePicking
                Children [
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(0.0),
                            top: px(0.0),
                            width: percent(100),
                            height: percent(100),
                        }
                        ImageNode {
                            image: HandleTemplate::Handle(background_icons),
                            color: {background_tint},
                            rect: {Some(background_rect)},
                        }
                        ActionHintBackgroundIcon
                        IgnorePicking
                    ),
                    (
                        Node {
                            display: {foreground_display},
                            position_type: PositionType::Absolute,
                            left: px(0.0),
                            top: px(0.0),
                            width: percent(100),
                            height: percent(100),
                        }
                        ImageNode {
                            image: HandleTemplate::Handle(foreground_icons),
                            color: {foreground_tint},
                            rect: {Some(foreground_rect)},
                        }
                        ActionHintForegroundIcon
                        IgnorePicking
                    ),
                    (
                        Text({key_label})
                        Node {
                            display: {text_display},
                        }
                        TextFont {
                            font: FontSourceTemplate::Handle(HandleTemplate::Handle(key_font)),
                            font_size: px(UI_BODY_FONT_SIZE),
                        }
                        TextColor({theme.primary})
                        UiThemeTextColor::Primary
                        ActionHintKeyLabel
                        IgnorePicking
                    )
                ]
            ),
            (
                Text({label})
                TextFont {
                    font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
                    font_size: px(UI_BODY_FONT_SIZE),
                }
                TextColor({theme.primary})
                UiThemeTextColor::Primary
                IgnorePicking
            )
        ]
    }
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
struct ActionHintIcon {
    action: InputAction,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct ActionHintBackgroundIcon;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct ActionHintForegroundIcon;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct ActionHintRoot;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct ActionHintKeyLabel;

#[derive(Clone)]
struct ActionKeyHint {
    label: String,
    background_rect: Rect,
    background_tint: ActionHintTint,
    foreground_rect: Option<Rect>,
    foreground_tint: ActionHintTint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ActionHintTint {
    Primary,
    White,
}

impl ActionHintTint {
    fn resolve(self, theme: &ActiveTheme) -> Color {
        match self {
            Self::Primary => theme.primary,
            Self::White => Color::WHITE,
        }
    }
}

fn sync_action_hints(
    storage: Option<Res<LocalStorage>>,
    primary_input: Option<Res<PrimaryInputDevice>>,
    controllers: Option<Res<ConnectedControllers>>,
    theme: Option<Res<ActiveTheme>>,
    mut roots: Query<
        &mut Node,
        (
            With<ActionHintRoot>,
            Without<ActionHintKeyLabel>,
            Without<ActionHintForegroundIcon>,
        ),
    >,
    hints: Query<(&ActionHintIcon, &Children)>,
    mut backgrounds: Query<
        &mut ImageNode,
        (
            With<ActionHintBackgroundIcon>,
            Without<ActionHintForegroundIcon>,
        ),
    >,
    mut foregrounds: Query<
        (&mut ImageNode, &mut Node),
        (
            With<ActionHintForegroundIcon>,
            Without<ActionHintBackgroundIcon>,
            Without<ActionHintKeyLabel>,
            Without<ActionHintRoot>,
        ),
    >,
    mut labels: Query<
        (&mut Text, &mut Node),
        (
            With<ActionHintKeyLabel>,
            Without<ActionHintRoot>,
            Without<ActionHintForegroundIcon>,
        ),
    >,
) {
    let (Some(storage), Some(primary_input), Some(controllers), Some(theme)) =
        (storage, primary_input, controllers, theme)
    else {
        return;
    };
    let display = if selected_mapping_has_available_device(&primary_input, &storage, &controllers) {
        Display::Flex
    } else {
        Display::None
    };
    for mut root in &mut roots {
        root.display = display;
    }

    let mapping = selected_mapping(&primary_input, &storage);
    for (icon, children) in &hints {
        let key = action_key_hint(mapping, icon.action);
        let foreground_display = if key.foreground_rect.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        for child in children {
            if let Ok(mut background) = backgrounds.get_mut(*child) {
                background.rect = Some(key.background_rect);
                background.color = key.background_tint.resolve(&theme);
            }
            if let Ok((mut foreground, mut node)) = foregrounds.get_mut(*child) {
                foreground.rect = Some(key.foreground_rect.unwrap_or_else(generic_button_rect));
                foreground.color = key.foreground_tint.resolve(&theme);
                node.display = foreground_display;
            }
            if let Ok((mut text, mut node)) = labels.get_mut(*child) {
                text.0 = key.label.clone();
                node.display = if key.label.is_empty() {
                    Display::None
                } else {
                    Display::Flex
                };
            }
        }
    }
}

fn action_key_hint(mapping: Option<&InputDeviceMapping>, action: InputAction) -> ActionKeyHint {
    let key_id = mapping.and_then(|mapping| {
        mapping
            .map
            .iter()
            .find_map(|entry| (entry.map_to == action).then_some(entry.key_id))
    });
    let device_type = mapping
        .map(|mapping| mapping.r#type)
        .unwrap_or(InputDeviceType::Keyboard);

    match (device_type, key_id) {
        (InputDeviceType::Controller, Some(key_id)) => controller_action_key_hint(mapping, key_id),
        (_, Some(key_id)) => labelled_generic_button_hint(key_id_label(key_id)),
        _ => unlabelled_generic_button_hint(),
    }
}

fn controller_action_key_hint(
    mapping: Option<&InputDeviceMapping>,
    key_id: InputKeyId,
) -> ActionKeyHint {
    if let Some(face_button) = platform_face_button_hint(mapping, key_id) {
        return face_button;
    }

    let label = controller_key_id_label(key_id);
    if label.is_empty() {
        icon_only_hint(controller_icon_rect(key_id), ActionHintTint::Primary)
    } else {
        ActionKeyHint {
            label,
            background_rect: controller_icon_rect(key_id),
            background_tint: ActionHintTint::Primary,
            foreground_rect: None,
            foreground_tint: ActionHintTint::Primary,
        }
    }
}

fn key_id_label(key_id: InputKeyId) -> String {
    match key_id {
        InputKeyId::ArrowLeft => "Left",
        InputKeyId::ArrowRight => "Right",
        InputKeyId::ArrowUp => "Up",
        InputKeyId::ArrowDown => "Down",
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
        InputKeyId::ShiftRight | InputKeyId::ShiftLeft => "Shift",
        InputKeyId::Escape => "Esc",
        InputKeyId::ControlLeft | InputKeyId::ControlRight => "Ctrl",
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
        _ => "?",
    }
    .to_string()
}

fn controller_key_id_label(key_id: InputKeyId) -> String {
    match key_id {
        InputKeyId::LeftTrigger => "LB",
        InputKeyId::RightTrigger => "RB",
        InputKeyId::LeftTrigger2 => "LT",
        InputKeyId::RightTrigger2 => "RT",
        InputKeyId::Mode => "Home",
        InputKeyId::LeftThumb => "L3",
        InputKeyId::RightThumb => "R3",
        _ => "",
    }
    .to_string()
}

fn labelled_generic_button_hint(label: String) -> ActionKeyHint {
    ActionKeyHint {
        label,
        background_rect: generic_button_rect(),
        background_tint: ActionHintTint::Primary,
        foreground_rect: None,
        foreground_tint: ActionHintTint::Primary,
    }
}

fn unlabelled_generic_button_hint() -> ActionKeyHint {
    ActionKeyHint {
        label: String::new(),
        background_rect: generic_button_rect(),
        background_tint: ActionHintTint::Primary,
        foreground_rect: None,
        foreground_tint: ActionHintTint::Primary,
    }
}

fn icon_only_hint(rect: Rect, tint: ActionHintTint) -> ActionKeyHint {
    ActionKeyHint {
        label: String::new(),
        background_rect: rect,
        background_tint: tint,
        foreground_rect: None,
        foreground_tint: ActionHintTint::White,
    }
}

fn platform_face_button_hint(
    mapping: Option<&InputDeviceMapping>,
    key_id: InputKeyId,
) -> Option<ActionKeyHint> {
    let platform = mapping
        .and_then(|mapping| mapping.controller_model_id.as_deref())
        .map(controller_button_platform)
        .unwrap_or(ControllerButtonPlatform::Xbox);

    match (platform, key_id) {
        (ControllerButtonPlatform::Xbox, InputKeyId::South) => {
            Some(platform_button_hint(xbox_button_a_rect()))
        }
        (ControllerButtonPlatform::Xbox, InputKeyId::East) => {
            Some(platform_button_hint(xbox_button_b_rect()))
        }
        (ControllerButtonPlatform::Xbox, InputKeyId::North) => {
            Some(platform_button_hint(xbox_button_y_rect()))
        }
        (ControllerButtonPlatform::Xbox, InputKeyId::West) => {
            Some(platform_button_hint(xbox_button_x_rect()))
        }
        (ControllerButtonPlatform::PlayStation, InputKeyId::South) => {
            Some(platform_button_hint(playstation_button_cross_rect()))
        }
        (ControllerButtonPlatform::PlayStation, InputKeyId::East) => {
            Some(platform_button_hint(playstation_button_circle_rect()))
        }
        (ControllerButtonPlatform::PlayStation, InputKeyId::North) => {
            Some(platform_button_hint(playstation_button_triangle_rect()))
        }
        (ControllerButtonPlatform::PlayStation, InputKeyId::West) => {
            Some(platform_button_hint(playstation_button_square_rect()))
        }
        _ => None,
    }
}

fn platform_button_hint(foreground_rect: Rect) -> ActionKeyHint {
    ActionKeyHint {
        label: String::new(),
        background_rect: generic_button_rect(),
        background_tint: ActionHintTint::Primary,
        foreground_rect: Some(foreground_rect),
        foreground_tint: ActionHintTint::White,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControllerButtonPlatform {
    Xbox,
    PlayStation,
}

fn controller_button_platform(model_id: &str) -> ControllerButtonPlatform {
    let lower = model_id.to_ascii_lowercase();
    if lower.contains("playstation")
        || lower.contains("dualshock")
        || lower.contains("dualsense")
        || lower.contains("wireless controller")
        || lower.contains("sony")
    {
        ControllerButtonPlatform::PlayStation
    } else {
        ControllerButtonPlatform::Xbox
    }
}

fn generic_button_rect() -> Rect {
    icon_grid_rect(
        GENERIC_BUTTON_ICON_X,
        GENERIC_BUTTON_ICON_Y,
        GENERIC_BUTTON_ICON_SIZE,
        GENERIC_BUTTON_ICON_SIZE,
    )
}

fn controller_icon_rect(key_id: InputKeyId) -> Rect {
    match key_id {
        InputKeyId::South => xbox_button_a_rect(),
        InputKeyId::East => xbox_button_b_rect(),
        InputKeyId::North => xbox_button_y_rect(),
        InputKeyId::West => xbox_button_x_rect(),
        InputKeyId::DPadLeft
        | InputKeyId::DPadRight
        | InputKeyId::DPadUp
        | InputKeyId::DPadDown => icon_grid_rect(0.0, 0.0, 8.0, 4.0),
        InputKeyId::Select => icon_grid_rect(SELECT_ICON_X, SELECT_ICON_Y, 4.0, 2.0),
        InputKeyId::Start => icon_grid_rect(START_ICON_X, START_ICON_Y, 4.0, 2.0),
        InputKeyId::LeftTrigger | InputKeyId::RightTrigger => icon_grid_rect(
            SHOULDER_ICON_X,
            SHOULDER_ICON_Y,
            SMALL_ICON_SIZE,
            SMALL_ICON_SIZE,
        ),
        InputKeyId::LeftTrigger2 | InputKeyId::RightTrigger2 => icon_grid_rect(
            TRIGGER_ICON_X,
            TRIGGER_ICON_Y,
            SMALL_ICON_SIZE,
            SMALL_ICON_SIZE,
        ),
        _ => generic_button_rect(),
    }
}

fn xbox_button_x_rect() -> Rect {
    icon_grid_rect(
        XBOX_BUTTON_X_X,
        XBOX_BUTTON_X_Y,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
    )
}

fn xbox_button_y_rect() -> Rect {
    icon_grid_rect(
        XBOX_BUTTON_Y_X,
        XBOX_BUTTON_Y_Y,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
    )
}

fn xbox_button_a_rect() -> Rect {
    icon_grid_rect(
        XBOX_BUTTON_A_X,
        XBOX_BUTTON_A_Y,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
    )
}

fn xbox_button_b_rect() -> Rect {
    icon_grid_rect(
        XBOX_BUTTON_B_X,
        XBOX_BUTTON_B_Y,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
    )
}

fn playstation_button_square_rect() -> Rect {
    icon_grid_rect(
        PLAYSTATION_BUTTON_SQUARE_X,
        PLAYSTATION_BUTTON_SQUARE_Y,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
    )
}

fn playstation_button_triangle_rect() -> Rect {
    icon_grid_rect(
        PLAYSTATION_BUTTON_TRIANGLE_X,
        PLAYSTATION_BUTTON_TRIANGLE_Y,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
    )
}

fn playstation_button_cross_rect() -> Rect {
    icon_grid_rect(
        PLAYSTATION_BUTTON_CROSS_X,
        PLAYSTATION_BUTTON_CROSS_Y,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
    )
}

fn playstation_button_circle_rect() -> Rect {
    icon_grid_rect(
        PLAYSTATION_BUTTON_CIRCLE_X,
        PLAYSTATION_BUTTON_CIRCLE_Y,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
        PLATFORM_FACE_BUTTON_ICON_SIZE,
    )
}

fn icon_grid_rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    let unit = ICON_TEXTURE_SIZE / ICON_GRID_UNITS;
    Rect {
        min: Vec2::new(x * unit, y * unit),
        max: Vec2::new((x + width) * unit, (y + height) * unit),
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;

    #[test]
    fn action_hint_system_queries_are_valid() {
        let mut app = App::new();
        app.add_plugins(ActionHintPlugin);
        app.update();
    }

    #[test]
    fn controller_home_hint_uses_generic_button_with_label() {
        let hint = action_key_hint(
            Some(&InputDeviceMapping {
                r#type: InputDeviceType::Controller,
                controller_model_id: Some("controller".to_string()),
                map: vec![crate::storage::input_mappings::InputMapEntry {
                    key_id: InputKeyId::Mode,
                    map_to: InputAction::PauseAndResume,
                }],
            }),
            InputAction::PauseAndResume,
        );

        assert_eq!(hint.label, "Home");
        assert_eq!(hint.background_rect, generic_button_rect());
        assert!(hint.foreground_rect.is_none());
    }

    #[test]
    fn xbox_face_button_hints_use_tinted_circle_background_and_platform_icon() {
        let hint = controller_action_key_hint(None, InputKeyId::West);

        assert!(hint.label.is_empty());
        assert_eq!(hint.background_rect, generic_button_rect());
        assert_eq!(hint.background_tint, ActionHintTint::Primary);
        assert_eq!(hint.foreground_rect, Some(xbox_button_x_rect()));
    }

    #[test]
    fn playstation_face_button_hints_use_tinted_circle_background_and_platform_icon() {
        let mapping = InputDeviceMapping {
            r#type: InputDeviceType::Controller,
            controller_model_id: Some("Sony DualSense Wireless Controller".to_string()),
            map: Vec::new(),
        };

        let hint = controller_action_key_hint(Some(&mapping), InputKeyId::South);

        assert!(hint.label.is_empty());
        assert_eq!(hint.background_rect, generic_button_rect());
        assert_eq!(hint.background_tint, ActionHintTint::Primary);
        assert_eq!(hint.foreground_rect, Some(playstation_button_cross_rect()));
    }

    #[test]
    fn keyboard_letter_hint_uses_tinted_circle_background() {
        let hint = action_key_hint(
            Some(&InputDeviceMapping {
                r#type: InputDeviceType::Keyboard,
                controller_model_id: None,
                map: vec![crate::storage::input_mappings::InputMapEntry {
                    key_id: InputKeyId::KeyX,
                    map_to: InputAction::A,
                }],
            }),
            InputAction::A,
        );

        assert_eq!(hint.label, "X");
        assert_eq!(hint.background_rect, generic_button_rect());
        assert_eq!(hint.background_tint, ActionHintTint::Primary);
        assert!(hint.foreground_rect.is_none());
    }
}
