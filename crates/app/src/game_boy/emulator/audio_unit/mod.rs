use crate::game_boy::emulator::constants::GB_CLOCK_HZ;

mod events;
mod registers;
#[cfg(test)]
mod tests;

pub(crate) use events::{
    GameBoyAudioBalance, GameBoyAudioChannel, GameBoyAudioCommand, GameBoyAudioEvent,
};
use registers::*;

#[derive(Clone, Copy, Debug, Default)]
struct EnvelopeState {
    volume: u8,
    increases: bool,
    period_ticks: u64,
    ticks_until_step: u64,
    active: bool,
}

#[derive(Debug)]
pub(crate) struct AudioUnitState {
    pub(crate) cumulative_ticks: u64,
    pub(crate) base_running_speed: usize,
    global_audio_enable: bool,
    running: [bool; CHANNEL_COUNT],
    length_enabled: [bool; CHANNEL_COUNT],
    length_remaining_ticks: [u64; CHANNEL_COUNT],
    envelope: [EnvelopeState; CHANNEL_COUNT],
    mix_gain: [f32; CHANNEL_COUNT],
    last_frequency_hz: [f32; CHANNEL_COUNT],
    last_volume: [f32; CHANNEL_COUNT],
    last_balance: [GameBoyAudioBalance; CHANNEL_COUNT],
    pending_events: Vec<GameBoyAudioEvent>,
}

impl Default for AudioUnitState {
    fn default() -> Self {
        Self {
            cumulative_ticks: 0,
            base_running_speed: GB_CLOCK_HZ as usize,
            global_audio_enable: true,
            running: [false; CHANNEL_COUNT],
            length_enabled: [false; CHANNEL_COUNT],
            length_remaining_ticks: [0; CHANNEL_COUNT],
            envelope: [EnvelopeState::default(); CHANNEL_COUNT],
            mix_gain: [1.0; CHANNEL_COUNT],
            last_frequency_hz: [0.0; CHANNEL_COUNT],
            last_volume: [0.0; CHANNEL_COUNT],
            last_balance: [GameBoyAudioBalance::Both; CHANNEL_COUNT],
            pending_events: Vec::new(),
        }
    }
}

impl AudioUnitState {
    pub(crate) fn reset_for_rom_load(&mut self, clock_frequency_hz: i64) {
        self.cumulative_ticks = 0;
        self.global_audio_enable = true;
        self.running = [false; CHANNEL_COUNT];
        self.length_enabled = [false; CHANNEL_COUNT];
        self.length_remaining_ticks = [0; CHANNEL_COUNT];
        self.envelope = [EnvelopeState::default(); CHANNEL_COUNT];
        self.mix_gain = [1.0; CHANNEL_COUNT];
        self.last_frequency_hz = [0.0; CHANNEL_COUNT];
        self.last_volume = [0.0; CHANNEL_COUNT];
        self.last_balance = [GameBoyAudioBalance::Both; CHANNEL_COUNT];
        self.pending_events.clear();
        self.base_running_speed =
            usize::try_from(clock_frequency_hz).unwrap_or(GB_CLOCK_HZ as usize);
        self.queue_event(None, GameBoyAudioCommand::AllNotesOff);
    }

    pub(crate) fn advance_ticks(&mut self, ticks: i32) {
        let Ok(ticks) = u64::try_from(ticks.max(0)) else {
            return;
        };
        if ticks == 0 {
            return;
        }

        let start_tick = self.cumulative_ticks;
        for channel in [
            GameBoyAudioChannel::Pulse1,
            GameBoyAudioChannel::Pulse2,
            GameBoyAudioChannel::Wave,
            GameBoyAudioChannel::Noise,
        ] {
            let index = channel.index();
            if !self.running[index] {
                continue;
            }

            let active_ticks = self.active_ticks_before_length_expiry(channel, ticks);
            if active_ticks > 0 {
                self.advance_envelope(channel, start_tick, active_ticks);
            }
            self.advance_length(channel, start_tick, ticks);
        }

        self.cumulative_ticks = self.cumulative_ticks.saturating_add(ticks);
    }

