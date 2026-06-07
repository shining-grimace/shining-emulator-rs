#![allow(dead_code)]

use bevy::prelude::*;
use bevy_midi_graph::{MidiFileSource, MidiGraph};

pub const SHINING_GRIMACE_LOGO_PATH: &str = "images/shining-grimace-logo.png";
pub const SHINING_EMULATOR_LOGO_PATH: &str = "images/shining-emulator-logo.png";
pub const UBUNTU_MONO_FONT_PATH: &str = "fonts/UbuntuMono-Regular.ttf";
pub const ICONS_PATH: &str = "images/icons.png";
pub const HEROES_PATH: &str = "images/heroes.png";
pub const MENU_MIDI_PATH: &str = "audio/audio.mid";
pub const DEFAULT_AUDIO_GRAPH_PATH: &str = "audio/default-graph.json";

#[derive(Resource, Default)]
pub struct AppAssets {
    pub shining_grimace_logo: Handle<Image>,
    pub shining_emulator_logo: Handle<Image>,
    pub ubuntu_mono_font: Handle<Font>,
    pub icons: Handle<Image>,
    pub heroes: Handle<Image>,
    pub theme_background: Option<Handle<Image>>,
    pub menu_midi: Handle<MidiFileSource>,
    pub default_audio_graph: Handle<MidiGraph>,
}
