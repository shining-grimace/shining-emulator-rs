use super::events::{GameBoyAudioBalance, GameBoyAudioChannel, GameBoyAudioCommand};
use super::registers::{
    ENVELOPE_CLOCK_HZ, LENGTH_CLOCK_HZ, LENGTH_ENABLE_BIT, NR10_IO_INDEX, NR11_IO_INDEX,
    NR12_IO_INDEX, NR13_IO_INDEX, NR14_IO_INDEX, NR50_IO_INDEX, NR51_IO_INDEX, NR52_IO_INDEX,
    SWEEP_CLOCK_HZ, TRIGGER_BIT, WAVE_RAM_END_INDEX, WAVE_RAM_START_INDEX,
};
use super::*;
use crate::game_boy::emulator::constants::GB_CLOCK_HZ;

fn default_io() -> [u8; 256] {
    let mut io = [0; 256];
    io[NR50_IO_INDEX] = 0x77;
    io[NR51_IO_INDEX] = 0xff;
    io
}

#[test]
fn pulse_trigger_queues_note_with_register_frequency_and_volume() {
    let mut unit = AudioUnitState::default();
    let mut io = default_io();
    io[NR12_IO_INDEX] = 0xf0;
    io[NR13_IO_INDEX] = 0x00;
    io[NR14_IO_INDEX] = TRIGGER_BIT;

    unit.write_register(NR14_IO_INDEX, io[NR14_IO_INDEX], &io);
    let events = unit.drain_pending_events();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].channel, Some(GameBoyAudioChannel::Pulse1));
    assert_eq!(events[0].tick, 0);
    assert_eq!(events[0].command, GameBoyAudioCommand::DutyCycle(0.125));
    assert_eq!(events[1].channel, Some(GameBoyAudioChannel::Pulse1));
    assert_eq!(events[1].tick, 0);
    assert_eq!(
        events[1].command,
        GameBoyAudioCommand::NoteOn {
            frequency_hz: 64.0,
            volume: 1.0,
            balance: GameBoyAudioBalance {
                left_gain: 1.0,
                right_gain: 1.0,
            },
        }
    );
}

#[test]
fn length_counter_queues_note_off_at_precise_tick() {
    let mut unit = AudioUnitState::default();
    let mut io = default_io();
    let length_ticks = GB_CLOCK_HZ as u64 / LENGTH_CLOCK_HZ;
    io[super::registers::NR11_IO_INDEX] = 63;
    io[NR12_IO_INDEX] = 0xf0;
    io[NR14_IO_INDEX] = TRIGGER_BIT | LENGTH_ENABLE_BIT;

    unit.write_register(NR14_IO_INDEX, io[NR14_IO_INDEX], &io);
    unit.drain_pending_events();
    unit.advance_ticks((length_ticks - 4) as i32, &mut io);
    assert!(unit.drain_pending_events().is_empty());
    unit.advance_ticks(4, &mut io);

    let events = unit.drain_pending_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].tick, length_ticks);
    assert_eq!(events[0].command, GameBoyAudioCommand::NoteOff);
}

#[test]
fn envelope_generates_volume_steps() {
    let mut unit = AudioUnitState::default();
    let mut io = default_io();
    let envelope_ticks = GB_CLOCK_HZ as u64 / ENVELOPE_CLOCK_HZ * 2;
    io[NR12_IO_INDEX] = 0x82;
    io[NR14_IO_INDEX] = TRIGGER_BIT;

    unit.write_register(NR14_IO_INDEX, io[NR14_IO_INDEX], &io);
    unit.drain_pending_events();
    unit.advance_ticks(envelope_ticks as i32, &mut io);

    let events = unit.drain_pending_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].tick, envelope_ticks);
    assert_eq!(events[0].command, GameBoyAudioCommand::Volume(7.0 / 15.0));
}

