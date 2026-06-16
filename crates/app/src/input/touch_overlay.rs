use std::collections::HashMap;

use bevy::asset::HandleTemplate;
use bevy::color::Alpha;
use bevy::input::ButtonState;
use bevy::math::Rect;
use bevy::picking::{
    events::{Cancel, DragEnter, DragLeave, DragOver, Pointer, Press, Release},
    pointer::{PointerButton, PointerId},
};
use bevy::prelude::*;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::dimensions::{
    ACTION_HINT_ICON_SIZE, TOUCH_OVERLAY_DPAD_CLUSTER_HEIGHT, TOUCH_OVERLAY_DPAD_CLUSTER_SIZE,
    TOUCH_OVERLAY_DPAD_ICON_LONG, TOUCH_OVERLAY_DPAD_ICON_SHORT, TOUCH_OVERLAY_FACE_BUTTON_GAP,
    TOUCH_OVERLAY_FACE_BUTTON_SIZE, TOUCH_OVERLAY_FACE_ICON_SIZE, TOUCH_OVERLAY_MARGIN,
    TOUCH_OVERLAY_SYSTEM_BUTTON_GAP, TOUCH_OVERLAY_SYSTEM_BUTTON_HEIGHT,
    TOUCH_OVERLAY_SYSTEM_BUTTON_WIDTH, TOUCH_OVERLAY_SYSTEM_ICON_HEIGHT,
    TOUCH_OVERLAY_SYSTEM_ICON_WIDTH, TOUCH_OVERLAY_SYSTEM_ROW_GAP, UI_SCREEN_PADDING,
};
use crate::input::controller::ConnectedControllers;
use crate::input::events::MappedInputEvent;
use crate::input::game_boy::GameBoyInputState;
use crate::input::selection::{PrimaryInputDevice, selected_mapping_has_available_device};
use crate::storage::LocalStorage;
use crate::storage::input_mappings::InputAction;
use crate::ui_elements::interactions::IgnorePicking;

