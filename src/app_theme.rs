#![allow(dead_code)]

use bevy::prelude::*;

pub const IMAGE_THEME_COUNT: usize = 16;

#[derive(Clone, Copy)]
pub struct ThemeDefinition {
    pub name: &'static str,
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub background_asset_path: Option<&'static str>,
}

#[derive(Resource, Clone, Copy)]
pub struct ActiveTheme {
    pub name: &'static str,
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub background_asset_path: Option<&'static str>,
}

impl From<ThemeDefinition> for ActiveTheme {
    fn from(theme: ThemeDefinition) -> Self {
        Self {
            name: theme.name,
            primary: theme.primary,
            secondary: theme.secondary,
            tertiary: theme.tertiary,
            background_asset_path: theme.background_asset_path,
        }
    }
}

impl FromWorld for ActiveTheme {
    fn from_world(_world: &mut World) -> Self {
        IMAGE_THEMES[fastrand::usize(0..IMAGE_THEME_COUNT)].into()
    }
}

pub const MINIMAL_THEME: ThemeDefinition = ThemeDefinition {
    name: "Minimal",
    primary: Color::srgb_u8(0xbc, 0x31, 0xff),
    secondary: Color::srgb_u8(0xe4, 0xbd, 0xa3),
    tertiary: Color::srgb_u8(0x8c, 0xb9, 0xca),
    background_asset_path: None,
};

pub const IMAGE_THEMES: [ThemeDefinition; IMAGE_THEME_COUNT] = [
    ThemeDefinition {
        name: "Forest",
        primary: Color::srgb_u8(0x45, 0xcc, 0x44),
        secondary: Color::srgb_u8(0x93, 0x8d, 0x2f),
        tertiary: Color::srgb_u8(0x89, 0x49, 0x00),
        background_asset_path: Some("images/theme-1.png"),
    },
    ThemeDefinition {
        name: "Jungle",
        primary: Color::srgb_u8(0xb2, 0x5b, 0x26),
        secondary: Color::srgb_u8(0x97, 0x9d, 0x6e),
        tertiary: Color::srgb_u8(0x05, 0x97, 0x47),
        background_asset_path: Some("images/theme-2.png"),
    },
    ThemeDefinition {
        name: "Temple",
        primary: Color::srgb_u8(0xa5, 0xb5, 0x85),
        secondary: Color::srgb_u8(0x2a, 0x93, 0x38),
        tertiary: Color::srgb_u8(0x90, 0xaf, 0xac),
        background_asset_path: Some("images/theme-3.png"),
    },
    ThemeDefinition {
        name: "Cyber",
        primary: Color::srgb_u8(0x44, 0x89, 0xb2),
        secondary: Color::srgb_u8(0xb6, 0xd7, 0x4c),
        tertiary: Color::srgb_u8(0xa6, 0x4c, 0xd7),
        background_asset_path: Some("images/theme-4.png"),
    },
    ThemeDefinition {
        name: "Engine room",
        primary: Color::srgb_u8(0xe2, 0xe6, 0xd2),
        secondary: Color::srgb_u8(0xf0, 0x47, 0x11),
        tertiary: Color::srgb_u8(0x04, 0xed, 0x07),
        background_asset_path: Some("images/theme-5.png"),
    },
    ThemeDefinition {
        name: "Deep sea",
        primary: Color::srgb_u8(0x26, 0x65, 0xe9),
        secondary: Color::srgb_u8(0x18, 0xab, 0x52),
        tertiary: Color::srgb_u8(0x6f, 0xbd, 0xba),
        background_asset_path: Some("images/theme-6.png"),
    },
    ThemeDefinition {
        name: "Starry night",
        primary: Color::srgb_u8(0xbc, 0xee, 0xec),
        secondary: Color::srgb_u8(0xe6, 0xec, 0x94),
        tertiary: Color::srgb_u8(0xdf, 0x94, 0xec),
        background_asset_path: Some("images/theme-7.png"),
    },
    ThemeDefinition {
        name: "Alien space",
        primary: Color::srgb_u8(0xdf, 0x94, 0xec),
        secondary: Color::srgb_u8(0x32, 0x60, 0xaa),
        tertiary: Color::srgb_u8(0x5a, 0xaa, 0x32),
        background_asset_path: Some("images/theme-8.png"),
    },
    ThemeDefinition {
        name: "Black hole",
        primary: Color::srgb_u8(0x87, 0x6c, 0xcc),
        secondary: Color::srgb_u8(0x82, 0x19, 0xb1),
        tertiary: Color::srgb_u8(0x40, 0x40, 0xde),
        background_asset_path: Some("images/theme-9.png"),
    },
    ThemeDefinition {
        name: "Loneliness",
        primary: Color::srgb_u8(0x6b, 0x9f, 0xd8),
        secondary: Color::srgb_u8(0x2a, 0x6d, 0xb6),
        tertiary: Color::srgb_u8(0x6d, 0x5c, 0x99),
        background_asset_path: Some("images/theme-10.png"),
    },
    ThemeDefinition {
        name: "Cathedral",
        primary: Color::srgb_u8(0x6d, 0x5c, 0x99),
        secondary: Color::srgb_u8(0xf2, 0x2b, 0x58),
        tertiary: Color::srgb_u8(0xf2, 0xdd, 0x2b),
        background_asset_path: Some("images/theme-11.png"),
    },
    ThemeDefinition {
        name: "Runway",
        primary: Color::srgb_u8(0x96, 0x96, 0x96),
        secondary: Color::srgb_u8(0xb0, 0xb2, 0x40),
        tertiary: Color::srgb_u8(0x40, 0x77, 0xb2),
        background_asset_path: Some("images/theme-12.png"),
    },
    ThemeDefinition {
        name: "Swamp",
        primary: Color::srgb_u8(0x29, 0xb7, 0x82),
        secondary: Color::srgb_u8(0x36, 0x7f, 0xac),
        tertiary: Color::srgb_u8(0x80, 0x6d, 0x50),
        background_asset_path: Some("images/theme-13.png"),
    },
    ThemeDefinition {
        name: "Fire cavern",
        primary: Color::srgb_u8(0x9c, 0x72, 0x37),
        secondary: Color::srgb_u8(0xfd, 0x51, 0x51),
        tertiary: Color::srgb_u8(0xfa, 0xb9, 0x15),
        background_asset_path: Some("images/theme-14.png"),
    },
    ThemeDefinition {
        name: "Twilight city",
        primary: Color::srgb_u8(0xec, 0x66, 0xab),
        secondary: Color::srgb_u8(0xa7, 0x17, 0xe0),
        tertiary: Color::srgb_u8(0x30, 0x83, 0xbe),
        background_asset_path: Some("images/theme-15.png"),
    },
    ThemeDefinition {
        name: "In the clouds",
        primary: Color::srgb_u8(0xa1, 0xc2, 0xd9),
        secondary: Color::srgb_u8(0xd9, 0xd4, 0xa1),
        tertiary: Color::srgb_u8(0xff, 0xff, 0xff),
        background_asset_path: Some("images/theme-16.png"),
    },
];
