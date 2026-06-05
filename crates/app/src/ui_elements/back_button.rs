use bevy::asset::HandleTemplate;
use bevy::math::Rect;
use bevy::prelude::*;

use crate::app_theme::ActiveTheme;
use crate::dimensions::{BACK_BUTTON_SIZE, BACK_ICON_SIZE_PX};
use crate::ui_elements::interactions::IgnorePicking;
use crate::ui_elements::theme::UiThemeImageColor;

const ICON_TEXTURE_SIZE: f32 = 1024.0;
const BACK_ICON_X: f32 = 0.25;
const BACK_ICON_Y: f32 = 0.5;
const BACK_ICON_SIZE: f32 = 0.25;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiBackButton;

pub fn back_button(icons: Handle<Image>, theme: ActiveTheme) -> impl Scene {
    bsn! {
        Node {
            width: px(BACK_BUTTON_SIZE),
            height: px(BACK_BUTTON_SIZE),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Button
        UiBackButton
        Children [
            (
                Node {
                    width: px(BACK_ICON_SIZE_PX),
                    height: px(BACK_ICON_SIZE_PX),
                }
                ImageNode {
                    image: HandleTemplate::Handle(icons),
                    color: {theme.primary},
                    rect: {Some(back_icon_rect())},
                }
                UiThemeImageColor::Primary
                IgnorePicking
            )
        ]
    }
}

fn back_icon_rect() -> Rect {
    Rect {
        min: Vec2::new(
            BACK_ICON_X * ICON_TEXTURE_SIZE,
            BACK_ICON_Y * ICON_TEXTURE_SIZE,
        ),
        max: Vec2::new(
            (BACK_ICON_X + BACK_ICON_SIZE) * ICON_TEXTURE_SIZE,
            (BACK_ICON_Y + BACK_ICON_SIZE) * ICON_TEXTURE_SIZE,
        ),
    }
}
