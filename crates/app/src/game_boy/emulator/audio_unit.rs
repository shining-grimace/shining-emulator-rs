use crate::game_boy::emulator::constants::{
    AUDIO_BUFFER_FRAMES, AUDIO_WAVEFORM_SAMPLES, GB_CLOCK_HZ,
};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AudioSample {
    pub(crate) left: i16,
    pub(crate) right: i16,
}

#[derive(Debug)]
pub(crate) struct AudioUnitState {
    pub(crate) cumulative_ticks: u64,
    pub(crate) buffer_write_head: u32,
    pub(crate) buffer_read_head: u32,
    pub(crate) buffer: Box<[AudioSample]>,
    pub(crate) waveform_data: [i16; AUDIO_WAVEFORM_SAMPLES],
    pub(crate) global_audio_enable: bool,
    pub(crate) base_running_speed: usize,
    pub(crate) out1_generator1: i16,
    pub(crate) out1_generator2: i16,
    pub(crate) out1_generator3: i16,
    pub(crate) out1_generator4: i16,
    pub(crate) out2_generator1: i16,
    pub(crate) out2_generator2: i16,
    pub(crate) out2_generator3: i16,
    pub(crate) out2_generator4: i16,
    pub(crate) s1_running: bool,
    pub(crate) s1_duty_on_length_in_ticks: usize,
    pub(crate) s1_duty_bits: u8,
    pub(crate) s1_duty_period_in_ticks: usize,
    pub(crate) s1_current_duty_progress: usize,
    pub(crate) s1_has_sweep: bool,
    pub(crate) s1_sweep_increases: bool,
    pub(crate) s1_sweep_period_in_ticks: usize,
    pub(crate) s1_current_sweep_progress: usize,
    pub(crate) s1_current_frequency: usize,
    pub(crate) s1_frequency_divisor: usize,
    pub(crate) s1_has_length: bool,
    pub(crate) s1_length_in_ticks: usize,
    pub(crate) s1_current_length_progress: usize,
    pub(crate) s1_has_envelope: bool,
    pub(crate) s1_envelope_increases: bool,
    pub(crate) s1_envelope_value: u32,
    pub(crate) s1_envelope_step_in_ticks: usize,
    pub(crate) s1_current_envelope_step_progress: usize,
    pub(crate) s2_running: bool,
    pub(crate) s2_duty_on_length_in_ticks: usize,
    pub(crate) s2_duty_period_in_ticks: usize,
    pub(crate) s2_current_duty_progress: usize,
    pub(crate) s2_has_length: bool,
    pub(crate) s2_length_in_ticks: usize,
    pub(crate) s2_current_length_progress: usize,
    pub(crate) s2_has_envelope: bool,
    pub(crate) s2_envelope_increases: bool,
    pub(crate) s2_envelope_value: u32,
    pub(crate) s2_envelope_step_in_ticks: usize,
    pub(crate) s2_current_envelope_step_progress: usize,
    pub(crate) s3_running: bool,
    pub(crate) s3_current_waveform_position: usize,
    pub(crate) s3_has_length: bool,
    pub(crate) s3_length_in_ticks: usize,
    pub(crate) s3_current_length_progress: usize,
    pub(crate) s3_period_in_ticks: usize,
    pub(crate) s3_current_progress: usize,
    pub(crate) s3_volume_multiplier: i16,
    pub(crate) s3_volume_divisor: i16,
    pub(crate) s4_running: bool,
    pub(crate) lfsr: u32,
    pub(crate) s4_shift_period: u32,
    pub(crate) s4_shift_progress: u32,
    pub(crate) s4_shift_feedback_mask: u32,
    pub(crate) s4_has_length: bool,
    pub(crate) s4_length_in_ticks: usize,
    pub(crate) s4_current_length_progress: usize,
    pub(crate) s4_has_envelope: bool,
    pub(crate) s4_envelope_increases: bool,
    pub(crate) s4_envelope_value: u32,
    pub(crate) s4_envelope_step_in_ticks: usize,
    pub(crate) s4_current_envelope_step_progress: usize,
}

impl Default for AudioUnitState {
    fn default() -> Self {
        Self {
            cumulative_ticks: 0,
            buffer_write_head: 0,
            buffer_read_head: 0,
            buffer: vec![AudioSample::default(); AUDIO_BUFFER_FRAMES].into_boxed_slice(),
            waveform_data: [0; AUDIO_WAVEFORM_SAMPLES],
            global_audio_enable: false,
            base_running_speed: GB_CLOCK_HZ as usize,
            out1_generator1: 0,
            out1_generator2: 0,
            out1_generator3: 0,
            out1_generator4: 0,
            out2_generator1: 0,
            out2_generator2: 0,
            out2_generator3: 0,
            out2_generator4: 0,
            s1_running: false,
            s1_duty_on_length_in_ticks: 4,
            s1_duty_bits: 0,
            s1_duty_period_in_ticks: 8,
            s1_current_duty_progress: 0,
            s1_has_sweep: false,
            s1_sweep_increases: false,
            s1_sweep_period_in_ticks: 8,
            s1_current_sweep_progress: 0,
            s1_current_frequency: 0,
            s1_frequency_divisor: 2,
            s1_has_length: false,
            s1_length_in_ticks: 8,
            s1_current_length_progress: 0,
            s1_has_envelope: false,
            s1_envelope_increases: false,
            s1_envelope_value: 0,
            s1_envelope_step_in_ticks: 8,
            s1_current_envelope_step_progress: 0,
            s2_running: false,
            s2_duty_on_length_in_ticks: 4,
            s2_duty_period_in_ticks: 8,
            s2_current_duty_progress: 0,
            s2_has_length: false,
            s2_length_in_ticks: 8,
            s2_current_length_progress: 0,
            s2_has_envelope: false,
            s2_envelope_increases: false,
            s2_envelope_value: 0,
            s2_envelope_step_in_ticks: 8,
            s2_current_envelope_step_progress: 0,
            s3_running: false,
            s3_current_waveform_position: 0,
            s3_has_length: false,
            s3_length_in_ticks: 8,
            s3_current_length_progress: 0,
            s3_period_in_ticks: 8,
            s3_current_progress: 0,
            s3_volume_multiplier: 0,
            s3_volume_divisor: 1,
            s4_running: false,
            lfsr: 0x0001,
            s4_shift_period: 8,
            s4_shift_progress: 0,
            s4_shift_feedback_mask: 0x004000,
            s4_has_length: false,
            s4_length_in_ticks: 8,
            s4_current_length_progress: 0,
            s4_has_envelope: false,
            s4_envelope_increases: false,
            s4_envelope_value: 0,
            s4_envelope_step_in_ticks: 8,
            s4_current_envelope_step_progress: 0,
        }
    }
}

impl AudioUnitState {
    pub(crate) fn reset_for_rom_load(&mut self, clock_frequency_hz: i64) {
        self.buffer_write_head = 0;
        self.buffer_read_head = 0;
        self.cumulative_ticks = 0;
        self.global_audio_enable = false;
        self.s1_running = false;
        self.s2_running = false;
        self.s3_running = false;
        self.s4_running = false;
        self.base_running_speed =
            usize::try_from(clock_frequency_hz).unwrap_or(GB_CLOCK_HZ as usize);
    }

    pub(crate) fn simulate_placeholder(&mut self, ticks: i32) {
        if let Ok(ticks) = u64::try_from(ticks.max(0)) {
            self.cumulative_ticks = self.cumulative_ticks.saturating_add(ticks);
        }
    }
}
