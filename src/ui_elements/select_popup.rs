use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::interactions::{BlockPickingOnly, IgnorePicking};
use crate::ui_elements::styles::{
    UI_BODY_FONT_SIZE, UI_ELEMENT_HEIGHT, UI_INNER_PADDING, hover_fill, ui_border, ui_radius,
};

pub struct SelectPopupOption {
    pub label: &'static str,
    pub focused: bool,
}

pub struct SelectPopupConfig {
    pub title: &'static str,
    pub width: f32,
    pub options: Vec<SelectPopupOption>,
}

pub fn select_popup(
    font: Handle<Font>,
    theme: ActiveTheme,
    config: SelectPopupConfig,
) -> impl Scene {
    let title = popup_label(font.clone(), theme.primary, config.title);
    let options = config
        .options
        .into_iter()
        .map(|option| popup_option(font.clone(), option, theme))
        .collect::<Vec<_>>();

    bsn! {
        Node {
            width: px(config.width),
            border: ui_border(),
            border_radius: ui_radius(),
            padding: UiRect::all(px(UI_INNER_PADDING)),
            flex_direction: FlexDirection::Column,
            row_gap: px(12.0),
        }
        BorderColor::all(theme.primary)
        BackgroundColor(Color::BLACK)
        BlockPickingOnly
        Children [
            title,
            {options}
        ]
    }
}

fn popup_option(font: Handle<Font>, option: SelectPopupOption, theme: ActiveTheme) -> impl Scene {
    let border = if option.focused {
        theme.secondary
    } else {
        Color::NONE
    };
    let background = if option.focused {
        hover_fill(&theme)
    } else {
        Color::NONE
    };
    let colour = if option.focused {
        theme.secondary
    } else {
        theme.primary
    };

    bsn! {
        Node {
            width: percent(100),
            height: px(UI_ELEMENT_HEIGHT),
            border: ui_border(),
            border_radius: ui_radius(),
            padding: UiRect::horizontal(px(UI_INNER_PADDING)),
            align_items: AlignItems::Center,
        }
        BorderColor { top: {border}, right: {border}, bottom: {border}, left: {border} }
        BackgroundColor({background})
        IgnorePicking
        Children [
            popup_label(font, colour, option.label)
        ]
    }
}

fn popup_label(font: Handle<Font>, colour: Color, text: &'static str) -> impl Scene {
    bsn! {
        Text({text})
        TextFont {
            font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
            font_size: px(UI_BODY_FONT_SIZE),
        }
        TextColor({colour})
        IgnorePicking
        TextLayout::new(Justify::Left, LineBreak::NoWrap)
    }
}
