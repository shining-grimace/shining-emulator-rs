mod components;
pub(crate) mod constants;
mod effects;
mod systems;
pub(crate) mod utils;

use bevy::prelude::*;

use crate::circuit_board::effects::CircuitBoardDisplay;
use crate::circuit_board::systems::{
    animate_circuit_board, spawn_circuit_board, update_circuit_board_target, update_circuit_nodes,
    update_circuit_schematic,
};

pub struct CircuitBoardPlugin;

impl Plugin for CircuitBoardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CircuitBoardDisplay>().add_systems(
            Update,
            (
                update_circuit_board_target,
                spawn_circuit_board,
                animate_circuit_board,
                update_circuit_nodes,
                update_circuit_schematic,
            )
                .chain(),
        );
    }
}
