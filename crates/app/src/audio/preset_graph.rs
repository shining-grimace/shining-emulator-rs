use std::fs;
use std::path::Path;

use bevy::prelude::*;
use bevy_midi_graph::{
    GraphAssetLoader, MidiFileSource, MidiGraphAudioContext, Sf2FileSource, WaveFileSource,
    midi::event::{Event, EventTarget, Message},
    midi::node::ChildConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::audio::{MENU_MIDI_NODE_ID, MENU_PROGRAM_NO};

const MENU_MIDI_PATH: &str = "audio/audio.mid";
const MENU_MIDI_TRACK_INDEX: usize = 0;

const OSCILLATOR_SQUARE: &str = "Square Wave";
const OSCILLATOR_TRIANGLE: &str = "Triangle Wave";
const OSCILLATOR_SAWTOOTH: &str = "Sawtooth Wave";
const OSCILLATOR_LFSR_NOISE: &str = "LFSR Noise";
const OSCILLATOR_BUILT_IN_SAMPLER: &str = "Built-in Sampler";
const OSCILLATOR_CUSTOM_SAMPLER: &str = "Custom Sampler";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioPreset {
    #[serde(default = "default_audio_channels")]
    pub(crate) channels: Vec<AudioChannelPreset>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioChannelPreset {
    pub(crate) oscillator: String,
    pub(crate) built_in_sample: String,
    pub(crate) custom_sample_path: String,
    pub(crate) modulation_a: String,
    pub(crate) modulation_b: Option<String>,
}

pub(crate) fn default_audio_preset() -> AudioPreset {
    AudioPreset {
        channels: default_audio_channels(),
    }
}

fn default_audio_channels() -> Vec<AudioChannelPreset> {
    vec![
        AudioChannelPreset {
            oscillator: "Square Wave".to_string(),
            built_in_sample: "Piano".to_string(),
            custom_sample_path: String::new(),
            modulation_a: "Duty Cycle".to_string(),
            modulation_b: Some("Pitch Envelope".to_string()),
        },
        AudioChannelPreset {
            oscillator: "Square Wave".to_string(),
            built_in_sample: "Piano".to_string(),
            custom_sample_path: String::new(),
            modulation_a: "Duty Cycle".to_string(),
            modulation_b: None,
        },
        AudioChannelPreset {
            oscillator: "Wave Table".to_string(),
            built_in_sample: "Piano".to_string(),
            custom_sample_path: String::new(),
            modulation_a: "None".to_string(),
            modulation_b: None,
        },
        AudioChannelPreset {
            oscillator: "LFSR Noise".to_string(),
            built_in_sample: "Piano".to_string(),
            custom_sample_path: String::new(),
            modulation_a: "None".to_string(),
            modulation_b: None,
        },
    ]
}

pub(crate) fn load_audio_preset(path: &Path) -> Result<AudioPreset, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

pub(crate) fn fallback_audio_channel(index: usize) -> AudioChannelPreset {
    default_audio_channels()
        .get(index)
        .cloned()
        .unwrap_or_else(|| AudioChannelPreset {
            oscillator: "Square Wave".to_string(),
            built_in_sample: "Piano".to_string(),
            custom_sample_path: String::new(),
            modulation_a: "Duty Cycle".to_string(),
            modulation_b: None,
        })
}

pub(crate) fn apply_audio_preset_to_playback(
    preset: &AudioPreset,
    asset_server: &Res<AssetServer>,
    audio_context: &mut MidiGraphAudioContext,
    midi_assets: &Res<Assets<MidiFileSource>>,
    sf2_assets: &Res<Assets<Sf2FileSource>>,
    wave_assets: &Res<Assets<WaveFileSource>>,
) -> Result<(), String> {
    let graph_json = midi_graph_json_from_audio_preset(preset);
    let config: ChildConfig =
        serde_json::from_value(graph_json).map_err(|error| error.to_string())?;
    let mut loader = GraphAssetLoader::new(asset_server, midi_assets, sf2_assets, wave_assets);

    let state_snapshot = match audio_context.capture_node_state(MENU_MIDI_NODE_ID) {
        Some(Ok(snapshot)) => Some(snapshot),
        Some(Err(error)) => return Err(error.to_string()),
        None => None,
    };

    audio_context
        .store_new_program(MENU_PROGRAM_NO, &config, &mut loader)
        .map_err(|error| error.to_string())?;

    if let Some(snapshot) = state_snapshot {
        let sender = audio_context.get_event_sender();
        sender
            .send(Message {
                target: EventTarget::SpecificNode(MENU_MIDI_NODE_ID),
                data: Event::StateSnapshot(snapshot),
            })
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn midi_graph_json_from_audio_preset(preset: &AudioPreset) -> Value {
    json!({
        "type": "Midi",
        "node_id": MENU_MIDI_NODE_ID,
        "source": {
            "FilePath": {
                "path": MENU_MIDI_PATH,
                "track_index": MENU_MIDI_TRACK_INDEX
            }
        },
        "channels": {
            "0": midi_channel_source_json(preset, 0),
            "1": midi_channel_source_json(preset, 1),
            "2": midi_channel_source_json(preset, 2),
            "3": midi_channel_source_json(preset, 3)
        }
    })
}

fn midi_channel_source_json(preset: &AudioPreset, index: usize) -> Value {
    let channel = preset_channel(preset, index);
    let source = oscillator_source_json(index, &channel);

    if matches!(
        channel.oscillator.as_str(),
        OSCILLATOR_SQUARE | OSCILLATOR_BUILT_IN_SAMPLER | OSCILLATOR_CUSTOM_SAMPLER
    ) {
        json!({
            "type": "AdsrEnvelope",
            "attack_time": 0.01,
            "decay_time": 0.08,
            "sustain_multiplier": 0.4,
            "release_time": 0.08,
            "source": source
        })
    } else {
        source
    }
}

fn preset_channel(preset: &AudioPreset, index: usize) -> AudioChannelPreset {
    preset
        .channels
        .get(index)
        .cloned()
        .unwrap_or_else(|| fallback_audio_channel(index))
}

fn oscillator_source_json(channel_index: usize, preset: &AudioChannelPreset) -> Value {
    match preset.oscillator.as_str() {
        OSCILLATOR_TRIANGLE => json!({
            "type": "TriangleWave",
            "amplitude": channel_amplitude(channel_index)
        }),
        OSCILLATOR_SAWTOOTH => json!({
            "type": "SawtoothWave",
            "amplitude": channel_amplitude(channel_index)
        }),
        OSCILLATOR_LFSR_NOISE => json!({
            "type": "LfsrNoise",
            "amplitude": channel_amplitude(channel_index) * 0.6,
            "inside_feedback": false
        }),
        OSCILLATOR_BUILT_IN_SAMPLER => built_in_sample_source_json(preset, channel_index),
        OSCILLATOR_CUSTOM_SAMPLER => custom_sample_source_json(preset, channel_index),
        _ => json!({
            "type": "SquareWave",
            "amplitude": channel_amplitude(channel_index),
            "duty_cycle": square_duty_cycle(preset)
        }),
    }
}

fn built_in_sample_source_json(preset: &AudioChannelPreset, channel_index: usize) -> Value {
    match preset.built_in_sample.as_str() {
        "Guitar" => json!({
            "type": "SawtoothWave",
            "amplitude": channel_amplitude(channel_index)
        }),
        "Bass" => json!({
            "type": "TriangleWave",
            "amplitude": channel_amplitude(channel_index) * 0.8
        }),
        "Bell" => json!({
            "type": "SquareWave",
            "amplitude": channel_amplitude(channel_index) * 0.7,
            "duty_cycle": 0.125
        }),
        _ => json!({
            "type": "SquareWave",
            "amplitude": channel_amplitude(channel_index),
            "duty_cycle": 0.5
        }),
    }
}

fn custom_sample_source_json(preset: &AudioChannelPreset, channel_index: usize) -> Value {
    // The current MIDI Graph Bevy loader resolves samples through Bevy's asset server, while this
    // screen stores native file picker paths. Keep applying the preset instead of failing until a
    // local-file asset source is available.
    built_in_sample_source_json(preset, channel_index)
}

fn channel_amplitude(index: usize) -> f32 {
    match index {
        0 => 0.24,
        1 => 0.2,
        2 => 0.28,
        _ => 0.12,
    }
}

fn square_duty_cycle(preset: &AudioChannelPreset) -> f32 {
    if preset.modulation_a == "Duty Cycle" || preset.modulation_b.as_deref() == Some("Duty Cycle") {
        0.25
    } else {
        0.5
    }
}
