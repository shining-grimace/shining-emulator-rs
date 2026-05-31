#![allow(dead_code)]

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, States)]
pub enum AppState {
    #[default]
    Loading,
    Splash,
    Home,
    Settings,
    InputMapping,
    RomProvider,
    RomData,
    AudioSettings,
    Gameplay,
}
