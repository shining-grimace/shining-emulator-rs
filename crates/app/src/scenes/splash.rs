use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::dimensions::{
    APP_LOGO_WIDTH, DEVELOPER_BRANDING_GAP, DEVELOPER_BRANDING_MARGIN, DEVELOPER_LOGO_SIZE,
    DEVELOPER_TEXT_SIZE,
};
pub const SPLASH_SCREEN_SECONDS: f32 = 2.0;

pub struct SplashScenePlugin;

impl Plugin for SplashScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Splash), spawn_splash_scene)
            .add_systems(
                Update,
                transition_to_home.run_if(in_state(AppState::Splash)),
            );
    }
}

fn spawn_splash_scene(mut commands: Commands, assets: Res<AppAssets>) {
    commands.insert_resource(SplashScreenTimer(Timer::from_seconds(
        SPLASH_SCREEN_SECONDS,
        TimerMode::Once,
    )));
    commands.spawn_scene(splash_scene(&assets));
}

#[derive(Resource)]
struct SplashScreenTimer(Timer);

fn transition_to_home(
    time: Res<Time>,
    mut timer: ResMut<SplashScreenTimer>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        next_state.set(AppState::Home);
    }
}

fn splash_scene(assets: &AppAssets) -> impl Scene {
    let app_logo = assets.shining_emulator_logo.clone();
    let developer_logo = assets.shining_grimace_logo.clone();
    let font = assets.ubuntu_mono_font.clone();

    bsn! {
        #SplashScene
        DespawnOnExit::<AppState>(AppState::Splash)
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Children [
            (
                Node {
                    width: px(APP_LOGO_WIDTH),
                }
                ImageNode {
                    image: {app_logo},
                }
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    right: px(DEVELOPER_BRANDING_MARGIN),
                    bottom: px(DEVELOPER_BRANDING_MARGIN),
                    align_items: AlignItems::Center,
                    column_gap: px(DEVELOPER_BRANDING_GAP),
                }
                Children [
                    (
                        Node {
                            width: px(DEVELOPER_LOGO_SIZE),
                            height: px(DEVELOPER_LOGO_SIZE),
                        }
                        ImageNode {
                            image: {developer_logo},
                        }
                    ),
                    (
                        Text("Shining Grimace")
                        TextFont {
                            font: FontSourceTemplate::Handle(font),
                            font_size: px(DEVELOPER_TEXT_SIZE),
                        }
                        TextColor(Color::WHITE)
                    ),
                ]
            ),
        ]
    }
}
