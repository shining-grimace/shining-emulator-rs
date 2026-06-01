use bevy::prelude::*;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::back_button::back_button;
use crate::ui_elements::heading::heading;

pub fn settings_header(
    font: Handle<Font>,
    icons: Handle<Image>,
    theme: ActiveTheme,
    label: impl Into<String>,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(18.0),
        }
        Children [
            (
                Node {
                    flex_grow: 1.0,
                    min_width: px(0.0),
                }
                Children [
                    heading(font, theme, label),
                ]
            ),
            back_button(icons, theme),
        ]
    }
}