const ICON_TEXTURE_SIZE: f32 = 1024.0;
const ICON_GRID_UNITS: f32 = 16.0;
const DPAD_ICON_X: f32 = 0.0;
const DPAD_ICON_Y: f32 = 0.0;
const DPAD_ICON_WIDTH: f32 = 8.0;
const DPAD_ICON_HEIGHT: f32 = 4.0;
const B_ICON_X: f32 = 0.0;
const B_ICON_Y: f32 = 4.0;
const A_ICON_X: f32 = 4.0;
const A_ICON_Y: f32 = 4.0;
const FACE_ICON_SIZE: f32 = 4.0;
const SELECT_ICON_X: f32 = 8.0;
const SELECT_ICON_Y: f32 = 0.0;
const START_ICON_X: f32 = 8.0;
const START_ICON_Y: f32 = 2.0;
const SYSTEM_ICON_WIDTH: f32 = 4.0;
const SYSTEM_ICON_HEIGHT: f32 = 2.0;
const INACTIVE_ICON_ALPHA: f32 = 0.78;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct TouchControllerButton {
    pub action: InputAction,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct TouchControllerButtonIcon {
    pub action: InputAction,
}

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct TouchControllerButtonIconFrame {
    pub action: InputAction,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct TouchControllerOverlayRoot;

#[derive(Default, Resource)]
pub(super) struct TouchControllerOverlayInput {
    pressed_pointers: HashMap<PointerId, TouchPointerState>,
    pressed_actions: HashMap<InputAction, usize>,
}

#[derive(Clone, Copy, Debug)]
struct TouchPointerState {
    primary_action: InputAction,
    hover_action: Option<InputAction>,
}

pub(crate) fn should_show_touch_controller_overlay(storage: &LocalStorage) -> bool {
    storage.data.settings.force_button_overlay > 0 || cfg!(target_os = "android")
}

pub(crate) fn touch_controller_overlay(
    icons: Handle<Image>,
    theme: ActiveTheme,
    visible: bool,
    despawn_state: AppState,
    reserve_action_hints: bool,
) -> impl Scene {
    let display = if visible {
        Display::Flex
    } else {
        Display::None
    };
    let bottom = overlay_bottom_offset(reserve_action_hints);
    let dpad_icons = icons.clone();
    let face_icons = icons.clone();
    let system_icons = icons;

    bsn! {
        Node {
            display: {display},
            position_type: PositionType::Absolute,
            left: px(0.0),
            right: px(0.0),
            top: px(0.0),
            bottom: px(0.0),
            width: percent(100),
            height: percent(100),
        }
        GlobalZIndex(90)
        TouchControllerOverlayRoot
        DespawnOnExit::<AppState>({despawn_state})
        Children [
            dpad_cluster(dpad_icons, theme, bottom),
            face_button_cluster(face_icons, theme, bottom),
            system_button_cluster(system_icons, theme, bottom),
        ]
    }
}

pub(super) fn spawn_touch_controller_overlay(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    storage: Res<LocalStorage>,
    primary_input: Res<PrimaryInputDevice>,
    controllers: Res<ConnectedControllers>,
) {
    commands.spawn_scene(touch_controller_overlay(
        assets.icons.clone(),
        *theme,
        should_show_touch_controller_overlay(&storage),
        AppState::Gameplay,
        selected_mapping_has_available_device(&primary_input, &storage, &controllers),
    ));
}

pub(super) fn despawn_touch_controller_overlay(
    mut commands: Commands,
    overlays: Query<Entity, With<TouchControllerOverlayRoot>>,
) {
    for entity in &overlays {
        commands.entity(entity).try_despawn();
    }
}

fn overlay_bottom_offset(reserve_action_hints: bool) -> f32 {
    if reserve_action_hints {
        UI_SCREEN_PADDING + ACTION_HINT_ICON_SIZE + TOUCH_OVERLAY_MARGIN
    } else {
        TOUCH_OVERLAY_MARGIN
    }
}

fn dpad_cluster(icons: Handle<Image>, theme: ActiveTheme, bottom: f32) -> impl Scene {
    let left_icons = icons.clone();
    let up_icons = icons.clone();
    let right_icons = icons.clone();
    let down_icons = icons;
    let horizontal_width = TOUCH_OVERLAY_DPAD_ICON_LONG;
    let horizontal_height = TOUCH_OVERLAY_DPAD_ICON_SHORT;
    let vertical_width = TOUCH_OVERLAY_DPAD_ICON_SHORT;
    let vertical_height = TOUCH_OVERLAY_DPAD_ICON_LONG;
    let cluster_width = TOUCH_OVERLAY_DPAD_CLUSTER_SIZE;
    let cluster_height = TOUCH_OVERLAY_DPAD_CLUSTER_HEIGHT;

    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(TOUCH_OVERLAY_MARGIN),
            bottom: px(bottom),
            width: px(cluster_width),
            height: px(cluster_height),
        }
        Children [
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px((cluster_width - vertical_width) * 0.5),
                    top: px(0.0),
                }
                Children [
                    touch_button(up_icons, theme, InputAction::Dup, vertical_width, vertical_height)
                ]
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    top: px((cluster_height - horizontal_height) * 0.5),
                }
                Children [
                    touch_button(left_icons, theme, InputAction::Dleft, horizontal_width, horizontal_height)
                ]
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    right: px(0.0),
                    top: px((cluster_height - horizontal_height) * 0.5),
                }
                Children [
                    touch_button(right_icons, theme, InputAction::Dright, horizontal_width, horizontal_height)
                ]
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px((cluster_width - vertical_width) * 0.5),
                    bottom: px(0.0),
                }
                Children [
                    touch_button(down_icons, theme, InputAction::Ddown, vertical_width, vertical_height)
                ]
            ),
        ]
    }
}

