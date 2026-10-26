#![allow(dead_code)]

pub mod events;
pub mod game_boy;
pub mod mappings;
pub mod selection;
pub(crate) mod touch_overlay;

pub(crate) mod controller;
pub(crate) mod key_ids;
mod systems;

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::input::controller::ConnectedControllers;
use crate::input::events::MappedInputEvent;
use crate::input::game_boy::GameBoyInputState;
use crate::input::mappings::RuntimeInputMappings;
use crate::input::selection::{InputMappingEditTarget, PrimaryInputDevice};
use crate::input::systems::{
    collect_controller_input, collect_keyboard_input, register_connected_controllers,
    update_game_boy_input_state,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, SystemSet)]
pub enum InputSet {
    Collect,
    UpdateGameBoyState,
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RuntimeInputMappings>()
            .init_resource::<ConnectedControllers>()
            .init_resource::<GameBoyInputState>()
            .init_resource::<PrimaryInputDevice>()
            .init_resource::<InputMappingEditTarget>()
            .init_resource::<touch_overlay::TouchControllerOverlayInput>()
            .add_message::<MappedInputEvent>()
            .add_systems(
                Update,
                (
                    register_connected_controllers,
                    collect_keyboard_input,
                    collect_controller_input,
                )
                    .chain()
                    .in_set(InputSet::Collect),
            )
            .add_systems(
                Update,
                touch_overlay::collect_touch_controller_overlay_input
                    .in_set(InputSet::Collect)
                    .run_if(in_state(AppState::Gameplay)),
            )
            .add_systems(
                Update,
                update_game_boy_input_state
                    .in_set(InputSet::UpdateGameBoyState)
                    .after(InputSet::Collect),
            )
            .add_systems(
                Update,
                touch_overlay::update_touch_controller_overlay_visuals
                    .after(InputSet::UpdateGameBoyState)
                    .run_if(in_state(AppState::Gameplay)),
            )
            .add_systems(
                OnEnter(AppState::Gameplay),
                touch_overlay::spawn_touch_controller_overlay,
            )
            .add_systems(
                OnExit(AppState::Gameplay),
                (
                    touch_overlay::release_touch_controller_overlay_input,
                    touch_overlay::despawn_touch_controller_overlay,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod button_tests {
    use super::*;
    use crate::storage::input_mappings::{default_controller_mapping, default_keyboard_mapping};
    use crate::ui_elements::interactions::*;
    use bevy::input::{
        ButtonState,
        gamepad::{GamepadButton, GamepadButtonStateChangedEvent},
        keyboard::{Key, KeyboardInput},
    };
    use bevy::state::app::StatesPlugin;
    use bevy::ui::{InteractionDisabled, Pressed};
    use bevy::ui_widgets::{Activate, Button, ButtonPlugin};

    #[derive(Resource, Default)]
    struct Activations {
        app: Vec<Entity>,
        bevy: Vec<Entity>,
    }

    fn app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            bevy::input::InputPlugin,
            ButtonPlugin,
        ))
        .insert_state(AppState::Settings)
        .init_resource::<crate::settings_transition::SettingsTransition>()
        .init_resource::<UiScale>()
        .init_resource::<ConnectedControllers>()
        .init_resource::<Activations>()
        .insert_resource(RuntimeInputMappings::from_storage_mappings(&[
            default_keyboard_mapping(),
            default_controller_mapping("test-controller"),
        ]))
        .add_message::<MappedInputEvent>()
        .add_message::<PointerPress>()
        .add_message::<PointerRelease>()
        .add_message::<PointerDrag>()
        .add_message::<PointerDragEnd>()
        .add_message::<PointerCancel>()
        .add_plugins(UiInteractionsPlugin)
        .add_systems(
            Update,
            (collect_keyboard_input, collect_controller_input)
                .chain()
                .in_set(InputSet::Collect),
        )
        .add_observer(
            |event: On<Add<ActivatedUiElement>>, mut log: ResMut<Activations>| {
                log.app.push(event.entity)
            },
        )
        .add_observer(|event: On<Activate>, mut log: ResMut<Activations>| {
            log.bevy.push(event.entity)
        });
        let gamepad = app.world_mut().spawn_empty().id();
        app.world_mut()
            .resource_mut::<ConnectedControllers>()
            .insert(controller::ConnectedController {
                entity: gamepad,
                name: "Test controller".into(),
                model_id: "test-controller".into(),
            });
        (app, gamepad)
    }

