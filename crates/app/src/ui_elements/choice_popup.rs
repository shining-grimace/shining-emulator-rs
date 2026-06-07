use bevy::asset::HandleTemplate;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;
use bevy::window::PrimaryWindow;

use crate::app_theme::ActiveTheme;
use crate::dimensions::UI_BODY_FONT_SIZE;
use crate::ui_elements::button::button;
use crate::ui_elements::interactions::tree::contains_entity;
use crate::ui_elements::interactions::{
    HoveredUiElement, InitialFocus, ModalUiElement, UiFocusNav, UiPointerClicked,
};
use crate::ui_elements::styles::{ui_border, ui_padding, ui_radius};
use crate::ui_elements::theme::{UiThemeBorderColor, UiThemeTextColor};

const CHOICE_POPUP_SCREEN_MARGIN: f32 = 16.0;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct ChoicePopupRoot;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct DismissChoicePopupOnOutsideClick {
    armed: bool,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct ChoicePopupOption {
    pub option_index: usize,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct ChoicePopupContext {
    pub context_index: usize,
}

pub struct ChoicePopupConfig {
    pub title: String,
    pub width: f32,
    pub options: [&'static str; 4],
}

pub struct ChoicePopupPlugin;

impl Plugin for ChoicePopupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, dismiss_choice_popups_on_outside_click);
    }
}

pub fn choice_popup_menu(
    font: Handle<Font>,
    theme: ActiveTheme,
    config: ChoicePopupConfig,
    position: Vec2,
    context_index: usize,
) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(position.x),
            top: px(position.y),
        }
        ChoicePopupContext { context_index: {context_index} }
        DismissChoicePopupOnOutsideClick
        Children [
            choice_popup(font, theme, config)
        ]
    }
}