fn face_button_cluster(icons: Handle<Image>, theme: ActiveTheme, bottom: f32) -> impl Scene {
    let b_icons = icons.clone();
    let a_icons = icons;
    let size = TOUCH_OVERLAY_FACE_BUTTON_SIZE;
    let gap = TOUCH_OVERLAY_FACE_BUTTON_GAP;
    let width = size * 2.0 + gap;
    let height = TOUCH_OVERLAY_DPAD_CLUSTER_HEIGHT;
    let a_bottom = (TOUCH_OVERLAY_DPAD_CLUSTER_HEIGHT - size) * 0.5;
    let b_bottom = 0.0;

    bsn! {
        Node {
            position_type: PositionType::Absolute,
            right: px(TOUCH_OVERLAY_MARGIN),
            bottom: px(bottom),
            width: px(width),
            height: px(height),
        }
        Children [
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    bottom: px(b_bottom),
                }
                Children [
                    touch_button(b_icons, theme, InputAction::B, size, size)
                ]
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    right: px(0.0),
                    bottom: px(a_bottom),
                }
                Children [
                    touch_button(a_icons, theme, InputAction::A, size, size)
                ]
            ),
        ]
    }
}

fn system_button_cluster(icons: Handle<Image>, theme: ActiveTheme, bottom: f32) -> impl Scene {
    let select_icons = icons.clone();
    let start_icons = icons;
    let width = TOUCH_OVERLAY_SYSTEM_BUTTON_WIDTH;
    let height = TOUCH_OVERLAY_SYSTEM_BUTTON_HEIGHT;
    let gap = TOUCH_OVERLAY_SYSTEM_BUTTON_GAP;
    let face_size = TOUCH_OVERLAY_DPAD_CLUSTER_HEIGHT;
    let row_width = width * 2.0 + gap;

    bsn! {
        Node {
            position_type: PositionType::Absolute,
            right: px(TOUCH_OVERLAY_MARGIN),
            bottom: px(bottom + face_size + TOUCH_OVERLAY_SYSTEM_ROW_GAP),
            width: px(row_width),
            height: px(height),
            column_gap: px(gap),
        }
        Children [
            touch_button(select_icons, theme, InputAction::Select, width, height),
            touch_button(start_icons, theme, InputAction::Start, width, height),
        ]
    }
}

