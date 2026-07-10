use bevy::color::Alpha;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::app_state::AppState;
use crate::visual_effects::{SETTINGS_TRANSITION_FADE_SECONDS, SETTINGS_TRANSITION_HOLD_SECONDS};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransitionPhase {
    Idle,
    FadeOut,
    Hold,
    FadeIn,
    Completing,
}

#[derive(Resource, Debug)]
pub(crate) struct SettingsTransition {
    phase: TransitionPhase,
    elapsed: f32,
    destination: Option<AppState>,
    foreground_opacity: f32,
}

impl Default for SettingsTransition {
    fn default() -> Self {
        Self {
            phase: TransitionPhase::Idle,
            elapsed: 0.0,
            destination: None,
            foreground_opacity: 1.0,
        }
    }
}

impl SettingsTransition {
    pub(crate) fn request(&mut self, current: AppState, destination: AppState) -> bool {
        if self.phase != TransitionPhase::Idle {
            return true;
        }
        if !is_settings_screen(current)
            || !is_settings_screen(destination)
            || current == destination
        {
            return false;
        }
        self.phase = TransitionPhase::FadeOut;
        self.elapsed = 0.0;
        self.destination = Some(destination);
        true
    }

    pub(crate) fn foreground_is_opaque(&self) -> bool {
        matches!(
            self.phase,
            TransitionPhase::Idle | TransitionPhase::Completing
        ) && self.foreground_opacity >= 1.0
    }

    fn is_idle(&self) -> bool {
        self.phase == TransitionPhase::Idle
    }

    pub(crate) fn circuit_screen(&self) -> Option<AppState> {
        self.destination
            .filter(|_| self.phase != TransitionPhase::Idle)
    }

    pub(crate) fn circuit_progress(&self) -> Option<f32> {
        let elapsed = match self.phase {
            TransitionPhase::Idle => return None,
            TransitionPhase::FadeOut => self.elapsed,
            TransitionPhase::Hold => SETTINGS_TRANSITION_FADE_SECONDS + self.elapsed,
            TransitionPhase::FadeIn => {
                SETTINGS_TRANSITION_FADE_SECONDS + SETTINGS_TRANSITION_HOLD_SECONDS + self.elapsed
            }
            TransitionPhase::Completing => crate::visual_effects::SETTINGS_TRANSITION_SECONDS,
        };
        Some((elapsed / crate::visual_effects::SETTINGS_TRANSITION_SECONDS).clamp(0.0, 1.0))
    }
}

pub(crate) fn request_or_set(
    transition: &mut SettingsTransition,
    next_state: &mut NextState<AppState>,
    current: AppState,
    destination: AppState,
) {
    if !transition.request(current, destination) {
        next_state.set(destination);
    }
}

#[derive(SystemParam)]
pub(crate) struct SettingsNavigation<'w> {
    state: Res<'w, State<AppState>>,
    next_state: ResMut<'w, NextState<AppState>>,
    transition: ResMut<'w, SettingsTransition>,
}

impl SettingsNavigation<'_> {
    pub(crate) fn current(&self) -> AppState {
        *self.state.get()
    }

    pub(crate) fn request(&mut self, destination: AppState) {
        let current = *self.state.get();
        request_or_set(
            &mut self.transition,
            &mut self.next_state,
            current,
            destination,
        );
    }
}

pub(crate) struct SettingsTransitionPlugin;

#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SettingsTransitionTimeline;

impl Plugin for SettingsTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SettingsTransition>()
            .add_systems(
                Update,
                advance_transition.in_set(SettingsTransitionTimeline),
            )
            .add_systems(PostUpdate, apply_foreground_opacity);
    }
}

fn advance_transition(
    time: Res<Time>,
    mut transition: ResMut<SettingsTransition>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if transition.phase == TransitionPhase::Idle {
        return;
    }
    transition.elapsed += time.delta_secs();
    match transition.phase {
        TransitionPhase::FadeOut => {
            transition.foreground_opacity =
                (1.0 - transition.elapsed / SETTINGS_TRANSITION_FADE_SECONDS).clamp(0.0, 1.0);
            if transition.elapsed >= SETTINGS_TRANSITION_FADE_SECONDS {
                let overflow = transition.elapsed - SETTINGS_TRANSITION_FADE_SECONDS;
                transition.phase = TransitionPhase::Hold;
                transition.elapsed = overflow;
                transition.foreground_opacity = 0.0;
                if let Some(destination) = transition.destination {
                    next_state.set(destination);
                }
            }
        }
        TransitionPhase::Hold => {
            transition.foreground_opacity = 0.0;
            if transition.elapsed >= SETTINGS_TRANSITION_HOLD_SECONDS {
                let overflow = transition.elapsed - SETTINGS_TRANSITION_HOLD_SECONDS;
                transition.phase = TransitionPhase::FadeIn;
                transition.elapsed = overflow;
                transition.foreground_opacity =
                    (overflow / SETTINGS_TRANSITION_FADE_SECONDS).clamp(0.0, 1.0);
            }
        }
        TransitionPhase::FadeIn => {
            transition.foreground_opacity =
                (transition.elapsed / SETTINGS_TRANSITION_FADE_SECONDS).clamp(0.0, 1.0);
            if transition.elapsed >= SETTINGS_TRANSITION_FADE_SECONDS {
                transition.phase = TransitionPhase::Completing;
                transition.elapsed = 0.0;
                transition.foreground_opacity = 1.0;
            }
        }
        TransitionPhase::Completing => {
            transition.phase = TransitionPhase::Idle;
            transition.destination = None;
        }
        TransitionPhase::Idle => {}
    }
}

