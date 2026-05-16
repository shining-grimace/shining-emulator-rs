use bevy::prelude::*;

use crate::app_theme::ActiveTheme;

pub const LOADING_INDICATOR_SQUARE_SIZE: f32 = 24.0;
pub const LOADING_INDICATOR_GRID_SIZE: f32 = LOADING_INDICATOR_SQUARE_SIZE * 2.0;

const LOADING_INDICATOR_STEP_SECONDS: f32 = 0.25;

pub struct LoadingIndicatorPlugin;

impl Plugin for LoadingIndicatorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_loading_indicator);
    }
}

#[derive(Clone, Component, Default, FromTemplate)]
struct LoadingIndicatorSquare {
    clockwise_position: usize,
}

pub fn loading_indicator_scene(theme: ActiveTheme) -> impl Scene {
    let colours = loading_indicator_colours(&theme);

    bsn! {
        #LoadingIndicator
        Node {
            display: Display::Grid,
            grid_template_columns: {vec![RepeatedGridTrack::px::<RepeatedGridTrack>(
                2,
                LOADING_INDICATOR_SQUARE_SIZE,
            )]},
            grid_template_rows: {vec![RepeatedGridTrack::px::<RepeatedGridTrack>(
                2,
                LOADING_INDICATOR_SQUARE_SIZE,
            )]},
            width: px(LOADING_INDICATOR_GRID_SIZE),
            height: px(LOADING_INDICATOR_GRID_SIZE),
        }
        Children [
            (
                Node {
                    grid_column: GridPlacement::start(1),
                    grid_row: GridPlacement::start(1),
                    width: px(LOADING_INDICATOR_SQUARE_SIZE),
                    height: px(LOADING_INDICATOR_SQUARE_SIZE),
                }
                BackgroundColor({colours[0]})
                LoadingIndicatorSquare { clockwise_position: 0 }
            ),
            (
                Node {
                    grid_column: GridPlacement::start(2),
                    grid_row: GridPlacement::start(1),
                    width: px(LOADING_INDICATOR_SQUARE_SIZE),
                    height: px(LOADING_INDICATOR_SQUARE_SIZE),
                }
                BackgroundColor({colours[1]})
                LoadingIndicatorSquare { clockwise_position: 1 }
            ),
            (
                Node {
                    grid_column: GridPlacement::start(2),
                    grid_row: GridPlacement::start(2),
                    width: px(LOADING_INDICATOR_SQUARE_SIZE),
                    height: px(LOADING_INDICATOR_SQUARE_SIZE),
                }
                BackgroundColor({colours[2]})
                LoadingIndicatorSquare { clockwise_position: 2 }
            ),
            (
                Node {
                    grid_column: GridPlacement::start(1),
                    grid_row: GridPlacement::start(2),
                    width: px(LOADING_INDICATOR_SQUARE_SIZE),
                    height: px(LOADING_INDICATOR_SQUARE_SIZE),
                }
                BackgroundColor({colours[3]})
                LoadingIndicatorSquare { clockwise_position: 3 }
            ),
        ]
    }
}

fn update_loading_indicator(
    time: Res<Time>,
    theme: Res<ActiveTheme>,
    mut squares: Query<(&LoadingIndicatorSquare, &mut BackgroundColor)>,
) {
    let colours = loading_indicator_colours(&theme);
    let animation_step =
        (time.elapsed_secs() / LOADING_INDICATOR_STEP_SECONDS).floor() as usize % 4;

    for (square, mut background) in &mut squares {
        let colour_index =
            (square.clockwise_position + colours.len() - animation_step) % colours.len();

        background.0 = colours[colour_index];
    }
}

fn loading_indicator_colours(theme: &ActiveTheme) -> [Color; 4] {
    [Color::BLACK, theme.tertiary, theme.secondary, theme.primary]
}
