use bevy::prelude::*;

use super::focus::FocusedUiElement;
use super::visual_state::{DisabledUiElement, UiElementColors};

const BACKSPACE_REPEAT_DELAY_SECONDS: f32 = 0.32;
const BACKSPACE_REPEAT_SECONDS: f32 = 0.045;
const TEXT_INPUT_VISIBLE_CHARS: usize = 31;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct EditableUiElement;

#[derive(Clone, Component, Debug, FromTemplate)]
pub struct UiTextInput {
    pub value: String,
    pub placeholder: String,
    pub cursor: usize,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiTextInputText;

#[derive(Default)]
pub(super) struct RepeatState {
    held_for_seconds: f32,
    repeat_for_seconds: f32,
}

pub(super) fn edit_text_inputs(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut inputs: Query<
        &mut UiTextInput,
        (
            With<FocusedUiElement>,
            With<EditableUiElement>,
            Without<DisabledUiElement>,
        ),
    >,
    mut repeat: Local<RepeatState>,
) {
    let Ok(mut input) = inputs.single_mut() else {
        *repeat = RepeatState::default();
        return;
    };

    if keys.just_pressed(KeyCode::Backspace) && input.cursor > 0 {
        delete_before_cursor(&mut input);
        repeat.held_for_seconds = 0.0;
        repeat.repeat_for_seconds = 0.0;
    } else if keys.pressed(KeyCode::Backspace) {
        repeat.held_for_seconds += time.delta_secs();
        if repeat.held_for_seconds >= BACKSPACE_REPEAT_DELAY_SECONDS {
            repeat.repeat_for_seconds += time.delta_secs();
            while repeat.repeat_for_seconds >= BACKSPACE_REPEAT_SECONDS {
                repeat.repeat_for_seconds -= BACKSPACE_REPEAT_SECONDS;
                delete_before_cursor(&mut input);
            }
        }
    } else {
        *repeat = RepeatState::default();
    }

    if keys.just_pressed(KeyCode::ArrowLeft) && input.cursor > 0 {
        input.cursor -= 1;
    }
    if keys.just_pressed(KeyCode::ArrowRight) && input.cursor < input.value.len() {
        input.cursor += 1;
    }

    if keys.just_pressed(KeyCode::Space) {
        insert_char(&mut input, ' ');
    }

    for (key, character) in key_chars() {
        if keys.just_pressed(key) {
            insert_char(&mut input, character);
        }
    }
}

pub(super) fn update_text_input_text(
    time: Res<Time>,
    inputs: Query<(
        &UiTextInput,
        &UiElementColors,
        Has<FocusedUiElement>,
        &Children,
    )>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<UiTextInputText>>,
) {
    let cursor_visible = (time.elapsed_secs() * 2.0).floor() as i32 % 2 == 0;

    for (input, colours, focused, children) in &inputs {
        let text = rendered_text(input, focused, cursor_visible);
        let colour = if input.value.is_empty() {
            colours.tertiary
        } else if focused {
            colours.secondary
        } else {
            colours.primary
        };

        for child in children {
            if let Ok((mut text_component, mut text_colour)) = text_query.get_mut(*child) {
                text_component.0 = text.clone();
                text_colour.0 = colour;
            }
        }
    }
}

fn delete_before_cursor(input: &mut UiTextInput) {
    if input.cursor == 0 {
        return;
    }
    let remove_at = input.cursor - 1;
    input.value.remove(remove_at);
    input.cursor = remove_at;
}

fn insert_char(input: &mut UiTextInput, character: char) {
    input.value.insert(input.cursor, character);
    input.cursor += 1;
}

fn rendered_text(input: &UiTextInput, focused: bool, cursor_visible: bool) -> String {
    if input.value.is_empty() {
        if focused {
            return format!(
                "{}{}",
                if cursor_visible { '|' } else { '\u{00a0}' },
                input.placeholder
            );
        } else {
            return input.placeholder.clone();
        };
    }

    let max_chars = if focused {
        TEXT_INPUT_VISIBLE_CHARS.saturating_sub(1)
    } else {
        TEXT_INPUT_VISIBLE_CHARS
    };
    let (visible, cursor) = visible_text_around_cursor(&input.value, input.cursor, max_chars);

    if focused {
        return text_with_cursor(visible, cursor, cursor_visible);
    }

    visible
}

fn visible_text_around_cursor(value: &str, cursor: usize, max_chars: usize) -> (String, usize) {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return (value.to_string(), cursor);
    }

    let start = cursor.saturating_sub(max_chars);
    (
        value.chars().skip(start).take(max_chars).collect(),
        cursor - start,
    )
}

fn text_with_cursor(mut text: String, cursor: usize, cursor_visible: bool) -> String {
    text.insert(
        cursor.min(text.len()),
        if cursor_visible { '|' } else { '\u{00a0}' },
    );
    text
}

fn key_chars() -> impl Iterator<Item = (KeyCode, char)> {
    [
        (KeyCode::KeyA, 'a'),
        (KeyCode::KeyB, 'b'),
        (KeyCode::KeyC, 'c'),
        (KeyCode::KeyD, 'd'),
        (KeyCode::KeyE, 'e'),
        (KeyCode::KeyF, 'f'),
        (KeyCode::KeyG, 'g'),
        (KeyCode::KeyH, 'h'),
        (KeyCode::KeyI, 'i'),
        (KeyCode::KeyJ, 'j'),
        (KeyCode::KeyK, 'k'),
        (KeyCode::KeyL, 'l'),
        (KeyCode::KeyM, 'm'),
        (KeyCode::KeyN, 'n'),
        (KeyCode::KeyO, 'o'),
        (KeyCode::KeyP, 'p'),
        (KeyCode::KeyQ, 'q'),
        (KeyCode::KeyR, 'r'),
        (KeyCode::KeyS, 's'),
        (KeyCode::KeyT, 't'),
        (KeyCode::KeyU, 'u'),
        (KeyCode::KeyV, 'v'),
        (KeyCode::KeyW, 'w'),
        (KeyCode::KeyX, 'x'),
        (KeyCode::KeyY, 'y'),
        (KeyCode::KeyZ, 'z'),
        (KeyCode::Digit0, '0'),
        (KeyCode::Digit1, '1'),
        (KeyCode::Digit2, '2'),
        (KeyCode::Digit3, '3'),
        (KeyCode::Digit4, '4'),
        (KeyCode::Digit5, '5'),
        (KeyCode::Digit6, '6'),
        (KeyCode::Digit7, '7'),
        (KeyCode::Digit8, '8'),
        (KeyCode::Digit9, '9'),
    ]
    .into_iter()
}