    fn button(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                Button,
                Node::default(),
                UiElementKind::Button,
                UiFocusNav::default(),
            ))
            .id()
    }

    fn key(app: &mut App, code: KeyCode, state: ButtonState, repeat: bool) {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            state,
            text: None,
            repeat,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }

    fn controller(app: &mut App, entity: Entity, button: GamepadButton, state: ButtonState) {
        app.world_mut()
            .write_message(GamepadButtonStateChangedEvent::new(entity, button, state));
        app.update();
    }

    #[test]
    fn keyboard_mapping_activates_once_per_press_and_ignores_repeat_and_release() {
        let (mut app, _) = app();
        let target = button(&mut app);
        app.world_mut()
            .entity_mut(target)
            .insert(InitialFocus { enabled: true });
        app.update();
        assert!(app.world().get::<FocusedUiElement>(target).is_some());
        key(&mut app, KeyCode::KeyX, ButtonState::Pressed, false);
        app.update();
        key(&mut app, KeyCode::KeyX, ButtonState::Pressed, true);
        key(&mut app, KeyCode::KeyX, ButtonState::Released, false);
        assert_eq!(app.world().resource::<Activations>().app, vec![target]);
        key(&mut app, KeyCode::KeyX, ButtonState::Pressed, false);
        assert_eq!(
            app.world().resource::<Activations>().app,
            vec![target, target]
        );
    }

    #[test]
    fn controller_mapping_navigates_and_activates_once_without_a_keyboard() {
        let (mut app, pad) = app();
        let first = button(&mut app);
        let next = button(&mut app);
        app.world_mut().entity_mut(first).insert((
            InitialFocus { enabled: true },
            UiFocusNav {
                down: next,
                ..default()
            },
        ));
        app.update();
        controller(&mut app, pad, GamepadButton::DPadDown, ButtonState::Pressed);
        assert!(app.world().get::<FocusedUiElement>(first).is_none());
        assert!(app.world().get::<FocusedUiElement>(next).is_some());
        controller(&mut app, pad, GamepadButton::East, ButtonState::Pressed);
        controller(&mut app, pad, GamepadButton::East, ButtonState::Released);
        app.update();
        assert_eq!(app.world().resource::<Activations>().app, vec![next]);
    }

    #[test]
    fn navigation_skips_disabled_and_hidden_buttons() {
        let (mut app, _) = app();
        let first = button(&mut app);
        let disabled = button(&mut app);
        let hidden = button(&mut app);
        let last = button(&mut app);
        app.world_mut().entity_mut(first).insert((
            InitialFocus { enabled: true },
            UiFocusNav {
                down: disabled,
                ..default()
            },
        ));
        app.world_mut().entity_mut(disabled).insert((
            DisabledUiElement,
            UiFocusNav {
                down: hidden,
                ..default()
            },
        ));
        app.world_mut().entity_mut(hidden).insert(UiFocusNav {
            down: last,
            ..default()
        });
        app.world_mut().get_mut::<Node>(hidden).unwrap().display = Display::None;
        app.update();
        key(&mut app, KeyCode::ArrowDown, ButtonState::Pressed, false);
        assert!(app.world().get::<FocusedUiElement>(last).is_some());
    }

    #[test]
    fn disabling_a_focused_button_blocks_all_activation_paths_and_can_be_reversed() {
        use crate::ui_elements::interactions::test_support::*;
        let (mut app, pad) = app();
        let target = button(&mut app);
        app.world_mut()
            .entity_mut(target)
            .insert(InitialFocus { enabled: true });
        app.update();
        app.world_mut()
            .entity_mut(target)
            .insert((Pressed, DisabledUiElement));
        assert!(app.world().get::<InteractionDisabled>(target).is_some());
        assert!(app.world().get::<Pressed>(target).is_none());
        key(&mut app, KeyCode::KeyX, ButtonState::Pressed, false);
        controller(&mut app, pad, GamepadButton::East, ButtonState::Pressed);
        app.world_mut()
            .write_message(UiPointerClicked { entity: target });
        app.world_mut().trigger(press(target, 1));
        app.world_mut()
            .trigger(bevy::picking::events::PointerClick {
                entity: target,
                pointer: pointer(1),
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: hit(),
                duration: std::time::Duration::ZERO,
                count: 1,
            });
        app.update();
        assert!(app.world().get::<FocusedUiElement>(target).is_none());
        assert!(app.world().resource::<Activations>().app.is_empty());
        assert!(app.world().resource::<Activations>().bevy.is_empty());
        app.world_mut()
            .entity_mut(target)
            .remove::<DisabledUiElement>();
        assert!(app.world().get::<InteractionDisabled>(target).is_none());
        controller(&mut app, pad, GamepadButton::East, ButtonState::Released);
        controller(&mut app, pad, GamepadButton::East, ButtonState::Pressed);
        assert_eq!(app.world().resource::<Activations>().app, vec![target]);
    }

    #[test]
    fn hidden_focused_controls_and_captured_input_cannot_activate() {
        let (mut app, pad) = app();
        let target = button(&mut app);
        app.world_mut()
            .entity_mut(target)
            .insert(InitialFocus { enabled: true });
        app.update();
        let parent = app
            .world_mut()
            .spawn(Node {
                display: Display::None,
                ..default()
            })
            .add_child(target)
            .id();
        controller(&mut app, pad, GamepadButton::East, ButtonState::Pressed);
        app.world_mut()
            .write_message(UiPointerClicked { entity: target });
        app.update();
        assert!(app.world().get::<FocusedUiElement>(target).is_none());
        assert!(app.world().resource::<Activations>().app.is_empty());
        app.world_mut().get_mut::<Node>(parent).unwrap().display = Display::Flex;
        app.world_mut().entity_mut(target).insert(FocusedUiElement);
        app.world_mut().resource_mut::<UiInputCapture>().active = true;
        app.world_mut()
            .write_message(UiPointerClicked { entity: target });
        key(&mut app, KeyCode::KeyX, ButtonState::Pressed, false);
        assert!(app.world().resource::<Activations>().app.is_empty());
    }
}
