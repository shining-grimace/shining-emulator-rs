use bevy::prelude::*;

use crate::app_theme::ActiveTheme;
use crate::dimensions::{SCROLLBAR_TRACK_PADDING, UI_SCROLLBAR_WIDTH};
use crate::ui_elements::interactions::{
    DraggableUiElement, IgnorePicking, UiScrollThumb, UiScrollThumbColors, UiScrollbar,
};
use crate::ui_elements::styles::control_fill;
use crate::ui_elements::theme::{UiScrollThumbTheme, UiThemeBackgroundColor};

pub fn scrollbar(theme: ActiveTheme, thumb_height: f32, travel: f32) -> impl Scene {
    scrollbar_with_display(theme, thumb_height, travel, Display::Flex)
}

pub fn scrollbar_with_display(
    theme: ActiveTheme,
    thumb_height: f32,
    travel: f32,
    display: Display,
) -> impl Scene {
    let track_colour = control_fill(&theme);
    let thumb_colour = theme.primary;
    bsn! {
        Node {
            display: {display},
            width: px(UI_SCROLLBAR_WIDTH),
            height: percent(100),
            padding: UiRect::vertical(px(SCROLLBAR_TRACK_PADDING)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        UiScrollbar
        Children [
            (
                Node {
                    width: px(UI_SCROLLBAR_WIDTH),
                    height: percent(100),
                    border_radius: BorderRadius::MAX,
                }
                BackgroundColor({track_colour})
                UiThemeBackgroundColor::ControlFill
                IgnorePicking
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    width: px(UI_SCROLLBAR_WIDTH),
                    height: px(thumb_height),
                    top: px(6.0),
                    border_radius: BorderRadius::MAX,
                }
                BackgroundColor({thumb_colour})
                DraggableUiElement
                UiScrollThumb { height: {thumb_height}, travel: {travel} }
                UiScrollThumbTheme
                UiScrollThumbColors { primary: {theme.primary}, secondary: {theme.secondary} }
            ),
        ]
    }
}
