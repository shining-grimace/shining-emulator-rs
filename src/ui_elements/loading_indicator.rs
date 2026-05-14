use bevy::prelude::*;

pub const LOADING_INDICATOR_SQUARE_SIZE: f32 = 24.0;
pub const LOADING_INDICATOR_GRID_SIZE: f32 = LOADING_INDICATOR_SQUARE_SIZE * 2.0;

const LOADING_INDICATOR_STEP_SECONDS: f32 = 0.25;
const MINIMAL_THEME_PRIMARY: Color = Color::srgb_u8(0xbc, 0x31, 0xff);
const MINIMAL_THEME_SECONDARY: Color = Color::srgb_u8(0xe4, 0xbd, 0xa3);
const MINIMAL_THEME_TERTIARY: Color = Color::srgb_u8(0x8c, 0xb9, 0xca);
const LOADING_INDICATOR_COLOURS: [Color; 4] = [
    Color::BLACK,
    MINIMAL_THEME_TERTIARY,
    MINIMAL_THEME_SECONDARY,
    MINIMAL_THEME_PRIMARY,
];
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

pub fn spawn_loading_indicator(parent: &mut ChildSpawnerCommands) {
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
                    BackgroundColor(LOADING_INDICATOR_COLOURS[clockwise_position]),
                    LoadingIndicatorSquare { clockwise_position },
                ));
            }
        });
}

fn update_loading_indicator(
    time: Res<Time>,
    mut squares: Query<(&LoadingIndicatorSquare, &mut BackgroundColor)>,
) {
    let animation_step =
        (time.elapsed_secs() / LOADING_INDICATOR_STEP_SECONDS).floor() as usize % 4;

    for (square, mut background) in &mut squares {
        let colour_index = (square.clockwise_position + LOADING_INDICATOR_COLOURS.len()
            - animation_step)
            % LOADING_INDICATOR_COLOURS.len();

        background.0 = LOADING_INDICATOR_COLOURS[colour_index];
    }
}
