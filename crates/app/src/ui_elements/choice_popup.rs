use bevy::asset::HandleTemplate;
use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;
use bevy::ui::UiScale;
use bevy::window::PrimaryWindow;

use crate::app_theme::ActiveTheme;
use crate::dimensions::UI_BODY_FONT_SIZE;
use crate::ui_elements::button::button;
use crate::ui_elements::interactions::tree::contains_entity;
use crate::ui_elements::interactions::{
    HoveredUiElement, InitialFocus, ModalUiElement, UI_FOCUS_NONE, UiFocusId, UiFocusNav,
    UiFocusNavIds, UiPointerClicked,
};
use crate::ui_elements::styles::{ui_border, ui_padding, ui_radius};
use crate::ui_elements::theme::{UiThemeBorderColor, UiThemeTextColor};

const CHOICE_POPUP_SCREEN_MARGIN: f32 = 16.0;
const CHOICE_POPUP_OPTION_FOCUS_BASE: u16 = 60_000;

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
    pub options: Vec<&'static str>,
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
            {choice_popup_options(font, theme, config.options)}
        ]
    }
}

fn choice_popup_options(
    font: Handle<Font>,
    theme: ActiveTheme,
    options: Vec<&'static str>,
) -> Vec<Box<dyn SceneList>> {
    let option_count = options.len();
    options
        .into_iter()
        .enumerate()
        .map(|(option_index, label)| {
            let focus_id = choice_popup_option_focus_id(option_index);
            let up = option_index
                .checked_sub(1)
                .map(choice_popup_option_focus_id)
                .unwrap_or(UI_FOCUS_NONE);
            let down = if option_index + 1 < option_count {
                choice_popup_option_focus_id(option_index + 1)
            } else {
                UI_FOCUS_NONE
            };
            Box::new(bsn_list![(
                button(font.clone(), label, theme, UiFocusNav::default())
                UiFocusId { id: {focus_id} }
                UiFocusNavIds { up: {up}, right: UI_FOCUS_NONE, down: {down}, left: UI_FOCUS_NONE }
                ChoicePopupOption { option_index }
                InitialFocus { enabled: {option_index == 0} }
            )]) as Box<dyn SceneList>
        })
        .collect()
}

fn choice_popup_option_focus_id(option_index: usize) -> u16 {
    u16::try_from(option_index)
        .ok()
        .and_then(|option| CHOICE_POPUP_OPTION_FOCUS_BASE.checked_add(option))
        .unwrap_or(UI_FOCUS_NONE)
}

pub fn centered_choice_popup_position(
    windows: &Query<&Window, With<PrimaryWindow>>,
    width: f32,
    estimated_height: f32,
) -> Vec2 {
    centered_choice_popup_position_for_scale(windows, 1.0, width, estimated_height)
}

pub fn centered_scaled_choice_popup_position(
    windows: &Query<&Window, With<PrimaryWindow>>,
    ui_scale: &UiScale,
    width: f32,
    estimated_height: f32,
) -> Vec2 {
    centered_choice_popup_position_for_scale(windows, ui_scale.0, width, estimated_height)
}

fn centered_choice_popup_position_for_scale(
    windows: &Query<&Window, With<PrimaryWindow>>,
    ui_scale: f32,
    width: f32,
    estimated_height: f32,
) -> Vec2 {
    let scale = ui_scale.max(f32::EPSILON);
    let window_size = windows
        .single()
        .map(|window| Vec2::new(window.width(), window.height()) / scale)
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
    popup_roots: &Query<(Entity, &ChoicePopupContext, &Children), impl QueryFilter>,
) {
    for (popup, _, _) in popup_roots {
        commands.entity(popup).try_despawn();
    }
}

pub fn inside_choice_popup(
    entity: Entity,
    popup_roots: &Query<(Entity, &ChoicePopupContext, &Children), impl QueryFilter>,
    child_query: &Query<&Children>,
) -> bool {
    popup_roots.iter().any(|(popup, _, children)| {
        entity == popup || contains_entity(children, entity, child_query)
    })
}

pub fn choice_popup_context_index(
    option_entity: Entity,
    popup_roots: &Query<(Entity, &ChoicePopupContext, &Children), impl QueryFilter>,
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
            commands.entity(popup).try_despawn();
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
