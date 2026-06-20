use bevy::prelude::*;
use bevy_midi_graph::{
    MidiFileSource, MidiGraphAudioContext, MidiGraphPlugin, Sf2FileSource, WaveFileSource,
    midi::event::{CueData, Event, EventTarget, EventTiming, Message, MidiPlaybackState},
};

use crate::app_state::AppState;
use crate::app_theme::{ActiveTheme, ActiveThemeChanged};
use crate::audio::preset_graph::{
    apply_audio_preset_to_playback, default_audio_preset, load_audio_preset,
};
use crate::storage::LocalStorage;

pub(crate) mod preset_graph;

pub(crate) const MENU_PROGRAM_NO: usize = 1;
pub(crate) const MENU_MIDI_NODE_ID: u64 = 101;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MidiGraphPlugin)
            .add_systems(OnEnter(AppState::Splash), start_menu_audio)
            .add_systems(OnExit(AppState::Splash), seek_menu_audio_to_active_theme)
            .add_systems(OnEnter(AppState::Gameplay), stop_menu_audio)
            .add_systems(OnExit(AppState::Gameplay), restart_menu_audio)
            .add_observer(seek_menu_audio_on_theme_change);
    }
}

fn start_menu_audio(
    asset_server: Res<AssetServer>,
    mut audio_context: ResMut<MidiGraphAudioContext>,
    midi_assets: Res<Assets<MidiFileSource>>,
    sf2_assets: Res<Assets<Sf2FileSource>>,
    wave_assets: Res<Assets<WaveFileSource>>,
    storage: Res<LocalStorage>,
    theme: Res<ActiveTheme>,
) {
    if theme.audio_anchor.is_none() {
        return;
    }

    let preset_path = storage
        .paths
        .audio_preset_file(storage.data.settings.audio_preset.min(9));
    let preset = match load_audio_preset(&preset_path) {
        Ok(preset) => preset,
        Err(error) => {
            eprintln!("failed to load startup audio preset; using defaults: {error}");
            default_audio_preset()
        }
    };
    if let Err(error) = apply_audio_preset_to_playback(
        &preset,
        &asset_server,
        &mut audio_context,
        &midi_assets,
        &sf2_assets,
        &wave_assets,
    ) {
        eprintln!("failed to prepare menu audio preset: {error}");
        return;
    }
    if let Err(error) = audio_context.change_program(MENU_PROGRAM_NO) {
        eprintln!("failed to start menu audio preset: {error}");
    }
}

fn seek_menu_audio_to_active_theme(
    mut audio_context: ResMut<MidiGraphAudioContext>,
    theme: Res<ActiveTheme>,
) {
    seek_menu_audio_to_theme_anchor(&mut audio_context, &theme, "theme seek");
}

fn seek_menu_audio_on_theme_change(
    _theme_changed: On<ActiveThemeChanged>,
    mut audio_context: ResMut<MidiGraphAudioContext>,
    theme: Res<ActiveTheme>,
) {
    seek_menu_audio_to_theme_anchor(&mut audio_context, &theme, "theme change seek");
}

fn seek_menu_audio_to_theme_anchor(
    audio_context: &mut MidiGraphAudioContext,
    theme: &ActiveTheme,
    description: &'static str,
) {
    let Some(anchor) = theme.audio_anchor else {
        return;
    };

    queue_menu_audio_event(
        audio_context,
        Event::CueData(CueData::SeekWhenIdeal(anchor)),
        description,
    );
}

fn stop_menu_audio(mut audio_context: ResMut<MidiGraphAudioContext>, theme: Res<ActiveTheme>) {
    if theme.audio_anchor.is_none() {
        return;
    }

    queue_menu_audio_event(
        &mut audio_context,
        Event::MidiPlayback(MidiPlaybackState::Paused),
        "pause playback",
    );
    queue_menu_audio_event(&mut audio_context, Event::AllNotesOff, "stop notes");
}

fn restart_menu_audio(mut audio_context: ResMut<MidiGraphAudioContext>, theme: Res<ActiveTheme>) {
    let Some(anchor) = theme.audio_anchor else {
        return;
    };

    queue_menu_audio_event(
        &mut audio_context,
        Event::MidiPlayback(MidiPlaybackState::Playing),
        "restart playback",
    );
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
    if let Err(error) = sender.send(Message {
        target: EventTarget::SpecificNode(MENU_MIDI_NODE_ID),
        data: event,
        timing: EventTiming::Imprecise,
    }) {
        eprintln!("failed to queue menu audio {description}: {error}");
    }
}
