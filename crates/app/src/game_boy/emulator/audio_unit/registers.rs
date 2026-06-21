use super::events::{GameBoyAudioBalance, GameBoyAudioChannel};

pub(super) const CHANNEL_COUNT: usize = 4;
pub(super) const LENGTH_CLOCK_HZ: u64 = 256;
pub(super) const ENVELOPE_CLOCK_HZ: u64 = 64;
const PULSE_TIMER_HZ: f32 = 131_072.0;
const WAVE_TIMER_HZ: f32 = 65_536.0;
const NOISE_TIMER_HZ: f32 = 524_288.0;

pub(super) const NR10_IO_INDEX: usize = 0x10;
pub(super) const NR11_IO_INDEX: usize = 0x11;
pub(super) const NR12_IO_INDEX: usize = 0x12;
pub(super) const NR13_IO_INDEX: usize = 0x13;
pub(super) const NR14_IO_INDEX: usize = 0x14;
pub(super) const NR21_IO_INDEX: usize = 0x16;
pub(super) const NR22_IO_INDEX: usize = 0x17;
pub(super) const NR23_IO_INDEX: usize = 0x18;
pub(super) const NR24_IO_INDEX: usize = 0x19;
pub(super) const NR30_IO_INDEX: usize = 0x1a;
pub(super) const NR31_IO_INDEX: usize = 0x1b;
pub(super) const NR32_IO_INDEX: usize = 0x1c;
pub(super) const NR33_IO_INDEX: usize = 0x1d;
pub(super) const NR34_IO_INDEX: usize = 0x1e;
pub(super) const NR41_IO_INDEX: usize = 0x20;
pub(super) const NR42_IO_INDEX: usize = 0x21;
pub(super) const NR43_IO_INDEX: usize = 0x22;
pub(super) const NR44_IO_INDEX: usize = 0x23;
pub(super) const NR50_IO_INDEX: usize = 0x24;
pub(super) const NR51_IO_INDEX: usize = 0x25;
pub(super) const NR52_IO_INDEX: usize = 0x26;
pub(super) const WAVE_RAM_START_INDEX: usize = 0x30;
pub(super) const WAVE_RAM_END_INDEX: usize = 0x3f;

pub(super) const TRIGGER_BIT: u8 = 0x80;
pub(super) const LENGTH_ENABLE_BIT: u8 = 0x40;
pub(super) const GLOBAL_AUDIO_ENABLE_BIT: u8 = 0x80;

pub(super) fn envelope_dac_enabled(value: u8) -> bool {
    value & 0xf8 != 0
}

pub(super) fn envelope_volume(value: u8) -> f32 {
    f32::from(value.min(15)) / 15.0
}

pub(super) fn wave_output_level(io_ports: &[u8]) -> f32 {
    match (io(io_ports, NR32_IO_INDEX) >> 5) & 0x03 {
        0 => 0.0,
        1 => 1.0,
        2 => 0.5,
        _ => 0.25,
    }
}

pub(super) fn pulse_frequency(low: u8, high: u8) -> f32 {
    let raw = u16::from(low) | (u16::from(high & 0x07) << 8);
    timer_frequency(PULSE_TIMER_HZ, raw)
}

pub(super) fn wave_frequency(low: u8, high: u8) -> f32 {
    let raw = u16::from(low) | (u16::from(high & 0x07) << 8);
    timer_frequency(WAVE_TIMER_HZ, raw)
}

fn timer_frequency(clock_hz: f32, raw: u16) -> f32 {
    let divisor = (2048_u16.saturating_sub(raw)).max(1);
    clock_hz / f32::from(divisor)
}

pub(super) fn noise_frequency(value: u8) -> f32 {
    let shift = u32::from(value >> 4);
    let divisor_code = value & 0x07;
    let divisor = match divisor_code {
        0 => 8_u32,
        code => u32::from(code) * 16,
    };
    let shifted_divisor = divisor.checked_shl(shift).unwrap_or(u32::MAX).max(1);
    NOISE_TIMER_HZ / shifted_divisor as f32
}

pub(super) fn channel_output_gains(channel: GameBoyAudioChannel, io_ports: &[u8]) -> (f32, f32) {
    let nr50 = io(io_ports, NR50_IO_INDEX);
    let nr51 = io(io_ports, NR51_IO_INDEX);
    let channel_bit = 1_u8 << channel.index();
    let right_enabled = nr51 & channel_bit != 0;
    let left_enabled = nr51 & (channel_bit << 4) != 0;
    let right_gain = if right_enabled {
        f32::from(nr50 & 0x07) / 7.0
    } else {
        0.0
    };
    let left_gain = if left_enabled {
        f32::from((nr50 >> 4) & 0x07) / 7.0
    } else {
        0.0
    };
    (left_gain, right_gain)
}

pub(super) fn balance_from_gains(left_gain: f32, right_gain: f32) -> GameBoyAudioBalance {
    match (left_gain > 0.0, right_gain > 0.0) {
        (true, true) if (left_gain - right_gain).abs() <= f32::EPSILON => GameBoyAudioBalance::Both,
        (true, true) => GameBoyAudioBalance::Pan(right_gain / (left_gain + right_gain)),
        (true, false) => GameBoyAudioBalance::Left,
        (false, true) => GameBoyAudioBalance::Right,
        (false, false) => GameBoyAudioBalance::Both,
    }
}

pub(super) fn decode_wavetable(io_ports: &[u8]) -> [f32; 16] {
    let mut samples = [0.0; 16];
    for (index, output) in samples.iter_mut().enumerate() {
        let byte = io(io_ports, WAVE_RAM_START_INDEX + index);
        let nibble = byte >> 4;
        *output = (f32::from(nibble) / 7.5) - 1.0;
    }
    samples
}

pub(super) fn io(io_ports: &[u8], index: usize) -> u8 {
    io_ports.get(index).copied().unwrap_or(0)
}