fn touch_button(
    icons: Handle<Image>,
    theme: ActiveTheme,
    action: InputAction,
    width: f32,
    height: f32,
) -> impl Scene {
    let icon = touch_overlay_icon(action);
    bsn! {
        Node {
            width: px(width),
            height: px(height),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Button
        TouchControllerButton { action: {action} }
        Children [
            (
                Node {
                    width: px(icon.frame_width),
                    height: px(icon.frame_height),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                TouchControllerButtonIconFrame { action: {action} }
                IgnorePicking
                UiTransform::from_rotation(Rot2::radians(icon.rotation))
                Children [
                    (
                        Node {
                            width: px(icon.width),
                            height: px(icon.height),
                        }
                        TouchControllerButtonIcon { action: {action} }
                        IgnorePicking
                        ImageNode {
                            image: {HandleTemplate::Handle(icons)},
                            color: {theme.primary.with_alpha(INACTIVE_ICON_ALPHA)},
                            rect: {Some(icon.rect)},
                        }
                    )
                ]
            ),
        ]
    }
}

#[derive(Clone, Copy, Debug)]
struct TouchOverlayIcon {
    rect: Rect,
    width: f32,
    height: f32,
    frame_width: f32,
    frame_height: f32,
    rotation: f32,
}

fn touch_overlay_icon(action: InputAction) -> TouchOverlayIcon {
    match action {
        InputAction::Dleft => dpad_icon(0.0),
        InputAction::Dright => dpad_icon(std::f32::consts::PI),
        InputAction::Dup => dpad_icon(std::f32::consts::FRAC_PI_2),
        InputAction::Ddown => dpad_icon(-std::f32::consts::FRAC_PI_2),
        InputAction::A => TouchOverlayIcon {
            rect: icon_grid_rect(A_ICON_X, A_ICON_Y, FACE_ICON_SIZE, FACE_ICON_SIZE),
            width: TOUCH_OVERLAY_FACE_ICON_SIZE,
            height: TOUCH_OVERLAY_FACE_ICON_SIZE,
            frame_width: TOUCH_OVERLAY_FACE_ICON_SIZE,
            frame_height: TOUCH_OVERLAY_FACE_ICON_SIZE,
            rotation: 0.0,
        },
        InputAction::B => TouchOverlayIcon {
            rect: icon_grid_rect(B_ICON_X, B_ICON_Y, FACE_ICON_SIZE, FACE_ICON_SIZE),
            width: TOUCH_OVERLAY_FACE_ICON_SIZE,
            height: TOUCH_OVERLAY_FACE_ICON_SIZE,
            frame_width: TOUCH_OVERLAY_FACE_ICON_SIZE,
            frame_height: TOUCH_OVERLAY_FACE_ICON_SIZE,
            rotation: 0.0,
        },
        InputAction::Start => TouchOverlayIcon {
            rect: icon_grid_rect(
                START_ICON_X,
                START_ICON_Y,
                SYSTEM_ICON_WIDTH,
                SYSTEM_ICON_HEIGHT,
            ),
            width: TOUCH_OVERLAY_SYSTEM_ICON_WIDTH,
            height: TOUCH_OVERLAY_SYSTEM_ICON_HEIGHT,
            frame_width: TOUCH_OVERLAY_SYSTEM_ICON_WIDTH,
            frame_height: TOUCH_OVERLAY_SYSTEM_ICON_HEIGHT,
            rotation: 0.0,
        },
        InputAction::Select => TouchOverlayIcon {
            rect: icon_grid_rect(
                SELECT_ICON_X,
                SELECT_ICON_Y,
                SYSTEM_ICON_WIDTH,
                SYSTEM_ICON_HEIGHT,
            ),
            width: TOUCH_OVERLAY_SYSTEM_ICON_WIDTH,
            height: TOUCH_OVERLAY_SYSTEM_ICON_HEIGHT,
            frame_width: TOUCH_OVERLAY_SYSTEM_ICON_WIDTH,
            frame_height: TOUCH_OVERLAY_SYSTEM_ICON_HEIGHT,
            rotation: 0.0,
        },
        _ => TouchOverlayIcon {
            rect: icon_grid_rect(A_ICON_X, A_ICON_Y, FACE_ICON_SIZE, FACE_ICON_SIZE),
            width: TOUCH_OVERLAY_FACE_ICON_SIZE,
            height: TOUCH_OVERLAY_FACE_ICON_SIZE,
            frame_width: TOUCH_OVERLAY_FACE_ICON_SIZE,
            frame_height: TOUCH_OVERLAY_FACE_ICON_SIZE,
            rotation: 0.0,
        },
    }
}

fn dpad_icon(rotation: f32) -> TouchOverlayIcon {
    TouchOverlayIcon {
        rect: icon_grid_rect(DPAD_ICON_X, DPAD_ICON_Y, DPAD_ICON_WIDTH, DPAD_ICON_HEIGHT),
        width: TOUCH_OVERLAY_DPAD_ICON_LONG,
        height: TOUCH_OVERLAY_DPAD_ICON_SHORT,
        frame_width: TOUCH_OVERLAY_DPAD_ICON_LONG,
        frame_height: TOUCH_OVERLAY_DPAD_ICON_LONG,
        rotation,
    }
}

fn icon_grid_rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    let unit = ICON_TEXTURE_SIZE / ICON_GRID_UNITS;
    Rect {
        min: Vec2::new(x * unit, y * unit),
        max: Vec2::new((x + width) * unit, (y + height) * unit),
    }
}

pub(super) fn collect_touch_controller_overlay_input(
    mut presses: MessageReader<Pointer<Press>>,
    mut drag_enters: MessageReader<Pointer<DragEnter>>,
    mut drag_overs: MessageReader<Pointer<DragOver>>,
    mut drag_leaves: MessageReader<Pointer<DragLeave>>,
    mut releases: MessageReader<Pointer<Release>>,
    mut cancels: MessageReader<Pointer<Cancel>>,
    buttons: Query<&TouchControllerButton>,
    mut state: ResMut<TouchControllerOverlayInput>,
    mut mapped_events: MessageWriter<MappedInputEvent>,
) {
    for press in presses.read() {
        if press.button != PointerButton::Primary {
            continue;
        }
        let Ok(button) = buttons.get(press.entity) else {
            continue;
        };
        state.press_pointer(press.pointer_id, button.action, &mut mapped_events);
    }

    for drag_enter in drag_enters.read() {
        if drag_enter.button != PointerButton::Primary {
            continue;
        }
        let Ok(button) = buttons.get(drag_enter.entity) else {
            continue;
        };
        state.set_hover_action(drag_enter.pointer_id, button.action, &mut mapped_events);
    }

    for drag_over in drag_overs.read() {
        if drag_over.button != PointerButton::Primary {
            continue;
        }
        let Ok(button) = buttons.get(drag_over.entity) else {
            continue;
        };
        state.set_hover_action(drag_over.pointer_id, button.action, &mut mapped_events);
    }

    for drag_leave in drag_leaves.read() {
        if drag_leave.button != PointerButton::Primary {
            continue;
        }
        let Ok(button) = buttons.get(drag_leave.entity) else {
            continue;
        };
        state.release_hover_action(drag_leave.pointer_id, button.action, &mut mapped_events);
    }

    for release in releases.read() {
        if release.button == PointerButton::Primary {
            state.release_pointer(release.pointer_id, &mut mapped_events);
        }
    }

    for cancel in cancels.read() {
        state.release_pointer(cancel.pointer_id, &mut mapped_events);
    }
}

pub(super) fn release_touch_controller_overlay_input(
    mut state: ResMut<TouchControllerOverlayInput>,
    mut mapped_events: MessageWriter<MappedInputEvent>,
) {
    state.release_all(&mut mapped_events);
}

pub(super) fn update_touch_controller_overlay_visuals(
    input: Res<GameBoyInputState>,
    theme: Res<ActiveTheme>,
    mut icons: Query<(&TouchControllerButtonIcon, &mut ImageNode)>,
) {
    for (icon, mut image) in &mut icons {
        image.color = if action_pressed(&input, icon.action) {
            theme.secondary
        } else {
            theme.primary.with_alpha(INACTIVE_ICON_ALPHA)
        };
    }
}

fn action_pressed(input: &GameBoyInputState, action: InputAction) -> bool {
    match action {
        InputAction::Dleft => input.dleft,
        InputAction::Dright => input.dright,
        InputAction::Dup => input.dup,
        InputAction::Ddown => input.ddown,
        InputAction::A => input.a,
        InputAction::B => input.b,
        InputAction::Start => input.start,
        InputAction::Select => input.select,
        _ => false,
    }
}

impl TouchControllerOverlayInput {
    fn press_pointer(
        &mut self,
        pointer_id: PointerId,
        action: InputAction,
        mapped_events: &mut MessageWriter<MappedInputEvent>,
    ) {
        if self
            .pressed_pointers
            .get(&pointer_id)
            .is_some_and(|state| state.primary_action == action)
        {
            return;
        }
        self.release_pointer(pointer_id, mapped_events);

        self.pressed_pointers.insert(
            pointer_id,
            TouchPointerState {
                primary_action: action,
                hover_action: None,
            },
        );
        self.press_action(action, mapped_events);
    }

    fn set_hover_action(
        &mut self,
        pointer_id: PointerId,
        action: InputAction,
        mapped_events: &mut MessageWriter<MappedInputEvent>,
    ) {
        let Some(pointer_state) = self.pressed_pointers.get_mut(&pointer_id) else {
            return;
        };
        if pointer_state.hover_action == Some(action) {
            return;
        }

        let previous_action = pointer_state.hover_action;
        let next_action = (pointer_state.primary_action != action).then_some(action);
        pointer_state.hover_action = next_action;

        if let Some(previous_action) = previous_action {
            self.release_action(previous_action, mapped_events);
        }
        if let Some(next_action) = next_action {
            self.press_action(next_action, mapped_events);
        }
    }

    fn release_hover_action(
        &mut self,
        pointer_id: PointerId,
        action: InputAction,
        mapped_events: &mut MessageWriter<MappedInputEvent>,
    ) {
        let Some(pointer_state) = self.pressed_pointers.get_mut(&pointer_id) else {
            return;
        };
        if pointer_state.hover_action != Some(action) {
            return;
        }
        pointer_state.hover_action = None;
        self.release_action(action, mapped_events);
    }

    fn press_action(
        &mut self,
        action: InputAction,
        mapped_events: &mut MessageWriter<MappedInputEvent>,
    ) {
        let count = self.pressed_actions.entry(action).or_default();
        if *count == 0 {
            mapped_events.write(MappedInputEvent {
                action,
                state: ButtonState::Pressed,
            });
        }
        *count += 1;
    }

    fn release_pointer(
        &mut self,
        pointer_id: PointerId,
        mapped_events: &mut MessageWriter<MappedInputEvent>,
    ) {
        if let Some(pointer_state) = self.pressed_pointers.remove(&pointer_id) {
            if let Some(hover_action) = pointer_state.hover_action {
                self.release_action(hover_action, mapped_events);
            }
            self.release_action(pointer_state.primary_action, mapped_events);
        }
    }

    fn release_action(
        &mut self,
        action: InputAction,
        mapped_events: &mut MessageWriter<MappedInputEvent>,
    ) {
        let Some(count) = self.pressed_actions.get_mut(&action) else {
            return;
        };
        *count = count.saturating_sub(1);
        if *count > 0 {
            return;
        }

        self.pressed_actions.remove(&action);
        mapped_events.write(MappedInputEvent {
            action,
            state: ButtonState::Released,
        });
    }

    fn release_all(&mut self, mapped_events: &mut MessageWriter<MappedInputEvent>) {
        for action in self.pressed_actions.drain().map(|(action, _)| action) {
            mapped_events.write(MappedInputEvent {
                action,
                state: ButtonState::Released,
            });
        }
        self.pressed_pointers.clear();
    }
}

#[cfg(test)]
mod tests {
    use bevy::scene::ScenePlugin;

    use super::*;

    #[test]
    fn touch_overlay_icons_use_ui_transform_frames() {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            ScenePlugin::default(),
        ));

        app.world_mut()
            .spawn_scene(touch_controller_overlay(
                Handle::default(),
                crate::app_theme::active_theme_for_setting(crate::app_theme::MINIMAL_THEME_SETTING),
                true,
                AppState::Home,
                false,
            ))
            .expect("touch overlay scene should spawn");

        let world = app.world_mut();
        let mut icons = world.query::<(
            &TouchControllerButtonIcon,
            Option<&Transform>,
            Option<&GlobalTransform>,
        )>();

        for (_, transform, global_transform) in icons.iter(world) {
            assert!(transform.is_none());
            assert!(global_transform.is_none());
        }

        let mut frames = world.query::<(
            &TouchControllerButtonIconFrame,
            Option<&Transform>,
            Option<&GlobalTransform>,
            &UiTransform,
        )>();

        for (_, transform, global_transform, _) in frames.iter(world) {
            assert!(transform.is_none());
            assert!(global_transform.is_none());
        }
    }

    #[test]
    fn dpad_icons_point_toward_the_cluster_center() {
        assert_eq!(touch_overlay_icon(InputAction::Dleft).rotation, 0.0);
        assert_eq!(
            touch_overlay_icon(InputAction::Dright).rotation,
            std::f32::consts::PI
        );
        assert_eq!(
            touch_overlay_icon(InputAction::Dup).rotation,
            std::f32::consts::FRAC_PI_2
        );
        assert_eq!(
            touch_overlay_icon(InputAction::Ddown).rotation,
            -std::f32::consts::FRAC_PI_2
        );

        for action in [
            InputAction::Dleft,
            InputAction::Dright,
            InputAction::Dup,
            InputAction::Ddown,
        ] {
            let icon = touch_overlay_icon(action);
            assert_eq!(icon.width, TOUCH_OVERLAY_DPAD_ICON_LONG);
            assert_eq!(icon.height, TOUCH_OVERLAY_DPAD_ICON_SHORT);
            assert_eq!(icon.frame_width, TOUCH_OVERLAY_DPAD_ICON_LONG);
            assert_eq!(icon.frame_height, TOUCH_OVERLAY_DPAD_ICON_LONG);
        }
    }
}
