use bevy::asset::HandleTemplate;
use bevy::color::Alpha;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_theme::ActiveTheme;
use crate::ui_elements::styles::UI_BODY_FONT_SIZE;
use crate::ui_elements::theme::UiThemeTextColor;

pub const INFO_MESSAGE_VISIBLE_SECONDS: f32 = 3.5;
pub const INFO_MESSAGE_FADE_SECONDS: f32 = 1.0;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct InfoMessage {
    pub elapsed_seconds: f32,
    pub fades: bool,
    pub despawn_after_fade: bool,
}

pub struct InfoMessagePlugin;

impl Plugin for InfoMessagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fade_info_messages);
    }
}

pub fn info_message(
    font: Handle<Font>,
    theme: ActiveTheme,
    text: &'static str,
    fades: bool,
) -> impl Scene {
    info_message_text(font, theme, text.to_string(), fades)
}

pub fn info_message_text(
    font: Handle<Font>,
    theme: ActiveTheme,
    text: String,
    fades: bool,
) -> impl Scene {
    bsn! {
        Text({text})
        TextFont {
            font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
            font_size: px(UI_BODY_FONT_SIZE),
        }
        TextColor({theme.secondary})
        UiThemeTextColor::Secondary
        InfoMessage {
            elapsed_seconds: 0.0,
            fades: {fades},
            despawn_after_fade: {fades},
        }
        TextLayout::new(Justify::Left, LineBreak::WordBoundary)
    }
}

pub fn set_latest_info_message(
    messages: &mut Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
    message: &str,
) {
    if let Some((mut text, mut colour, mut message_state)) = messages.iter_mut().last() {
        text.0 = message.to_string();
        message_state.elapsed_seconds = 0.0;
        message_state.fades = true;
        message_state.despawn_after_fade = false;
        colour.0.set_alpha(1.0);
    }
}

fn fade_info_messages(
    mut commands: Commands,
    time: Res<Time>,
    mut messages: Query<(Entity, &mut InfoMessage, &mut TextColor)>,
) {
    for (entity, mut message, mut colour) in &mut messages {
        if !message.fades {
            continue;
        }

        message.elapsed_seconds += time.delta_secs();
        let fade_elapsed = message.elapsed_seconds - INFO_MESSAGE_VISIBLE_SECONDS;
        if fade_elapsed <= 0.0 {
            continue;
        }

        let alpha = 1.0 - fade_elapsed / INFO_MESSAGE_FADE_SECONDS;
        if alpha <= 0.0 {
            if message.despawn_after_fade {
                commands.entity(entity).despawn();
            } else {
                message.fades = false;
                message.elapsed_seconds = 0.0;
                colour.0.set_alpha(0.0);
            }
        } else {
            colour.0.set_alpha(alpha);
        }
    }
}
