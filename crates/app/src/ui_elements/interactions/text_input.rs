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

#[derive(Default)]
pub(super) struct PendingPaste {
    read: Option<(Entity, ClipboardRead)>,
}

pub(super) fn edit_text_inputs(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut clipboard: ResMut<Clipboard>,
    mut inputs: Query<
        (Entity, &mut UiTextInput),
        (
            With<FocusedUiElement>,
            With<EditableUiElement>,
            Without<DisabledUiElement>,
        ),
    >,
    mut repeat: Local<RepeatState>,
    mut pending_paste: Local<PendingPaste>,
) {
    let Ok((focused_entity, mut input)) = inputs.single_mut() else {
        *repeat = RepeatState::default();
        pending_paste.read = None;
        return;
    };

    poll_pending_paste(&mut input, &mut pending_paste.read, focused_entity);

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
        move_cursor_left(&mut input);
    }
    if keys.just_pressed(KeyCode::ArrowRight) && input.cursor < input.value.len() {
        move_cursor_right(&mut input);
    }

    if paste_requested(&keys) {
        start_paste(
            &mut input,
            &mut clipboard,
            &mut pending_paste.read,
            focused_entity,
        );
    }

    if command_modifier_pressed(&keys) {
        return;
    }

    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    for (key, character) in key_chars(shift) {
        if keys.just_pressed(key) {
            insert_char(&mut input, character);
        }
    }
}

fn poll_pending_paste(
    input: &mut UiTextInput,
    pending_paste: &mut Option<(Entity, ClipboardRead)>,
    focused_entity: Entity,
) {
    let mut clear_pending = false;
    if let Some((paste_entity, read)) = pending_paste.as_mut() {
        if *paste_entity == focused_entity {
            if let Some(result) = read.poll_result() {
                match result {
                    Ok(text) => insert_text(input, &text),
                    Err(error) => eprintln!("failed to paste from clipboard: {error}"),
                }
                clear_pending = true;
            }
        } else {
            clear_pending = true;
        }
    }

    if clear_pending {
        *pending_paste = None;
    }
}

fn start_paste(
    input: &mut UiTextInput,
    clipboard: &mut Clipboard,
    pending_paste: &mut Option<(Entity, ClipboardRead)>,
    focused_entity: Entity,
) {
    *pending_paste = None;
    let mut read = clipboard.fetch_text();
    match read.poll_result() {
        Some(Ok(text)) => insert_text(input, &text),
        Some(Err(error)) => eprintln!("failed to paste from clipboard: {error}"),
        None => *pending_paste = Some((focused_entity, read)),
    }
}

fn paste_requested(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::KeyV) && command_modifier_pressed(keys)
}

fn command_modifier_pressed(keys: &ButtonInput<KeyCode>) -> bool {
    keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ])
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
    let remove_at = previous_cursor_position(&input.value, input.cursor);
    input.value.remove(remove_at);
    input.cursor = remove_at;
}

fn insert_char(input: &mut UiTextInput, character: char) {
    input.value.insert(input.cursor, character);
    input.cursor += character.len_utf8();
}

fn insert_text(input: &mut UiTextInput, text: &str) {
    for character in text.chars().filter_map(single_line_text_character) {
        insert_char(input, character);
    }
}

fn single_line_text_character(character: char) -> Option<char> {
    match character {
        '\r' | '\n' => None,
        '\t' => Some(' '),
        character if character.is_control() => None,
        character => Some(character),
    }
}

fn move_cursor_left(input: &mut UiTextInput) {
    input.cursor = previous_cursor_position(&input.value, input.cursor);
}

fn move_cursor_right(input: &mut UiTextInput) {
    input.cursor = next_cursor_position(&input.value, input.cursor);
}