#[test]
fn wave_ram_write_decodes_both_nibbles_as_wavetable_samples() {
    let mut unit = AudioUnitState::default();
    let mut io = default_io();
    for index in WAVE_RAM_START_INDEX..=WAVE_RAM_END_INDEX {
        io[index] = 0xf0;
    }

    unit.write_register(WAVE_RAM_START_INDEX, io[WAVE_RAM_START_INDEX], &io);

    let events = unit.drain_pending_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].channel, Some(GameBoyAudioChannel::Wave));
    let mut expected = [0.0; 32];
    for chunk in expected.chunks_mut(2) {
        chunk[0] = 1.0;
        chunk[1] = -1.0;
    }
    assert_eq!(events[0].command, GameBoyAudioCommand::Wavetable(expected));
}

#[test]
fn mixer_uses_independent_nonzero_output_gains() {
    let mut unit = AudioUnitState::default();
    let mut io = default_io();
    io[NR50_IO_INDEX] = 0x03;
    io[NR51_IO_INDEX] = 0x01;
    io[NR12_IO_INDEX] = 0xf0;
    io[NR14_IO_INDEX] = TRIGGER_BIT;

    unit.write_register(NR14_IO_INDEX, io[NR14_IO_INDEX], &io);
    let events = unit.drain_pending_events();

    assert_eq!(
        events[1].command,
        GameBoyAudioCommand::NoteOn {
            frequency_hz: 64.0,
            volume: 1.0,
            balance: GameBoyAudioBalance {
                left_gain: 0.0,
                right_gain: 0.5,
            },
        }
    );
}

#[test]
fn pulse_duty_write_queues_runtime_update() {
    let mut unit = AudioUnitState::default();
    let mut io = default_io();
    io[NR12_IO_INDEX] = 0xf0;
    io[NR14_IO_INDEX] = TRIGGER_BIT;
    unit.write_register(NR14_IO_INDEX, io[NR14_IO_INDEX], &io);
    unit.drain_pending_events();

    io[NR11_IO_INDEX] = 0x80;
    unit.write_register(NR11_IO_INDEX, io[NR11_IO_INDEX], &io);

    let events = unit.drain_pending_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].command, GameBoyAudioCommand::DutyCycle(0.5));
}

#[test]
fn pulse_sweep_queues_frequency_update() {
    let mut unit = AudioUnitState::default();
    let mut io = default_io();
    let sweep_ticks = GB_CLOCK_HZ as u64 / SWEEP_CLOCK_HZ;
    io[NR10_IO_INDEX] = 0x19;
    io[NR11_IO_INDEX] = 0x80;
    io[NR12_IO_INDEX] = 0xf0;
    io[NR13_IO_INDEX] = 0x00;
    io[NR14_IO_INDEX] = TRIGGER_BIT | 0x04;

    unit.write_register(NR14_IO_INDEX, io[NR14_IO_INDEX], &io);
    unit.drain_pending_events();
    unit.advance_ticks(sweep_ticks as i32, &mut io);

    let events = unit.drain_pending_events();
    assert_eq!(events.len(), 1);
    let GameBoyAudioCommand::Frequency { frequency_hz } = events[0].command else {
        panic!("expected frequency event, got {:?}", events[0].command);
    };
    assert!((frequency_hz - (256.0 / 3.0)).abs() < 0.0001);
}

#[test]
fn global_disable_queues_all_notes_off_and_clears_status() {
    let mut unit = AudioUnitState::default();
    let mut io = default_io();
    io[NR12_IO_INDEX] = 0xf0;
    io[NR14_IO_INDEX] = TRIGGER_BIT;
    unit.write_register(NR14_IO_INDEX, io[NR14_IO_INDEX], &io);
    assert_eq!(unit.channel_status_bits(), 0x01);
    unit.drain_pending_events();

    unit.write_register(NR52_IO_INDEX, 0x00, &io);

    assert_eq!(unit.channel_status_bits(), 0x00);
    let events = unit.drain_pending_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].command, GameBoyAudioCommand::AllNotesOff);
}

#[test]
fn reset_queues_all_notes_off_for_existing_midi_graph_voices() {
    let mut unit = AudioUnitState::default();

    unit.reset_for_rom_load(GB_CLOCK_HZ);

    let events = unit.drain_pending_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].tick, 0);
    assert_eq!(events[0].command, GameBoyAudioCommand::AllNotesOff);
}
