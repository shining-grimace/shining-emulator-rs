use bevy::asset::HandleTemplate;
use bevy::input::ButtonState;
use bevy::math::Rect;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::dimensions::{
    GAMEPLAY_ERROR_ICON_SIZE, UI_BODY_FONT_SIZE, UI_CONTENT_GAP, UI_SCREEN_PADDING,
};
use crate::game_boy::GameBoyLoadStatus;
use crate::input::events::MappedInputEvent;
use crate::input::selection::PrimaryInputDevice;
use crate::storage::LocalStorage;
use crate::storage::input_mappings::InputAction;
use crate::ui_elements::action_hint::action_hints_for_actions;

const ICON_TEXTURE_SIZE: f32 = 1024.0;
const ICON_GRID_UNITS: f32 = 16.0;
const ERROR_ICON_X: f32 = 0.0;
const ERROR_ICON_Y: f32 = 8.0;
const ERROR_ICON_SIZE: f32 = 4.0;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct GameplayErrorOverlay;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct GameplayErrorMessage;

pub struct GameplayScenePlugin;

impl Plugin for GameplayScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Gameplay), spawn_gameplay_scene)
            .add_systems(
                Update,
                (return_home_from_gameplay, update_gameplay_error_overlay)
                    .run_if(in_state(AppState::Gameplay)),
            );
    }
}

fn spawn_gameplay_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    storage: Res<LocalStorage>,
    primary_input: Res<PrimaryInputDevice>,
) {
    commands.spawn_scene(gameplay_scene(&assets, *theme, &storage, &primary_input));
}

fn return_home_from_gameplay(
    mut input_events: MessageReader<MappedInputEvent>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if input_events
        .read()
        .any(|event| event.state == ButtonState::Pressed && event.action == InputAction::QuitRom)
    {
        next_state.set(AppState::Home);
    }
}

fn update_gameplay_error_overlay(
    status: Res<GameBoyLoadStatus>,
    mut overlays: Query<&mut Node, With<GameplayErrorOverlay>>,
    mut messages: Query<&mut Text, With<GameplayErrorMessage>>,
) {
    let message = status.overlay_message();
    let display = if message.is_some() {
        Display::Flex
    } else {
        Display::None
    };

    for mut node in &mut overlays {
        node.display = display;
    }
    let Some(message) = message else {
        return;
    };
    for mut text in &mut messages {
        if text.0 != message {
            text.0 = message.to_string();
        }
    }
}

fn gameplay_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    storage: &LocalStorage,
    primary_input: &PrimaryInputDevice,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let hint_font = font.clone();

    bsn! {
        #GameplayScene
        DespawnOnExit::<AppState>(AppState::Gameplay)
        Node {
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(UI_SCREEN_PADDING)),
            flex_direction: FlexDirection::Column,
            row_gap: px(UI_CONTENT_GAP),
        }
        Children [
            (
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_height: px(0.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                Children [
                    (
                        Node {
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            flex_direction: FlexDirection::Column,
                            row_gap: px(UI_CONTENT_GAP),
                        }
                        GameplayErrorOverlay
                        Children [
                            (
                                Node {
                                    width: px(GAMEPLAY_ERROR_ICON_SIZE),
                                    height: px(GAMEPLAY_ERROR_ICON_SIZE),
                                }
                                ImageNode {
                                    image: {HandleTemplate::Handle(assets.icons.clone())},
                                    color: {theme.secondary},
                                    rect: {Some(error_icon_rect())},
                                }
                            ),
                            (
                                Text("Loading ROM...")
                                GameplayErrorMessage
                                TextFont {
                                    font: {FontSourceTemplate::Handle(HandleTemplate::Handle(font))},
                                    font_size: px(UI_BODY_FONT_SIZE),
                                }
                                TextColor({theme.secondary})
                                TextLayout::new(Justify::Center, LineBreak::WordBoundary)
                            )
                        ]
                    )
                ]
            ),
            action_hints_for_actions(
                hint_font,
                assets.icons.clone(),
                theme,
                storage,
                primary_input,
                (InputAction::B, "Quit"),
                (InputAction::PauseAndResume, "Pause"),
            ),
        ]
    }
}

fn error_icon_rect() -> Rect {
    let unit = ICON_TEXTURE_SIZE / ICON_GRID_UNITS;
    Rect {
        min: Vec2::new(ERROR_ICON_X * unit, ERROR_ICON_Y * unit),
        max: Vec2::new(
            (ERROR_ICON_X + ERROR_ICON_SIZE) * unit,
            (ERROR_ICON_Y + ERROR_ICON_SIZE) * unit,
        ),
    }
}
