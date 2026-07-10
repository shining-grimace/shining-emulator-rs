pub const ACTIVE_SCREEN_RECT_ANIMATION_SECONDS: f32 = 0.55;

/// Time spent fading the foreground out or back in between settings screens.
pub const SETTINGS_TRANSITION_FADE_SECONDS: f32 = 0.25;
/// Time spent with the foreground hidden between settings screens.
pub const SETTINGS_TRANSITION_HOLD_SECONDS: f32 = 0.25;

pub const SETTINGS_TRANSITION_SECONDS: f32 =
    SETTINGS_TRANSITION_FADE_SECONDS * 2.0 + SETTINGS_TRANSITION_HOLD_SECONDS;
