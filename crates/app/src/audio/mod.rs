use bevy::prelude::*;
use bevy_midi_graph::MidiGraphPlugin;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MidiGraphPlugin);
    }
}
