use bevy::prelude::*;
use bevy_midi_graph::{
    MidiGraphAudioContext, MidiGraphPlugin,
    midi::event::{CueData, Event, EventTarget, Message},
};

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;

const MENU_PROGRAM_NO: usize = 1;
const MENU_MIDI_NODE_ID: u64 = 101;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MidiGraphPlugin)
            .add_systems(OnEnter(AppState::Splash), start_menu_audio)
            .add_systems(OnExit(AppState::Splash), seek_menu_audio_to_active_theme)
            .add_systems(OnEnter(AppState::Gameplay), stop_menu_audio)
            .add_systems(OnExit(AppState::Gameplay), restart_menu_audio);
    }
}

fn start_menu_audio(
    mut commands: Commands,
    mut audio_context: ResMut<MidiGraphAudioContext>,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
) {
    if theme.audio_anchor.is_none() {
        return;
    }

    audio_context.start_new_program(
        &mut commands,
        MENU_PROGRAM_NO,
        assets.default_audio_graph.clone(),
    );
}

fn seek_menu_audio_to_active_theme(
    mut audio_context: ResMut<MidiGraphAudioContext>,
    theme: Res<ActiveTheme>,
) {
    let Some(anchor) = theme.audio_anchor else {
        return;
    };

    queue_menu_audio_event(
        &mut audio_context,
        Event::CueData(CueData::SeekWhenIdeal(anchor)),
        "theme seek",
    );
}

fn stop_menu_audio(mut audio_context: ResMut<MidiGraphAudioContext>, theme: Res<ActiveTheme>) {
    if theme.audio_anchor.is_none() {
        return;
    }

    queue_menu_audio_event(&mut audio_context, Event::Volume(0.0), "stop volume");
    queue_menu_audio_event(
        &mut audio_context,
        Event::NoteOff { note: 0, vel: 1.0 },
        "stop notes",
    );
}

fn restart_menu_audio(mut audio_context: ResMut<MidiGraphAudioContext>, theme: Res<ActiveTheme>) {
    let Some(anchor) = theme.audio_anchor else {
        return;
    };

    queue_menu_audio_event(&mut audio_context, Event::Volume(1.0), "restart volume");
    queue_menu_audio_event(
        &mut audio_context,
        Event::CueData(CueData::SeekWhenIdeal(anchor)),
        "restart seek",
    );
}

fn queue_menu_audio_event(
    audio_context: &mut MidiGraphAudioContext,
    event: Event,
    description: &'static str,
) {
    let sender = audio_context.get_event_sender();
    if let Err(error) = sender.try_send(Message {
        target: EventTarget::SpecificNode(MENU_MIDI_NODE_ID),
        data: event,
    }) {
        eprintln!("failed to queue menu audio {description}: {error}");
    }
}