pub fn choice_popup(
    font: Handle<Font>,
    theme: ActiveTheme,
    config: ChoicePopupConfig,
) -> impl Scene {
    bsn! {
        Node {
            width: px(config.width),
            border: ui_border(),
            border_radius: ui_radius(),
            padding: ui_padding(),
            flex_direction: FlexDirection::Column,
            row_gap: px(12.0),
        }
        BorderColor::all(theme.secondary)
        UiThemeBorderColor::Secondary
        BackgroundColor(Color::BLACK)
        ModalUiElement
        ChoicePopupRoot
        Children [
            popup_label(font.clone(), theme, config.title),
            (
                #Option0
                button(font.clone(), config.options[0], theme, UiFocusNav::default())
                ChoicePopupOption { option_index: 0 }
                InitialFocus { enabled: true }
                UiFocusNav { up: {Entity::PLACEHOLDER}, right: {Entity::PLACEHOLDER}, down: #Option1, left: {Entity::PLACEHOLDER} }
            ),
            (
                #Option1
                button(font.clone(), config.options[1], theme, UiFocusNav::default())
                ChoicePopupOption { option_index: 1 }
                UiFocusNav { up: #Option0, right: {Entity::PLACEHOLDER}, down: #Option2, left: {Entity::PLACEHOLDER} }
            ),
            (
                #Option2
                button(font.clone(), config.options[2], theme, UiFocusNav::default())
                ChoicePopupOption { option_index: 2 }
                UiFocusNav { up: #Option1, right: {Entity::PLACEHOLDER}, down: #Option3, left: {Entity::PLACEHOLDER} }
            ),
            (
                #Option3
                button(font, config.options[3], theme, UiFocusNav::default())
                ChoicePopupOption { option_index: 3 }
                UiFocusNav { up: #Option2, right: {Entity::PLACEHOLDER}, down: {Entity::PLACEHOLDER}, left: {Entity::PLACEHOLDER} }
            ),
        ]
    }
}

pub fn centered_choice_popup_position(
    windows: &Query<&Window, With<PrimaryWindow>>,
    width: f32,
    estimated_height: f32,
) -> Vec2 {
    let window_size = windows
        .single()
        .map(|window| Vec2::new(window.width(), window.height()))
        .unwrap_or(Vec2::new(width, estimated_height));

    let centered = Vec2::new(
        (window_size.x - width) * 0.5,
        (window_size.y - estimated_height) * 0.5,
    );

    clamp_choice_popup_position(centered, window_size, width, estimated_height)
}

pub fn clamp_choice_popup_position(
    position: Vec2,
    window_size: Vec2,
    width: f32,
    estimated_height: f32,
) -> Vec2 {
    let max_left =
        (window_size.x - width - CHOICE_POPUP_SCREEN_MARGIN).max(CHOICE_POPUP_SCREEN_MARGIN);
    let max_top = (window_size.y - estimated_height - CHOICE_POPUP_SCREEN_MARGIN)
        .max(CHOICE_POPUP_SCREEN_MARGIN);

    Vec2::new(
        position.x.clamp(CHOICE_POPUP_SCREEN_MARGIN, max_left),
        position.y.clamp(CHOICE_POPUP_SCREEN_MARGIN, max_top),
    )
}

pub fn despawn_choice_popups(
    commands: &mut Commands,
    popup_roots: &Query<(Entity, &ChoicePopupContext, &Children)>,
) {
    for (popup, _, _) in popup_roots {
        commands.entity(popup).try_despawn();
    }
}

pub fn inside_choice_popup(
    entity: Entity,
    popup_roots: &Query<(Entity, &ChoicePopupContext, &Children)>,
    child_query: &Query<&Children>,
) -> bool {
    popup_roots.iter().any(|(popup, _, children)| {
        entity == popup || contains_entity(children, entity, child_query)
    })
}

pub fn choice_popup_context_index(
    option_entity: Entity,
    popup_roots: &Query<(Entity, &ChoicePopupContext, &Children)>,
    child_query: &Query<&Children>,
) -> Option<usize> {
    popup_roots
        .iter()
        .find(|(_, _, children)| contains_entity(children, option_entity, child_query))
        .map(|(_, popup, _)| popup.context_index)
}

fn dismiss_choice_popups_on_outside_click(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut clicked: MessageReader<UiPointerClicked>,
    mut popup_roots: Query<(Entity, &Children, &mut DismissChoicePopupOnOutsideClick)>,
    hovered: Query<Entity, With<HoveredUiElement>>,
    child_query: Query<&Children>,
) {
    if popup_roots.is_empty() {
        clicked.clear();
        return;
    }

    let mut has_armed_popup = false;
    for (_, _, mut dismiss) in &mut popup_roots {
        if dismiss.armed {
            has_armed_popup = true;
        } else {
            dismiss.armed = true;
        }
    }
    if !has_armed_popup {
        clicked.clear();
        return;
    }

    let clicked_entities = clicked.read().map(|click| click.entity).collect::<Vec<_>>();
    let clicked_outside = clicked_entities
        .iter()
        .any(|entity| !inside_any_popup(*entity, &popup_roots, &child_query));
    let pressed_outside = mouse_buttons.just_pressed(MouseButton::Left)
        && !hovered
            .iter()
            .any(|entity| inside_any_popup(entity, &popup_roots, &child_query));

    if clicked_outside || pressed_outside {
        for (popup, _, dismiss) in &popup_roots {
            if !dismiss.armed {
                continue;
            }
            commands.entity(popup).despawn();
        }
    }
}

fn inside_any_popup(
    entity: Entity,
    popup_roots: &Query<(Entity, &Children, &mut DismissChoicePopupOnOutsideClick)>,
    child_query: &Query<&Children>,
) -> bool {
    popup_roots.iter().any(|(popup, children, dismiss)| {
        dismiss.armed && (entity == popup || contains_entity(children, entity, child_query))
    })
}

fn popup_label(font: Handle<Font>, theme: ActiveTheme, text: String) -> impl Scene {
    bsn! {
        Text({text})
        TextFont {
            font: FontSourceTemplate::Handle(HandleTemplate::Handle(font)),
            font_size: px(UI_BODY_FONT_SIZE),
        }
        TextColor({theme.primary})
        UiThemeTextColor::Primary
        TextLayout::new(Justify::Left, LineBreak::WordBoundary)
    }
}
