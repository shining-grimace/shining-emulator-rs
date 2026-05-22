use bevy::prelude::*;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::interactions::{
    DraggableUiElement, IgnorePicking, UiScrollThumb, UiScrollThumbColors, UiScrollbar,
};
use crate::ui_elements::styles::{UI_SCROLLBAR_WIDTH, control_fill};

pub fn scrollbar(theme: ActiveTheme, thumb_height: f32, travel: f32) -> impl Scene {
    let track_colour = control_fill(&theme);
    let thumb_colour = theme.primary;
    bsn! {
        Node {
            width: px(UI_SCROLLBAR_WIDTH),
            height: percent(100),
            padding: UiRect::vertical(px(6.0)),
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
                UiScrollThumbColors { primary: {theme.primary}, secondary: {theme.secondary} }
            ),
        ]
    }
}