#[derive(Component, Default)]
struct ForegroundBaseAlpha {
    text: Option<f32>,
    background: Option<f32>,
    border: Option<[f32; 4]>,
    image: Option<f32>,
}

#[allow(clippy::type_complexity)]
fn apply_foreground_opacity(
    mut commands: Commands,
    transition: Res<SettingsTransition>,
    mut colours: Query<
        (
            Entity,
            Option<&mut TextColor>,
            Option<&mut BackgroundColor>,
            Option<&mut BorderColor>,
            Option<&mut ImageNode>,
            Option<&ForegroundBaseAlpha>,
        ),
        With<Node>,
    >,
) {
    let opacity = transition.foreground_opacity;
    for (entity, text, background, border, image, stored) in &mut colours {
        if transition.is_idle() && stored.is_none() {
            continue;
        }
        let mut text = text;
        let mut background = background;
        let mut border = border;
        let mut image = image;
        let base = stored.map_or_else(
            || ForegroundBaseAlpha {
                text: text.as_ref().map(|colour| colour.0.alpha()),
                background: background.as_ref().map(|colour| colour.0.alpha()),
                border: border.as_ref().map(|colour| {
                    [
                        colour.top.alpha(),
                        colour.right.alpha(),
                        colour.bottom.alpha(),
                        colour.left.alpha(),
                    ]
                }),
                image: image.as_ref().map(|node| node.color.alpha()),
            },
            |stored| ForegroundBaseAlpha {
                text: stored.text,
                background: stored.background,
                border: stored.border,
                image: stored.image,
            },
        );
        if stored.is_none() {
            commands
                .entity(entity)
                .insert(ForegroundBaseAlpha { ..base });
        }
        if let (Some(colour), Some(alpha)) = (&mut text, base.text) {
            colour.0.set_alpha(alpha * opacity);
        }
        if let (Some(colour), Some(alpha)) = (&mut background, base.background) {
            colour.0.set_alpha(alpha * opacity);
        }
        if let (Some(colour), Some(alpha)) = (&mut border, base.border) {
            colour.top.set_alpha(alpha[0] * opacity);
            colour.right.set_alpha(alpha[1] * opacity);
            colour.bottom.set_alpha(alpha[2] * opacity);
            colour.left.set_alpha(alpha[3] * opacity);
        }
        if let (Some(node), Some(alpha)) = (&mut image, base.image) {
            node.color.set_alpha(alpha * opacity);
        }
        if transition.is_idle() && stored.is_some() {
            commands.entity(entity).remove::<ForegroundBaseAlpha>();
        }
    }
}

fn is_settings_screen(state: AppState) -> bool {
    matches!(
        state,
        AppState::Home
            | AppState::Settings
            | AppState::InputMapping
            | AppState::RomProvider
            | AppState::AudioSettings
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_navigation_starts_a_foreground_transition() {
        let mut transition = SettingsTransition::default();

        assert!(transition.request(AppState::Settings, AppState::InputMapping));
        assert_eq!(transition.phase, TransitionPhase::FadeOut);
        assert_eq!(transition.destination, Some(AppState::InputMapping));
    }

    #[test]
    fn navigation_during_a_transition_is_consumed_without_retargeting() {
        let mut transition = SettingsTransition::default();
        transition.request(AppState::Settings, AppState::InputMapping);

        assert!(transition.request(AppState::Settings, AppState::AudioSettings));
        assert_eq!(transition.destination, Some(AppState::InputMapping));
    }

    #[test]
    fn navigation_from_home_to_settings_starts_a_transition() {
        let mut transition = SettingsTransition::default();

        assert!(transition.request(AppState::Home, AppState::Settings));
        assert_eq!(transition.phase, TransitionPhase::FadeOut);
    }
}