    pub(crate) fn write_register(&mut self, index: usize, value: u8, io_ports: &[u8]) {
        if index == NR52_IO_INDEX {
            self.write_global_audio_enable(value);
            return;
        }

        if (WAVE_RAM_START_INDEX..=WAVE_RAM_END_INDEX).contains(&index) {
            self.queue_wavetable_update(io_ports);
            return;
        }

        if !self.global_audio_enable {
            return;
        }

        match index {
            NR11_IO_INDEX => {
                self.refresh_length_from_register(GameBoyAudioChannel::Pulse1, io_ports)
            }
            NR12_IO_INDEX => {
                self.write_envelope_register(GameBoyAudioChannel::Pulse1, value, io_ports)
            }
            NR13_IO_INDEX => self.refresh_frequency(GameBoyAudioChannel::Pulse1, io_ports),
            NR14_IO_INDEX => {
                self.write_trigger_register(GameBoyAudioChannel::Pulse1, value, io_ports)
            }
            NR21_IO_INDEX => {
                self.refresh_length_from_register(GameBoyAudioChannel::Pulse2, io_ports)
            }
            NR22_IO_INDEX => {
                self.write_envelope_register(GameBoyAudioChannel::Pulse2, value, io_ports)
            }
            NR23_IO_INDEX => self.refresh_frequency(GameBoyAudioChannel::Pulse2, io_ports),
            NR24_IO_INDEX => {
                self.write_trigger_register(GameBoyAudioChannel::Pulse2, value, io_ports)
            }
            NR30_IO_INDEX => self.write_wave_dac_register(value),
            NR31_IO_INDEX => self.refresh_length_from_register(GameBoyAudioChannel::Wave, io_ports),
            NR32_IO_INDEX => self.queue_mix_update(GameBoyAudioChannel::Wave, io_ports),
            NR33_IO_INDEX => self.refresh_frequency(GameBoyAudioChannel::Wave, io_ports),
            NR34_IO_INDEX => {
                self.write_trigger_register(GameBoyAudioChannel::Wave, value, io_ports)
            }
            NR41_IO_INDEX => {
                self.refresh_length_from_register(GameBoyAudioChannel::Noise, io_ports)
            }
            NR42_IO_INDEX => {
                self.write_envelope_register(GameBoyAudioChannel::Noise, value, io_ports)
            }
            NR43_IO_INDEX => self.refresh_frequency(GameBoyAudioChannel::Noise, io_ports),
            NR44_IO_INDEX => {
                self.write_trigger_register(GameBoyAudioChannel::Noise, value, io_ports)
            }
            NR50_IO_INDEX | NR51_IO_INDEX => self.queue_all_mix_updates(io_ports),
            NR10_IO_INDEX => {}
            _ => {}
        }
    }

    pub(crate) fn channel_status_bits(&self) -> u8 {
        [
            GameBoyAudioChannel::Pulse1,
            GameBoyAudioChannel::Pulse2,
            GameBoyAudioChannel::Wave,
            GameBoyAudioChannel::Noise,
        ]
        .into_iter()
        .filter(|channel| self.running[channel.index()])
        .fold(0, |status, channel| status | channel.status_bit())
    }

    pub(crate) fn drain_pending_events(&mut self) -> Vec<GameBoyAudioEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn write_global_audio_enable(&mut self, value: u8) {
        let enabled = value & GLOBAL_AUDIO_ENABLE_BIT != 0;
        if self.global_audio_enable == enabled {
            return;
        }

        self.global_audio_enable = enabled;
        if !enabled {
            self.running = [false; CHANNEL_COUNT];
            self.length_remaining_ticks = [0; CHANNEL_COUNT];
            self.envelope = [EnvelopeState::default(); CHANNEL_COUNT];
            self.queue_event(None, GameBoyAudioCommand::AllNotesOff);
        }
    }

    fn write_wave_dac_register(&mut self, value: u8) {
        if value & 0x80 == 0 {
            self.stop_channel(GameBoyAudioChannel::Wave);
        }
    }

