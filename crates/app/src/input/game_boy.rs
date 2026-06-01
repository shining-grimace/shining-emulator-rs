use bevy::prelude::*;

use crate::storage::input_mappings::InputAction;

#[derive(Resource, Clone, Debug, Default)]
pub struct GameBoyInputState {
    pub dleft: bool,
    pub dright: bool,
    pub dup: bool,
    pub ddown: bool,
    pub a: bool,
    pub b: bool,
    pub start: bool,
    pub select: bool,
}

impl GameBoyInputState {
    pub(super) fn set_button(&mut self, button: GameBoyButton, pressed: bool) {
        match button {
            GameBoyButton::Dleft => self.dleft = pressed,
            GameBoyButton::Dright => self.dright = pressed,
            GameBoyButton::Dup => self.dup = pressed,
            GameBoyButton::Ddown => self.ddown = pressed,
            GameBoyButton::A => self.a = pressed,
            GameBoyButton::B => self.b = pressed,
            GameBoyButton::Start => self.start = pressed,
            GameBoyButton::Select => self.select = pressed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GameBoyButton {
    Dleft,
    Dright,
    Dup,
    Ddown,
    A,
    B,
    Start,
    Select,
}

impl TryFrom<InputAction> for GameBoyButton {
    type Error = ();

    fn try_from(action: InputAction) -> Result<Self, Self::Error> {
        match action {
            InputAction::Dleft => Ok(Self::Dleft),
            InputAction::Dright => Ok(Self::Dright),
            InputAction::Dup => Ok(Self::Dup),
            InputAction::Ddown => Ok(Self::Ddown),
            InputAction::A => Ok(Self::A),
            InputAction::B => Ok(Self::B),
            InputAction::Start => Ok(Self::Start),
            InputAction::Select => Ok(Self::Select),
            _ => Err(()),
        }
    }
}
