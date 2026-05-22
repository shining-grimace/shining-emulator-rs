use bevy::ecs::template::EntityTemplate;
use bevy::prelude::*;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::interactions::{
    AutoScrollFocusedChild, UiElementKind, UiScrollArea, UiScrollContent,
};
use crate::ui_elements::scrollbar::scrollbar;
use crate::ui_elements::styles::UI_SCROLLBAR_WIDTH;

pub struct ScrollViewConfig {
    pub width: Val,
    pub min_height: Val,
    pub thumb_height: f32,
}

pub fn scroll_view<S>(
    theme: ActiveTheme,
    focus_target: EntityTemplate,
    config: ScrollViewConfig,
    content: impl FnOnce(EntityTemplate) -> S,
) -> impl Scene
where
    S: Scene,
{
    bsn! {
        Node {
            width: {config.width},
            height: percent(100),
            min_height: {config.min_height},
            position_type: PositionType::Relative,
            overflow: Overflow::clip(),
        }
        Button
        UiElementKind::ScrollBar
        UiScrollArea { offset: 0.0, max_offset: 0.0 }
        AutoScrollFocusedChild
        Children [
            (
                Node {
                    position_type: PositionType::Absolute,
                    top: px(0.0),
                    left: px(0.0),
                    width: percent(100),
                }
                UiScrollContent
                Children [
                    content(focus_target)
                ]
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    top: px(0.0),
                    right: px(0.0),
                    width: px(UI_SCROLLBAR_WIDTH),
                    height: percent(100),
                }
                Children [
                    scrollbar(theme, config.thumb_height, 0.0)
                ]
            ),
        ]
    }
}
