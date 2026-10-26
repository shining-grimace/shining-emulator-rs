use bevy::prelude::*;
use bevy_midi_graph::{
    MidiFileSource, MidiGraphAudioContext, MidiGraphPlugin, Sf2FileSource, WaveFileSource,
    midi::event::{
        Balance, CueData, Event, EventTarget, EventTiming, Message, MessageSender,
        MidiPlaybackState,
    },
};

use crate::app_state::AppState;
use crate::app_theme::{ActiveTheme, ActiveThemeChanged};
use crate::audio::preset_graph::{
    GAME_AUDIO_CHANNEL_NODE_IDS, apply_audio_preset_to_gameplay, apply_audio_preset_to_playback,
    default_audio_preset, load_audio_preset,
};
use crate::game_boy::{
    GameBoyAudioChannel, GameBoyAudioCommand, GameBoyAudioEvent, GameBoyCore, GameBoyEmulator,
    GameBoyUpdateSet,
};
use crate::storage::LocalStorage;

pub(crate) mod preset_graph;

pub(crate) const MENU_PROGRAM_NO: usize = 1;
pub(crate) const GAME_AUDIO_PROGRAM_NO: usize = 2;
pub(crate) const MENU_MIDI_NODE_ID: u64 = 101;
const MIDI_GRAPH_SAMPLE_RATE_HZ: u64 = 48_000;
const GAME_AUDIO_LEAD_FRAMES: u64 = 4096;
const GAME_AUDIO_MIN_LEAD_FRAMES: u64 = 2048;
const A440_HZ: f32 = 440.0;

#[derive(Debug, Default, Resource)]
struct GameBoyAudioSchedule {
    anchor_tick: Option<u64>,
    anchor_frame: u64,
    ticks_per_second: usize,
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MidiGraphPlugin)
            .init_resource::<GameBoyAudioSchedule>()
            .add_systems(OnEnter(AppState::Splash), start_menu_audio)
            .add_systems(OnExit(AppState::Splash), seek_menu_audio_to_active_theme)
            .add_systems(
                OnEnter(AppState::Gameplay),
                (prepare_gameplay_audio, stop_menu_audio).chain(),
            )
            .add_systems(OnExit(AppState::Gameplay), restart_menu_audio)
            .add_systems(
                Update,
                queue_game_boy_audio_events
                    .after(GameBoyUpdateSet::Emulation)
                    .run_if(in_state(AppState::Gameplay)),
            )
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

    if !prepare_current_audio_preset(
        &asset_server,
        &mut audio_context,
        &midi_assets,
        &sf2_assets,
        &wave_assets,
        &storage,
        "startup",
    ) {
        return;
    }
    if let Err(error) = audio_context.change_program(MENU_PROGRAM_NO) {
        eprintln!("failed to start menu audio preset: {error}");
    }
}

fn prepare_gameplay_audio(
    asset_server: Res<AssetServer>,
    mut audio_context: ResMut<MidiGraphAudioContext>,
    midi_assets: Res<Assets<MidiFileSource>>,
    sf2_assets: Res<Assets<Sf2FileSource>>,
    wave_assets: Res<Assets<WaveFileSource>>,
    storage: Res<LocalStorage>,
    mut schedule: ResMut<GameBoyAudioSchedule>,
) {
    schedule.reset();
    if !prepare_gameplay_audio_preset(
        &asset_server,
        &mut audio_context,
        &midi_assets,
        &sf2_assets,
        &wave_assets,
        &storage,
    ) {
        return;
    }
    if let Err(error) = audio_context.change_program(GAME_AUDIO_PROGRAM_NO) {
        let message = error.to_string();
        if !message.contains("already playing") {
            eprintln!("failed to start gameplay audio preset: {error}");
        }
    }
}

fn prepare_current_audio_preset(
    asset_server: &Res<AssetServer>,
    audio_context: &mut MidiGraphAudioContext,
    midi_assets: &Res<Assets<MidiFileSource>>,
    sf2_assets: &Res<Assets<Sf2FileSource>>,
    wave_assets: &Res<Assets<WaveFileSource>>,
    storage: &LocalStorage,
    description: &'static str,
) -> bool {
    let preset_path = storage
        .paths
        .audio_preset_file(storage.data.settings.audio_preset.min(9));
    let preset = match load_audio_preset(&preset_path) {
        Ok(preset) => preset,
        Err(error) => {
            eprintln!("failed to load {description} audio preset; using defaults: {error}");
            default_audio_preset()
        }
    };
    if let Err(error) = apply_audio_preset_to_playback(
        &preset,
        asset_server,
        audio_context,
        midi_assets,
        sf2_assets,
        wave_assets,
    ) {
        eprintln!("failed to prepare {description} audio preset: {error}");
        return false;
    }
    true
}

