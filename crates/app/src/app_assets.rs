#![allow(dead_code)]

use bevy::prelude::*;
use bevy_midi_graph::{MidiFileSource, WaveFileSource};

pub const SHINING_GRIMACE_LOGO_PATH: &str = "images/shining-grimace-logo.png";
pub const SHINING_EMULATOR_LOGO_PATH: &str = "images/shining-emulator-logo.png";
pub const UBUNTU_MONO_FONT_PATH: &str = "fonts/UbuntuMono-Regular.ttf";
pub const ICONS_PATH: &str = "images/icons.png";
pub const HEROES_PATH: &str = "images/heroes.png";
pub const MENU_MIDI_PATH: &str = "audio/audio.mid";
pub const BUILT_IN_AUDIO_SAMPLE_PATHS: [(&str, &str); 4] = [
    ("Piano", "audio/Piano.wav"),
    ("Guitar", "audio/Guitar.wav"),
    ("Bass", "audio/Bass.wav"),
    ("Bell", "audio/Bell.wav"),
];

#[derive(Resource, Default)]
pub struct AppAssets {
    pub shining_grimace_logo: Handle<Image>,
    pub shining_emulator_logo: Handle<Image>,
    pub ubuntu_mono_font: Handle<Font>,
    pub icons: Handle<Image>,
    pub heroes: Handle<Image>,
    pub theme_background: Option<Handle<Image>>,
    pub menu_midi: Handle<MidiFileSource>,
    pub built_in_audio_samples: Vec<Handle<WaveFileSource>>,
}