fn previous_cursor_position(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_cursor_position(value: &str, cursor: usize) -> usize {
    value[cursor..]
        .chars()
        .next()
        .map(|character| cursor + character.len_utf8())
        .unwrap_or(value.len())
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
    let cursor_char = char_index_for_byte_index(value, cursor);
    if char_count <= max_chars {
        return (value.to_string(), cursor_char);
    }

    let start = cursor_char.saturating_sub(max_chars);
    let end = (start + max_chars).min(char_count);
    let start_byte = byte_index_for_char_index(value, start);
    let end_byte = byte_index_for_char_index(value, end);
    (value[start_byte..end_byte].to_string(), cursor_char - start)
}

fn text_with_cursor(mut text: String, cursor: usize, cursor_visible: bool) -> String {
    let cursor_byte = byte_index_for_char_index(&text, cursor);
    text.insert(cursor_byte, if cursor_visible { '|' } else { '\u{00a0}' });
    text
}

fn char_index_for_byte_index(value: &str, cursor: usize) -> usize {
    value[..cursor].chars().count()
}

fn byte_index_for_char_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}

fn key_chars(shift: bool) -> impl Iterator<Item = (KeyCode, char)> {
    let letters = [
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
    ];
    let digits = if shift {
        [
            (KeyCode::Digit0, ')'),
            (KeyCode::Digit1, '!'),
            (KeyCode::Digit2, '@'),
            (KeyCode::Digit3, '#'),
            (KeyCode::Digit4, '$'),
            (KeyCode::Digit5, '%'),
            (KeyCode::Digit6, '^'),
            (KeyCode::Digit7, '&'),
            (KeyCode::Digit8, '*'),
            (KeyCode::Digit9, '('),
        ]
    } else {
        [
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
    };
    let punctuation = if shift {
        [
            (KeyCode::Space, ' '),
            (KeyCode::Slash, '?'),
            (KeyCode::Backslash, '|'),
            (KeyCode::Period, '>'),
            (KeyCode::Comma, '<'),
            (KeyCode::Minus, '_'),
            (KeyCode::Equal, '+'),
            (KeyCode::Semicolon, ':'),
            (KeyCode::Quote, '"'),
            (KeyCode::BracketLeft, '{'),
            (KeyCode::BracketRight, '}'),
            (KeyCode::Backquote, '~'),
        ]
    } else {
        [
            (KeyCode::Space, ' '),
            (KeyCode::Slash, '/'),
            (KeyCode::Backslash, '\\'),
            (KeyCode::Period, '.'),
            (KeyCode::Comma, ','),
            (KeyCode::Minus, '-'),
            (KeyCode::Equal, '='),
            (KeyCode::Semicolon, ';'),
            (KeyCode::Quote, '\''),
            (KeyCode::BracketLeft, '['),
            (KeyCode::BracketRight, ']'),
            (KeyCode::Backquote, '`'),
        ]
    };

    letters
        .into_iter()
        .map(move |(key, value)| {
            (
                key,
                if shift {
                    value.to_ascii_uppercase()
                } else {
                    value
                },
            )
        })
        .chain(digits)
        .chain(punctuation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pressed_keys(keys: &[KeyCode]) -> ButtonInput<KeyCode> {
        let mut input = ButtonInput::default();
        for key in keys {
            input.press(*key);
        }
        input
    }

    fn input_value(value: &str, cursor: usize) -> UiTextInput {
        UiTextInput {
            value: value.to_string(),
            placeholder: String::new(),
            cursor,
        }
    }

    #[test]
    fn paste_is_requested_by_control_v_or_super_v() {
        assert!(paste_requested(&pressed_keys(&[
            KeyCode::ControlLeft,
            KeyCode::KeyV,
        ])));
        assert!(paste_requested(&pressed_keys(&[
            KeyCode::SuperRight,
            KeyCode::KeyV,
        ])));
    }

    #[test]
    fn paste_is_not_requested_by_plain_v() {
        assert!(!paste_requested(&pressed_keys(&[KeyCode::KeyV])));
    }

    #[test]
    fn insert_text_pastes_at_cursor_and_keeps_input_single_line() {
        let mut input = input_value("https://example.test", "https://".len());

        insert_text(&mut input, "homebrew\nhub\t{id}\u{0007}");

        assert_eq!(input.value, "https://homebrewhub {id}example.test");
        assert_eq!(input.cursor, "https://homebrewhub {id}".len());
    }

    #[test]
    fn cursor_edits_stay_on_unicode_character_boundaries() {
        let mut input = input_value("abé", "abé".len());

        move_cursor_left(&mut input);
        insert_text(&mut input, "ç");
        delete_before_cursor(&mut input);

        assert_eq!(input.value, "abé");
        assert_eq!(input.cursor, "ab".len());
    }
}
