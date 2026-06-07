#![allow(dead_code)]

use bevy::prelude::*;

use crate::storage::LocalStorage;

pub const IMAGE_THEME_COUNT: usize = 16;
pub const RANDOM_THEME_SETTING: u8 = 0;
pub const MINIMAL_THEME_SETTING: u8 = 1;
pub const FIRST_IMAGE_THEME_SETTING: u8 = 2;

#[derive(Clone, Copy)]
pub struct ThemeDefinition {
    pub name: &'static str,
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub background_asset_path: Option<&'static str>,
    pub audio_anchor: Option<u32>,
}

#[derive(Resource, Clone, Copy)]
pub struct ActiveTheme {
    pub name: &'static str,
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub background_asset_path: Option<&'static str>,
    pub audio_anchor: Option<u32>,
}

#[derive(Event)]
pub struct ActiveThemeChanged;

impl From<ThemeDefinition> for ActiveTheme {
    fn from(theme: ThemeDefinition) -> Self {
        Self {
            name: theme.name,
            primary: theme.primary,
            secondary: theme.secondary,
            tertiary: theme.tertiary,
            background_asset_path: theme.background_asset_path,
            audio_anchor: theme.audio_anchor,
        }
    }
}

impl FromWorld for ActiveTheme {
    fn from_world(world: &mut World) -> Self {
        let theme_setting = world
            .get_resource::<LocalStorage>()
            .map(|storage| storage.data.settings.ui_theme)
            .unwrap_or(RANDOM_THEME_SETTING);

        active_theme_for_setting(theme_setting)
    }
}

pub fn active_theme_for_setting(setting: u8) -> ActiveTheme {
    theme_definition_for_setting(setting).into()
}

fn theme_definition_for_setting(setting: u8) -> ThemeDefinition {
    match setting {
        RANDOM_THEME_SETTING => IMAGE_THEMES[fastrand::usize(0..IMAGE_THEME_COUNT)],
        MINIMAL_THEME_SETTING => MINIMAL_THEME,
        setting => {
            let index = setting
                .saturating_sub(FIRST_IMAGE_THEME_SETTING)
                .min((IMAGE_THEME_COUNT - 1) as u8) as usize;
            IMAGE_THEMES[index]
        }
    }
}

pub const MINIMAL_THEME: ThemeDefinition = ThemeDefinition {
    name: "Minimal",
    primary: Color::srgb_u8(0xbc, 0x31, 0xff),
    secondary: Color::srgb_u8(0xe4, 0xbd, 0xa3),
    tertiary: Color::srgb_u8(0x8c, 0xb9, 0xca),
    background_asset_path: None,
    audio_anchor: None,
};

