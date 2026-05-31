use bevy::input::ButtonState;
use bevy::prelude::*;

use crate::input::events::MappedInputEvent;
use crate::storage::input_mappings::InputAction;

#[derive(Clone, Copy, Debug, Default, Resource)]
pub(super) struct UiInputState {
    pub up: bool,
    pub right: bool,
    pub down: bool,
    pub left: bool,
    pub select: bool,
    pub back: bool,
    pub quit_app: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum UiInputDirection {
    Up,
    Right,
    Down,
    Left,
}

impl UiInputState {
    pub(super) fn direction(self) -> Option<UiInputDirection> {
        if self.up {
            Some(UiInputDirection::Up)
        } else if self.right {
            Some(UiInputDirection::Right)
        } else if self.down {
            Some(UiInputDirection::Down)
        } else if self.left {
            Some(UiInputDirection::Left)
        } else {
            None
        }
    }

    pub(super) fn focus_recovery_requested(self) -> bool {
        self.up || self.right || self.down || self.left || self.select
    }
}

pub(super) fn collect_ui_input_state(
    keys: Res<ButtonInput<KeyCode>>,
    mut mapped_events: MessageReader<MappedInputEvent>,
    mut input: ResMut<UiInputState>,
) {
    *input = UiInputState {
        up: keys.just_pressed(KeyCode::ArrowUp),
        right: keys.just_pressed(KeyCode::ArrowRight),
        down: keys.just_pressed(KeyCode::ArrowDown),
        left: keys.just_pressed(KeyCode::ArrowLeft),
        ..default()
    };

    for event in mapped_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        match event.action {
            InputAction::Dup => input.up = true,
            InputAction::Dright => input.right = true,
            InputAction::Ddown => input.down = true,
            InputAction::Dleft => input.left = true,
            InputAction::A => input.select = true,
            InputAction::B => input.back = true,
            InputAction::QuitApp => input.quit_app = true,
            _ => {}
        }
    }
}
