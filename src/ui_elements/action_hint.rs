use bevy::asset::HandleTemplate;
use bevy::math::Rect;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::input::mappings::RuntimeInputMappings;
use crate::storage::input_mappings::InputAction;
use crate::ui_elements::interactions::IgnorePicking;
use crate::ui_elements::styles::UI_BODY_FONT_SIZE;

const ICON_TEXTURE_SIZE: f32 = 1024.0;
const ICON_GRID_UNITS: f32 = 16.0;
const GENERIC_BUTTON_ICON_X: f32 = 12.0;
const GENERIC_BUTTON_ICON_Y: f32 = 0.0;
const GENERIC_BUTTON_ICON_SIZE: f32 = 4.0;
const ACTION_HINT_ICON_SIZE: f32 = 48.0;

pub fn action_hints(
    font: Handle<Font>,
    icons: Handle<Image>,
    theme: ActiveTheme,
    input_mappings: &RuntimeInputMappings,
) -> impl Scene {
    let quit_key = action_key_label(input_mappings, InputAction::QuitApp).unwrap_or_default();
    let select_key = action_key_label(input_mappings, InputAction::A).unwrap_or_default();

    bsn! {
        Node {
            width: percent(100),
            justify_content: JustifyContent::FlexEnd,
            column_gap: px(42.0),
        }
        Children [
            action_hint(font.clone(), icons.clone(), theme, quit_key, "Quit"),
            action_hint(font, icons, theme, select_key, "Select")
        ]
    }
}

fn action_hint(
    font: Handle<Font>,
    icons: Handle<Image>,
    theme: ActiveTheme,
    key: impl Into<String>,
    label: &'static str,
) -> impl Scene {
    let key_font = font.clone();
    let key = key.into();
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
                    rect: {Some(icon_grid_rect(
                        GENERIC_BUTTON_ICON_X,
                        GENERIC_BUTTON_ICON_Y,
                        GENERIC_BUTTON_ICON_SIZE,
                        GENERIC_BUTTON_ICON_SIZE,
                    ))},
                }
                IgnorePicking
                Children [
                    (
                        Text({key})
                        TextFont {
                            font: FontSourceTemplate::Handle(HandleTemplate::Handle(key_font)),
                            font_size: px(UI_BODY_FONT_SIZE),
                        }
                        TextColor(Color::BLACK)
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
                IgnorePicking
            )
        ]
    }
}

fn action_key_label(input_mappings: &RuntimeInputMappings, action: InputAction) -> Option<String> {
    input_mappings
        .keyboard_key_for_action(action)
        .map(key_code_label)
}

fn key_code_label(key_code: KeyCode) -> String {
    match key_code {
        KeyCode::KeyA => "A",
        KeyCode::KeyB => "B",
        KeyCode::KeyC => "C",
        KeyCode::KeyD => "D",
        KeyCode::KeyE => "E",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyI => "I",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::KeyM => "M",
        KeyCode::KeyN => "N",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyR => "R",
        KeyCode::KeyS => "S",
        KeyCode::KeyT => "T",
        KeyCode::KeyU => "U",
        KeyCode::KeyV => "V",
        KeyCode::KeyW => "W",
        KeyCode::KeyX => "X",
        KeyCode::KeyY => "Y",
        KeyCode::KeyZ => "Z",
        KeyCode::Enter => "Enter",
        KeyCode::Space => "Space",
        KeyCode::Escape => "Esc",
        KeyCode::Tab => "Tab",
        KeyCode::Backspace => "Backspace",
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "Shift",
        KeyCode::ControlLeft | KeyCode::ControlRight => "Ctrl",
        KeyCode::ArrowLeft => "Left",
        KeyCode::ArrowRight => "Right",
        KeyCode::ArrowUp => "Up",
        KeyCode::ArrowDown => "Down",
        KeyCode::Digit0 => "0",
        KeyCode::Digit1 => "1",
        KeyCode::Digit2 => "2",
        KeyCode::Digit3 => "3",
        KeyCode::Digit4 => "4",
        KeyCode::Digit5 => "5",
        KeyCode::Digit6 => "6",
        KeyCode::Digit7 => "7",
        KeyCode::Digit8 => "8",
        KeyCode::Digit9 => "9",
        _ => "?",
    }
    .to_string()
}

fn icon_grid_rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    let unit = ICON_TEXTURE_SIZE / ICON_GRID_UNITS;
    Rect {
        min: Vec2::new(x * unit, y * unit),
        max: Vec2::new((x + width) * unit, (y + height) * unit),
    }
}