pub const IMAGE_THEMES: [ThemeDefinition; IMAGE_THEME_COUNT] = [
    ThemeDefinition {
        name: "Forest",
        primary: Color::srgb_u8(0x3d, 0xb5, 0x3c),
        secondary: Color::srgb_u8(0x3c, 0x3c, 0xb5),
        tertiary: Color::srgb_u8(0xb5, 0xb5, 0x3c),
        background_asset_path: Some("images/theme-1.png"),
        audio_anchor: Some(1),
    },
    ThemeDefinition {
        name: "Jungle",
        primary: Color::srgb_u8(0xb5, 0x5d, 0x27),
        secondary: Color::srgb_u8(0x27, 0x38, 0xb5),
        tertiary: Color::srgb_u8(0x7f, 0xb5, 0x27),
        background_asset_path: Some("images/theme-2.png"),
        audio_anchor: Some(2),
    },
    ThemeDefinition {
        name: "Temple",
        primary: Color::srgb_u8(0xa5, 0xb5, 0x85),
        secondary: Color::srgb_u8(0xad, 0x85, 0xb5),
        tertiary: Color::srgb_u8(0x85, 0xb5, 0x95),
        background_asset_path: Some("images/theme-3.png"),
        audio_anchor: Some(3),
    },
    ThemeDefinition {
        name: "Cyber",
        primary: Color::srgb_u8(0x45, 0x8b, 0xb5),
        secondary: Color::srgb_u8(0xb5, 0x58, 0x45),
        tertiary: Color::srgb_u8(0x45, 0xb5, 0x93),
        background_asset_path: Some("images/theme-4.png"),
        audio_anchor: Some(4),
    },
    ThemeDefinition {
        name: "Engine room",
        primary: Color::srgb_u8(0xb2, 0xb5, 0xa5),
        secondary: Color::srgb_u8(0xa5, 0xaa, 0xb5),
        tertiary: Color::srgb_u8(0xb5, 0xa8, 0xa5),
        background_asset_path: Some("images/theme-5.png"),
        audio_anchor: Some(5),
    },
    ThemeDefinition {
        name: "Deep sea",
        primary: Color::srgb_u8(0x1e, 0x4e, 0xb5),
        secondary: Color::srgb_u8(0x99, 0xb5, 0x1e),
        tertiary: Color::srgb_u8(0x85, 0x1e, 0xb5),
        background_asset_path: Some("images/theme-6.png"),
        audio_anchor: Some(6),
    },
    ThemeDefinition {
        name: "Starry night",
        primary: Color::srgb_u8(0x8f, 0xb5, 0xb4),
        secondary: Color::srgb_u8(0xb5, 0x8f, 0xa3),
        tertiary: Color::srgb_u8(0x90, 0xb5, 0x8f),
        background_asset_path: Some("images/theme-7.png"),
        audio_anchor: Some(7),
    },
    ThemeDefinition {
        name: "Alien space",
        primary: Color::srgb_u8(0xab, 0x72, 0xb5),
        secondary: Color::srgb_u8(0x72, 0xb5, 0x89),
        tertiary: Color::srgb_u8(0xb5, 0x72, 0x7c),
        background_asset_path: Some("images/theme-8.png"),
        audio_anchor: Some(8),
    },
    ThemeDefinition {
        name: "Black hole",
        primary: Color::srgb_u8(0x78, 0x60, 0xb5),
        secondary: Color::srgb_u8(0xb5, 0xa1, 0x60),
        tertiary: Color::srgb_u8(0x60, 0x9e, 0xb5),
        background_asset_path: Some("images/theme-9.png"),
        audio_anchor: Some(9),
    },
    ThemeDefinition {
        name: "Loneliness",
        primary: Color::srgb_u8(0x5a, 0x85, 0xb5),
        secondary: Color::srgb_u8(0xb3, 0xb5, 0x5a),
        tertiary: Color::srgb_u8(0x89, 0x5a, 0xb5),
        background_asset_path: Some("images/theme-10.png"),
        audio_anchor: Some(10),
    },
    ThemeDefinition {
        name: "Cathedral",
        primary: Color::srgb_u8(0x81, 0x6d, 0xb5),
        secondary: Color::srgb_u8(0xb5, 0xa5, 0x6d),
        tertiary: Color::srgb_u8(0x6d, 0xa1, 0xb5),
        background_asset_path: Some("images/theme-11.png"),
        audio_anchor: Some(11),
    },
    ThemeDefinition {
        name: "Runway",
        primary: Color::srgb_u8(0xb5, 0xb5, 0xb5),
        secondary: Color::srgb_u8(0xb3, 0xb5, 0x41),
        tertiary: Color::srgb_u8(0x41, 0x79, 0xb5),
        background_asset_path: Some("images/theme-12.png"),
        audio_anchor: Some(12),
    },
    ThemeDefinition {
        name: "Swamp",
        primary: Color::srgb_u8(0x29, 0xb5, 0x81),
        secondary: Color::srgb_u8(0xb5, 0x3c, 0x29),
        tertiary: Color::srgb_u8(0x29, 0x5c, 0xb5),
        background_asset_path: Some("images/theme-13.png"),
        audio_anchor: Some(13),
    },
    ThemeDefinition {
        name: "Fire cavern",
        primary: Color::srgb_u8(0xb5, 0x84, 0x40),
        secondary: Color::srgb_u8(0x4a, 0x40, 0xb5),
        tertiary: Color::srgb_u8(0x71, 0xb5, 0x40),
        background_asset_path: Some("images/theme-14.png"),
        audio_anchor: Some(14),
    },
    ThemeDefinition {
        name: "Twilight city",
        primary: Color::srgb_u8(0xb5, 0x4e, 0x83),
        secondary: Color::srgb_u8(0x4e, 0xb5, 0xb3),
        tertiary: Color::srgb_u8(0xb5, 0x80, 0x4e),
        background_asset_path: Some("images/theme-15.png"),
        audio_anchor: Some(15),
    },
    ThemeDefinition {
        name: "In the clouds",
        primary: Color::srgb_u8(0x86, 0xa2, 0xb5),
        secondary: Color::srgb_u8(0xb5, 0xb0, 0x86),
        tertiary: Color::srgb_u8(0x99, 0x86, 0xb5),
        background_asset_path: Some("images/theme-16.png"),
        audio_anchor: Some(16),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_theme_has_no_audio_anchor() {
        let theme = active_theme_for_setting(MINIMAL_THEME_SETTING);

        assert_eq!(theme.audio_anchor, None);
    }

    #[test]
    fn image_theme_audio_anchors_match_theme_numbers() {
        for setting in
            FIRST_IMAGE_THEME_SETTING..FIRST_IMAGE_THEME_SETTING + IMAGE_THEME_COUNT as u8
        {
            let theme = active_theme_for_setting(setting);

            assert_eq!(
                theme.audio_anchor,
                Some((setting - FIRST_IMAGE_THEME_SETTING + 1) as u32)
            );
        }
    }
}
