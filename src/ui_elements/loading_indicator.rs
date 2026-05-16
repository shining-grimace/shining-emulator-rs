use bevy::prelude::*;

use crate::app_theme::ActiveTheme;

pub const LOADING_INDICATOR_SQUARE_SIZE: f32 = 24.0;
pub const LOADING_INDICATOR_GRID_SIZE: f32 = LOADING_INDICATOR_SQUARE_SIZE * 2.0;

const LOADING_INDICATOR_STEP_SECONDS: f32 = 0.25;
const CLOCKWISE_SQUARE_ORDER: [LoadingIndicatorSquarePosition; 4] = [
    LoadingIndicatorSquarePosition {
        grid_column: 1,
        grid_row: 1,
    },
    LoadingIndicatorSquarePosition {
        grid_column: 2,
        grid_row: 1,
    },
    LoadingIndicatorSquarePosition {
        grid_column: 2,
        grid_row: 2,
    },
    LoadingIndicatorSquarePosition {
        grid_column: 1,
        grid_row: 2,
    },
];

pub struct LoadingIndicatorPlugin;

impl Plugin for LoadingIndicatorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_loading_indicator);
    }
}

#[derive(Component)]
struct LoadingIndicatorSquare {
    clockwise_position: usize,
}

#[derive(Clone, Copy)]
struct LoadingIndicatorSquarePosition {
    grid_column: i16,
    grid_row: i16,
}

pub fn spawn_loading_indicator(parent: &mut ChildSpawnerCommands, theme: &ActiveTheme) {
    let colours = loading_indicator_colours(theme);

    parent
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::px(2, LOADING_INDICATOR_SQUARE_SIZE),
                grid_template_rows: RepeatedGridTrack::px(2, LOADING_INDICATOR_SQUARE_SIZE),
                width: Val::Px(LOADING_INDICATOR_GRID_SIZE),
                height: Val::Px(LOADING_INDICATOR_GRID_SIZE),
                ..default()
            },
            Name::new("Loading indicator"),
        ))
        .with_children(|indicator| {
            for (clockwise_position, square_position) in CLOCKWISE_SQUARE_ORDER.iter().enumerate() {
                indicator.spawn((
                    Node {
                        grid_column: GridPlacement::start(square_position.grid_column),
                        grid_row: GridPlacement::start(square_position.grid_row),
                        width: Val::Px(LOADING_INDICATOR_SQUARE_SIZE),
                        height: Val::Px(LOADING_INDICATOR_SQUARE_SIZE),
                        ..default()
                    },
                    BackgroundColor(colours[clockwise_position]),
                    LoadingIndicatorSquare { clockwise_position },
                ));
            }
        });
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