    fn write_envelope_register(
        &mut self,
        channel: GameBoyAudioChannel,
        value: u8,
        io_ports: &[u8],
    ) {
        if !envelope_dac_enabled(value) {
            self.stop_channel(channel);
            return;
        }

        if self.running[channel.index()] {
            self.configure_envelope(channel, value);
            self.queue_mix_update(channel, io_ports);
        }
    }

    fn write_trigger_register(&mut self, channel: GameBoyAudioChannel, value: u8, io_ports: &[u8]) {
        let index = channel.index();
        self.length_enabled[index] = value & LENGTH_ENABLE_BIT != 0;
        if self.running[index]
            && self.length_enabled[index]
            && self.length_remaining_ticks[index] == 0
        {
            self.refresh_length_from_register(channel, io_ports);
        }

        if value & TRIGGER_BIT == 0 {
            return;
        }

        self.trigger_channel(channel, io_ports);
    }

    fn trigger_channel(&mut self, channel: GameBoyAudioChannel, io_ports: &[u8]) {
        if !self.channel_dac_enabled(channel, io_ports) {
            self.stop_channel(channel);
            return;
        }

        let index = channel.index();
        self.running[index] = true;
        self.configure_channel_on_trigger(channel, io_ports);
        self.refresh_length_from_register(channel, io_ports);

        let frequency_hz = self.channel_frequency(channel, io_ports);
        let volume = self.current_channel_volume(channel, io_ports);
        let balance = self.current_channel_balance(channel, io_ports);
        self.last_frequency_hz[index] = frequency_hz;
        self.last_volume[index] = volume;
        self.last_balance[index] = balance;

        if channel == GameBoyAudioChannel::Wave {
            self.queue_wavetable_update(io_ports);
        }

        self.queue_event(
            Some(channel),
            GameBoyAudioCommand::NoteOn {
                frequency_hz,
                volume,
                balance,
            },
        );
    }

    fn configure_channel_on_trigger(&mut self, channel: GameBoyAudioChannel, io_ports: &[u8]) {
        match channel {
            GameBoyAudioChannel::Pulse1 => {
                self.configure_envelope(channel, io(io_ports, NR12_IO_INDEX));
            }
            GameBoyAudioChannel::Pulse2 => {
                self.configure_envelope(channel, io(io_ports, NR22_IO_INDEX));
            }
            GameBoyAudioChannel::Wave => {}
            GameBoyAudioChannel::Noise => {
                self.configure_envelope(channel, io(io_ports, NR42_IO_INDEX));
            }
        }
    }

    fn configure_envelope(&mut self, channel: GameBoyAudioChannel, value: u8) {
        let period = value & 0x07;
        let period_ticks = if period == 0 {
            0
        } else {
            self.clock_ticks_per_second()
                .saturating_mul(u64::from(period))
                / ENVELOPE_CLOCK_HZ
        };
        self.envelope[channel.index()] = EnvelopeState {
            volume: value >> 4,
            increases: value & 0x08 != 0,
            period_ticks,
            ticks_until_step: period_ticks,
            active: period_ticks > 0,
        };
    }

    fn refresh_length_from_register(&mut self, channel: GameBoyAudioChannel, io_ports: &[u8]) {
        let index = channel.index();
        if !self.length_enabled[index] {
            self.length_remaining_ticks[index] = 0;
            return;
        }

        let units = match channel {
            GameBoyAudioChannel::Pulse1 => 64 - u64::from(io(io_ports, NR11_IO_INDEX) & 0x3f),
            GameBoyAudioChannel::Pulse2 => 64 - u64::from(io(io_ports, NR21_IO_INDEX) & 0x3f),
            GameBoyAudioChannel::Wave => 256 - u64::from(io(io_ports, NR31_IO_INDEX)),
            GameBoyAudioChannel::Noise => 64 - u64::from(io(io_ports, NR41_IO_INDEX) & 0x3f),
        };
        self.length_remaining_ticks[index] =
            units.saturating_mul(self.clock_ticks_per_second()) / LENGTH_CLOCK_HZ;
    }