fn prepare_gameplay_audio_preset(
    asset_server: &Res<AssetServer>,
    audio_context: &mut MidiGraphAudioContext,
    midi_assets: &Res<Assets<MidiFileSource>>,
    sf2_assets: &Res<Assets<Sf2FileSource>>,
    wave_assets: &Res<Assets<WaveFileSource>>,
    storage: &LocalStorage,
) -> bool {
    let preset_path = storage
        .paths
        .audio_preset_file(storage.data.settings.audio_preset.min(9));
    let preset = match load_audio_preset(&preset_path) {
        Ok(preset) => preset,
        Err(error) => {
            eprintln!("failed to load gameplay audio preset; using defaults: {error}");
            default_audio_preset()
        }
    };
    if let Err(error) = apply_audio_preset_to_gameplay(
        &preset,
        storage.data.settings.game_audio_volume,
        asset_server,
        audio_context,
        midi_assets,
        sf2_assets,
        wave_assets,
    ) {
        eprintln!("failed to prepare gameplay audio preset: {error}");
        return false;
    }
    true
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

fn stop_menu_audio(mut audio_context: ResMut<MidiGraphAudioContext>) {
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

    queue_audio_event(
        &mut audio_context,
        EventTarget::Broadcast,
        Event::AllNotesOff,
        "clear gameplay notes",
    );
    if let Err(error) = audio_context.change_program(MENU_PROGRAM_NO) {
        let message = error.to_string();
        if !message.contains("already playing") {
            eprintln!("failed to restart menu audio preset: {error}");
            return;
        }
    }
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
    queue_audio_event(
        audio_context,
        EventTarget::SpecificNode(MENU_MIDI_NODE_ID),
        event,
        description,
    );
}

fn queue_audio_event(
    audio_context: &mut MidiGraphAudioContext,
    target: EventTarget,
    event: Event,
    description: &'static str,
) {
    let sender = audio_context.get_event_sender();
    if let Err(error) = sender.send(Message {
        target,
        event,
        timing: EventTiming::Imprecise,
    }) {
        eprintln!("failed to queue audio {description}: {error}");
    }
}

fn queue_game_boy_audio_events(
    mut audio_context: ResMut<MidiGraphAudioContext>,
    mut schedule: ResMut<GameBoyAudioSchedule>,
    mut emulators: Query<&mut GameBoyCore, With<GameBoyEmulator>>,
) {
    let sender = audio_context.get_event_sender();
    let current_audio_frame = sender.current_rendering_absolute_frame();
    for mut emulator in &mut emulators {
        let ticks_per_second = emulator.audio_unit.base_running_speed;
        let events = emulator.audio_unit.drain_pending_events();
        for event in events {
            let frame = schedule.frame_for_tick(event.tick, ticks_per_second, current_audio_frame);
            queue_game_boy_audio_event(&sender, event, frame);
        }
    }
}

fn queue_game_boy_audio_event(sender: &MessageSender, event: GameBoyAudioEvent, frame: u64) {
    match (event.channel, event.command) {
        (
            Some(channel),
            GameBoyAudioCommand::NoteOn {
                frequency_hz,
                volume,
                balance,
            },
        ) => {
            let target = game_audio_channel_target(channel);
            queue_game_audio_message(sender, target, Event::SourceBalance(balance.into()), frame);
            queue_game_audio_message(sender, target, Event::Volume(volume.clamp(0.0, 1.0)), frame);
            queue_game_audio_message(sender, target, Event::NoteOn { note: 69, vel: 1.0 }, frame);
            // Generators reset pitch on NoteOn, and equal-frame events are not stably ordered.
            queue_game_audio_message(
                sender,
                target,
                Event::PitchMultiplier(frequency_multiplier(frequency_hz)),
                frame.saturating_add(1),
            );
        }
        (Some(channel), GameBoyAudioCommand::NoteOff) => {
            queue_game_audio_message(
                sender,
                game_audio_channel_target(channel),
                Event::NoteOff { note: 69, vel: 0.0 },
                frame,
            );
        }
        (Some(channel), GameBoyAudioCommand::Frequency { frequency_hz }) => {
            queue_game_audio_message(
                sender,
                game_audio_channel_target(channel),
                Event::PitchMultiplier(frequency_multiplier(frequency_hz)),
                frame,
            );
        }
        (Some(channel), GameBoyAudioCommand::DutyCycle(duty_cycle)) => {
            queue_game_audio_message(
                sender,
                game_audio_channel_target(channel),
                Event::DutyCycle(duty_cycle),
                frame,
            );
        }
        (Some(channel), GameBoyAudioCommand::Volume(volume)) => {
            queue_game_audio_message(
                sender,
                game_audio_channel_target(channel),
                Event::Volume(volume.clamp(0.0, 1.0)),
                frame,
            );
        }
        (Some(channel), GameBoyAudioCommand::Balance(balance)) => {
            queue_game_audio_message(
                sender,
                game_audio_channel_target(channel),
                Event::SourceBalance(balance.into()),
                frame,
            );
        }
        (Some(channel), GameBoyAudioCommand::Wavetable(samples)) => {
            queue_game_audio_message(
                sender,
                game_audio_channel_target(channel),
                Event::Wavetable(samples.into()),
                frame,
            );
        }
        (_, GameBoyAudioCommand::AllNotesOff) => {
            for node_id in GAME_AUDIO_CHANNEL_NODE_IDS {
                queue_game_audio_message(
                    sender,
                    EventTarget::SpecificNode(node_id),
                    Event::AllNotesOff,
                    frame,
                );
            }
        }
        (None, _) => {}
    }
}

fn queue_game_audio_message(
    sender: &MessageSender,
    target: EventTarget,
    event: Event,
    absolute_frame: u64,
) {
    if let Err(error) = sender.send(Message {
        target,
        event,
        timing: EventTiming::AtAbsoluteFrame(absolute_frame),
    }) {
        warn!("failed to queue Game Boy audio event: {error}");
    }
}

fn game_audio_channel_target(channel: GameBoyAudioChannel) -> EventTarget {
    EventTarget::SpecificNode(GAME_AUDIO_CHANNEL_NODE_IDS[channel.index()])
}

fn frequency_multiplier(frequency_hz: f32) -> f32 {
    if frequency_hz.is_finite() && frequency_hz > 0.0 {
        frequency_hz / A440_HZ
    } else {
        1.0
    }
}

impl From<crate::game_boy::GameBoyAudioBalance> for Balance {
    fn from(value: crate::game_boy::GameBoyAudioBalance) -> Self {
        Balance::CustomLeftRight((value.left_gain, value.right_gain))
    }
}

impl GameBoyAudioSchedule {
    fn reset(&mut self) {
        self.anchor_tick = None;
        self.anchor_frame = 0;
        self.ticks_per_second = 0;
    }

    fn frame_for_tick(
        &mut self,
        tick: u64,
        ticks_per_second: usize,
        current_audio_frame: u64,
    ) -> u64 {
        let ticks_per_second = ticks_per_second.max(1);
        self.ensure_anchor(tick, ticks_per_second, current_audio_frame);

        let anchor_tick = match self.anchor_tick {
            Some(anchor_tick) => anchor_tick,
            None => tick,
        };
        let delta_ticks = tick.saturating_sub(anchor_tick);
        let delta_frames = ticks_to_audio_frames(delta_ticks, ticks_per_second);
        let mut frame = self.anchor_frame.saturating_add(delta_frames);
        let minimum_frame = current_audio_frame.saturating_add(GAME_AUDIO_MIN_LEAD_FRAMES);
        if frame < minimum_frame {
            let target_frame = current_audio_frame.saturating_add(GAME_AUDIO_LEAD_FRAMES);
            let shift = target_frame.saturating_sub(frame);
            self.anchor_frame = self.anchor_frame.saturating_add(shift);
            frame = frame.saturating_add(shift);
        }
        frame
    }

    fn ensure_anchor(&mut self, tick: u64, ticks_per_second: usize, current_audio_frame: u64) {
        let needs_reset = match self.anchor_tick {
            Some(anchor_tick) => tick < anchor_tick || self.ticks_per_second != ticks_per_second,
            None => true,
        };
        if needs_reset {
            self.anchor_tick = Some(tick);
            self.anchor_frame = current_audio_frame.saturating_add(GAME_AUDIO_LEAD_FRAMES);
            self.ticks_per_second = ticks_per_second;
        }
    }
}

fn ticks_to_audio_frames(ticks: u64, ticks_per_second: usize) -> u64 {
    let ticks_per_second = ticks_per_second.max(1) as u128;
    let frames = (u128::from(ticks) * u128::from(MIDI_GRAPH_SAMPLE_RATE_HZ)) / ticks_per_second;
    u64::try_from(frames).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_maps_game_boy_ticks_to_audio_frames() {
        let mut schedule = GameBoyAudioSchedule::default();
        let first = schedule.frame_for_tick(0, 4_194_304, 10_000);
        let second = schedule.frame_for_tick(4_194_304, 4_194_304, 10_000);

        assert_eq!(first, 10_000 + GAME_AUDIO_LEAD_FRAMES);
        assert_eq!(second, first + MIDI_GRAPH_SAMPLE_RATE_HZ);
    }

    #[test]
    fn schedule_keeps_events_ahead_of_current_audio_frame() {
        let mut schedule = GameBoyAudioSchedule::default();
        let first = schedule.frame_for_tick(0, 4_194_304, 10_000);
        let shifted = schedule.frame_for_tick(4, 4_194_304, first);

        assert!(shifted >= first + GAME_AUDIO_MIN_LEAD_FRAMES);
    }

    #[test]
    fn wavetable_events_preserve_game_boy_dac_samples() {
        let mut samples = [0.0; 32];
        samples[0] = -1.0;
        samples[1] = 1.0;

        let wavetable = Vec::from(samples);

        assert_eq!(wavetable[0], -1.0);
        assert_eq!(wavetable[1], 1.0);
    }
}
