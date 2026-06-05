use bevy::asset::HandleTemplate;
use bevy::math::Rect;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::dimensions::{ACTION_HINT_GAP, ACTION_HINT_ICON_SIZE, UI_BODY_FONT_SIZE};
use crate::input::selection::{PrimaryInputDevice, selected_mapping};
use crate::storage::LocalStorage;
use crate::storage::input_mappings::{
    InputAction, InputDeviceMapping, InputDeviceType, InputKeyId,
};
use crate::ui_elements::interactions::IgnorePicking;
use crate::ui_elements::theme::{UiThemeImageColor, UiThemeTextColor};

const ICON_TEXTURE_SIZE: f32 = 1024.0;
const ICON_GRID_UNITS: f32 = 16.0;
const GENERIC_BUTTON_ICON_X: f32 = 12.0;
const GENERIC_BUTTON_ICON_Y: f32 = 0.0;
const GENERIC_BUTTON_ICON_SIZE: f32 = 4.0;
const TRIGGER_ICON_X: f32 = 8.0;
const TRIGGER_ICON_Y: f32 = 4.0;
const SHOULDER_ICON_X: f32 = 12.0;
const SHOULDER_ICON_Y: f32 = 4.0;
const SELECT_ICON_X: f32 = 8.0;
const SELECT_ICON_Y: f32 = 0.0;
const START_ICON_X: f32 = 8.0;
const START_ICON_Y: f32 = 2.0;
const META_ICON_X: f32 = 0.0;
const META_ICON_Y: f32 = 4.0;
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
    let mapping = selected_mapping(primary_input, storage);
    let quit_key = action_key_hint(mapping, InputAction::B);
    let select_key = action_key_hint(mapping, InputAction::A);

    bsn! {
        Node {
            width: percent(100),
            justify_content: JustifyContent::FlexEnd,
            column_gap: px(ACTION_HINT_GAP),
        }
        Children [
            action_hint(font.clone(), icons.clone(), theme, InputAction::B, quit_key, back_label),
            action_hint(font, icons, theme, InputAction::A, select_key, action_label)
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
    let key_label = key.label;
    let key_rect = key.rect;
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
                ImageNode {
                    image: HandleTemplate::Handle(icons),
                    color: {theme.primary},
                    rect: {Some(key_rect)},
                }
                ActionHintIcon { action: {action} }
                UiThemeImageColor::Primary
                IgnorePicking
                Children [
                    (
                        Text({key_label})
                        Node {
                            display: {text_display},
                        }
                        TextFont {
                            font: FontSourceTemplate::Handle(HandleTemplate::Handle(key_font)),
                            font_size: px(UI_BODY_FONT_SIZE),
                        }
                        TextColor(Color::BLACK)
                        UiThemeTextColor::Black
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
struct ActionHintKeyLabel;

#[derive(Clone)]
struct ActionKeyHint {
    label: String,
    rect: Rect,
}

fn sync_action_hints(
    storage: Option<Res<LocalStorage>>,
    primary_input: Option<Res<PrimaryInputDevice>>,
    mut icons: Query<(&ActionHintIcon, &mut ImageNode, &Children)>,
    mut labels: Query<(&mut Text, &mut Node), With<ActionHintKeyLabel>>,
) {
    let (Some(storage), Some(primary_input)) = (storage, primary_input) else {
        return;
    };
    let mapping = selected_mapping(&primary_input, &storage);
    for (icon, mut image, children) in &mut icons {
        let key = action_key_hint(mapping, icon.action);
        image.rect = Some(key.rect);
        for child in children {
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
        (InputDeviceType::Controller, Some(key_id)) => ActionKeyHint {
            label: String::new(),
            rect: controller_icon_rect(key_id),
        },
        (_, Some(key_id)) => ActionKeyHint {
            label: key_id_label(key_id),
            rect: generic_button_rect(),
        },
        _ => ActionKeyHint {
            label: String::new(),
            rect: generic_button_rect(),
        },
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
        InputKeyId::Mode | InputKeyId::LeftThumb | InputKeyId::RightThumb => {
            icon_grid_rect(META_ICON_X, META_ICON_Y, SMALL_ICON_SIZE, SMALL_ICON_SIZE)
        }
        _ => generic_button_rect(),
    }
}

fn icon_grid_rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    let unit = ICON_TEXTURE_SIZE / ICON_GRID_UNITS;
    Rect {
        min: Vec2::new(x * unit, y * unit),
        max: Vec2::new((x + width) * unit, (y + height) * unit),
    }
}
