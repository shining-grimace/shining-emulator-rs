use bevy::color::Alpha;
use bevy::prelude::*;

use crate::app_theme::{ActiveTheme, ActiveThemeChanged};
use crate::ui_elements::file_picker::UiFilePickerHoverFill;
use crate::ui_elements::interactions::{UiElementColors, UiScrollThumbColors};
use crate::ui_elements::styles::{control_fill, hover_fill};

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub enum UiElementTheme {
    #[default]
    Control,
    TransparentControl,
    List,
    ListItem,
    ScrollBar,
    PopupOption,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub enum UiThemeTextColor {
    #[default]
    Primary,
    Secondary,
    Tertiary,
    Black,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub enum UiThemeBorderColor {
    #[default]
    Primary,
    Secondary,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub enum UiThemeBackgroundColor {
    #[default]
    Primary,
    ControlFill,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub enum UiThemeImageColor {
    #[default]
    Primary,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiScrollThumbTheme;

pub struct UiThemePlugin;

impl Plugin for UiThemePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(update_ui_theme_colours);
    }
}

pub fn element_colours(theme: &ActiveTheme, element_theme: UiElementTheme) -> UiElementColors {
    match element_theme {
        UiElementTheme::Control => UiElementColors {
            primary: theme.primary,
            secondary: theme.secondary,
            tertiary: theme.tertiary,
            fill: control_fill(theme),
            hover_fill: hover_fill(theme),
        },
        UiElementTheme::TransparentControl => UiElementColors {
            primary: theme.primary,
            secondary: theme.secondary,
            tertiary: theme.tertiary,
            fill: Color::NONE,
            hover_fill: Color::NONE,
        },
        UiElementTheme::List => UiElementColors {
            primary: theme.primary,
            secondary: theme.secondary,
            tertiary: theme.primary,
            fill: Color::NONE,
            hover_fill: Color::NONE,
        },
        UiElementTheme::ListItem => UiElementColors {
            primary: theme.primary,
            secondary: theme.secondary,
            tertiary: theme.primary,
            fill: Color::NONE,
            hover_fill: hover_fill(theme),
        },
        UiElementTheme::ScrollBar => UiElementColors {
            primary: theme.secondary,
            secondary: theme.secondary,
            tertiary: control_fill(theme),
            fill: Color::NONE,
            hover_fill: Color::NONE,
        },
        UiElementTheme::PopupOption => UiElementColors {
            primary: theme.primary,
            secondary: theme.secondary,
            tertiary: theme.tertiary,
            fill: Color::BLACK,
            hover_fill: hover_fill(theme),
        },
    }
}

fn update_ui_theme_colours(
    _theme_changed: On<ActiveThemeChanged>,
    theme: Res<ActiveTheme>,
    mut elements: Query<(&UiElementTheme, &mut UiElementColors)>,
    mut text_colours: Query<(&UiThemeTextColor, &mut TextColor)>,
    mut borders: Query<(&UiThemeBorderColor, &mut BorderColor)>,
    mut backgrounds: Query<(&UiThemeBackgroundColor, &mut BackgroundColor)>,
    mut images: Query<(&UiThemeImageColor, &mut ImageNode)>,
    mut scroll_thumbs: Query<&mut UiScrollThumbColors, With<UiScrollThumbTheme>>,
    mut file_picker_fills: Query<&mut UiFilePickerHoverFill>,
) {
    for (element_theme, mut colours) in &mut elements {
        *colours = element_colours(&theme, *element_theme);
    }

    for (role, mut text_colour) in &mut text_colours {
        let alpha = text_colour.0.alpha();
        text_colour.0 = match role {
            UiThemeTextColor::Primary => theme.primary,
            UiThemeTextColor::Secondary => theme.secondary,
            UiThemeTextColor::Tertiary => theme.tertiary,
            UiThemeTextColor::Black => Color::BLACK,
        };
        text_colour.0.set_alpha(alpha);
    }

    for (role, mut border) in &mut borders {
        *border = BorderColor::all(match role {
            UiThemeBorderColor::Primary => theme.primary,
            UiThemeBorderColor::Secondary => theme.secondary,
        });
    }

    for (role, mut background) in &mut backgrounds {
        background.0 = match role {
            UiThemeBackgroundColor::Primary => theme.primary,
            UiThemeBackgroundColor::ControlFill => control_fill(&theme),
        };
    }

    for (role, mut image) in &mut images {
        let alpha = image.color.alpha();
        image.color = match role {
            UiThemeImageColor::Primary => theme.primary,
        };
        image.color.set_alpha(alpha);
    }

    for mut colours in &mut scroll_thumbs {
        colours.primary = theme.primary;
        colours.secondary = theme.secondary;
    }

    for mut fills in &mut file_picker_fills {
        fills.fill = control_fill(&theme);
        fills.hover_fill = hover_fill(&theme);
    }
}