    fn refresh_frequency(&mut self, channel: GameBoyAudioChannel, io_ports: &[u8]) {
        let index = channel.index();
        if !self.running[index] {
            return;
        }

        let frequency_hz = self.channel_frequency(channel, io_ports);
        if (frequency_hz - self.last_frequency_hz[index]).abs() <= f32::EPSILON {
            return;
        }
        self.last_frequency_hz[index] = frequency_hz;
        self.queue_event(
            Some(channel),
            GameBoyAudioCommand::Frequency { frequency_hz },
        );
    }

    fn queue_all_mix_updates(&mut self, io_ports: &[u8]) {
        for channel in [
            GameBoyAudioChannel::Pulse1,
            GameBoyAudioChannel::Pulse2,
            GameBoyAudioChannel::Wave,
            GameBoyAudioChannel::Noise,
        ] {
            self.queue_mix_update(channel, io_ports);
        }
    }

    fn queue_mix_update(&mut self, channel: GameBoyAudioChannel, io_ports: &[u8]) {
        let index = channel.index();
        if !self.running[index] {
            self.update_mix_state(channel, io_ports);
            return;
        }

        let volume = self.current_channel_volume(channel, io_ports);
        let balance = self.current_channel_balance(channel, io_ports);
        if (volume - self.last_volume[index]).abs() > f32::EPSILON {
            self.last_volume[index] = volume;
            self.queue_event(Some(channel), GameBoyAudioCommand::Volume(volume));
        }
        if balance != self.last_balance[index] {
            self.last_balance[index] = balance;
            self.queue_event(Some(channel), GameBoyAudioCommand::Balance(balance));
        }
    }

    fn update_mix_state(&mut self, channel: GameBoyAudioChannel, io_ports: &[u8]) {
        let index = channel.index();
        let (left_gain, right_gain) = channel_output_gains(channel, io_ports);
        self.mix_gain[index] = left_gain.max(right_gain);
        self.last_balance[index] = balance_from_gains(left_gain, right_gain);
    }

    fn advance_length(&mut self, channel: GameBoyAudioChannel, start_tick: u64, ticks: u64) -> u64 {
        let index = channel.index();
        if !self.length_enabled[index] || self.length_remaining_ticks[index] == 0 {
            return ticks;
        }

        let remaining = self.length_remaining_ticks[index];
        if remaining > ticks {
            self.length_remaining_ticks[index] = remaining - ticks;
            return ticks;
        }

        let event_tick = start_tick.saturating_add(remaining);
        self.length_remaining_ticks[index] = 0;
        self.running[index] = false;
        self.queue_event_at(event_tick, Some(channel), GameBoyAudioCommand::NoteOff);
        remaining
    }

    fn active_ticks_before_length_expiry(&self, channel: GameBoyAudioChannel, ticks: u64) -> u64 {
        let index = channel.index();
        if !self.length_enabled[index] || self.length_remaining_ticks[index] == 0 {
            return ticks;
        }
        ticks.min(self.length_remaining_ticks[index])
    }

    fn advance_envelope(&mut self, channel: GameBoyAudioChannel, start_tick: u64, ticks: u64) {
        let index = channel.index();
        let mut envelope = self.envelope[index];
        if !envelope.active || envelope.period_ticks == 0 {
            return;
        }

        let mut elapsed_ticks = ticks;
        let mut cursor_tick = start_tick;
        while elapsed_ticks >= envelope.ticks_until_step {
            cursor_tick = cursor_tick.saturating_add(envelope.ticks_until_step);
            elapsed_ticks -= envelope.ticks_until_step;

            let next_volume = if envelope.increases {
                envelope.volume.saturating_add(1).min(15)
            } else {
                envelope.volume.saturating_sub(1)
            };

            if next_volume == envelope.volume {
                envelope.active = false;
                break;
            }

            envelope.volume = next_volume;
            envelope.ticks_until_step = envelope.period_ticks;
            let volume = envelope_volume(envelope.volume) * self.mix_gain[index];
            self.last_volume[index] = volume;
            self.queue_event_at(
                cursor_tick,
                Some(channel),
                GameBoyAudioCommand::Volume(volume),
            );
        }

        if envelope.active {
            envelope.ticks_until_step = envelope.ticks_until_step.saturating_sub(elapsed_ticks);
        }
        self.envelope[index] = envelope;
    }

