#![allow(dead_code)]

use bevy::prelude::*;

pub const SHINING_GRIMACE_LOGO_PATH: &str = "images/shining-grimace-logo.png";
pub const SHINING_EMULATOR_LOGO_PATH: &str = "images/shining-emulator-logo.png";
pub const UBUNTU_MONO_FONT_PATH: &str = "fonts/UbuntuMono-Regular.ttf";
pub const ICONS_PATH: &str = "images/icons.png";

#[derive(Resource, Default)]
pub struct AppAssets {
    pub shining_grimace_logo: Handle<Image>,
    pub shining_emulator_logo: Handle<Image>,
    pub ubuntu_mono_font: Handle<Font>,
    pub icons: Handle<Image>,
    pub theme_background: Option<Handle<Image>>,
}