    fn stop_channel(&mut self, channel: GameBoyAudioChannel) {
        let index = channel.index();
        if !self.running[index] {
            return;
        }
        self.running[index] = false;
        self.length_remaining_ticks[index] = 0;
        self.envelope[index] = EnvelopeState::default();
        self.queue_event(Some(channel), GameBoyAudioCommand::NoteOff);
    }

    fn queue_wavetable_update(&mut self, io_ports: &[u8]) {
        self.queue_event(
            Some(GameBoyAudioChannel::Wave),
            GameBoyAudioCommand::Wavetable(decode_wavetable(io_ports)),
        );
    }

    fn current_channel_volume(&mut self, channel: GameBoyAudioChannel, io_ports: &[u8]) -> f32 {
        let index = channel.index();
        let (left_gain, right_gain) = channel_output_gains(channel, io_ports);
        let mix_gain = left_gain.max(right_gain);
        self.mix_gain[index] = mix_gain;
        match channel {
            GameBoyAudioChannel::Pulse1
            | GameBoyAudioChannel::Pulse2
            | GameBoyAudioChannel::Noise => envelope_volume(self.envelope[index].volume) * mix_gain,
            GameBoyAudioChannel::Wave => wave_output_level(io_ports) * mix_gain,
        }
    }

    fn current_channel_balance(
        &mut self,
        channel: GameBoyAudioChannel,
        io_ports: &[u8],
    ) -> GameBoyAudioBalance {
        let (left_gain, right_gain) = channel_output_gains(channel, io_ports);
        let balance = balance_from_gains(left_gain, right_gain);
        self.last_balance[channel.index()] = balance;
        balance
    }

    fn channel_dac_enabled(&self, channel: GameBoyAudioChannel, io_ports: &[u8]) -> bool {
        match channel {
            GameBoyAudioChannel::Pulse1 => envelope_dac_enabled(io(io_ports, NR12_IO_INDEX)),
            GameBoyAudioChannel::Pulse2 => envelope_dac_enabled(io(io_ports, NR22_IO_INDEX)),
            GameBoyAudioChannel::Wave => io(io_ports, NR30_IO_INDEX) & 0x80 != 0,
            GameBoyAudioChannel::Noise => envelope_dac_enabled(io(io_ports, NR42_IO_INDEX)),
        }
    }

    fn channel_frequency(&self, channel: GameBoyAudioChannel, io_ports: &[u8]) -> f32 {
        match channel {
            GameBoyAudioChannel::Pulse1 => {
                pulse_frequency(io(io_ports, NR13_IO_INDEX), io(io_ports, NR14_IO_INDEX))
            }
            GameBoyAudioChannel::Pulse2 => {
                pulse_frequency(io(io_ports, NR23_IO_INDEX), io(io_ports, NR24_IO_INDEX))
            }
            GameBoyAudioChannel::Wave => {
                wave_frequency(io(io_ports, NR33_IO_INDEX), io(io_ports, NR34_IO_INDEX))
            }
            GameBoyAudioChannel::Noise => noise_frequency(io(io_ports, NR43_IO_INDEX)),
        }
    }

    fn queue_event(&mut self, channel: Option<GameBoyAudioChannel>, command: GameBoyAudioCommand) {
        self.queue_event_at(self.cumulative_ticks, channel, command);
    }

    fn queue_event_at(
        &mut self,
        tick: u64,
        channel: Option<GameBoyAudioChannel>,
        command: GameBoyAudioCommand,
    ) {
        self.pending_events.push(GameBoyAudioEvent {
            tick,
            channel,
            command,
        });
    }

    fn clock_ticks_per_second(&self) -> u64 {
        u64::try_from(self.base_running_speed)
            .unwrap_or(GB_CLOCK_HZ as u64)
            .max(1)
    }
}
