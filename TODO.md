
BUGS:
- The current GameBoy homebrew integration (pull pages indiscriminately from the API) seems to produce duplicates
- Emulation accuracy fixes:
  - Sequence interrupt acknowledgement and stack writes across the five M-cycles rather than applying their effects before advancing those cycles
  - Replace whole-scanline rendering and estimated mode 3 penalties with a dot-stepped pixel fetcher/FIFO, so mid-scanline register changes and fetch stalls take effect at the correct time
  - Implement CGB bus-specific CPU conflicts, PPU/OAM conflicts, and hardware DMA source decoding. Tie DMA start/restart timing to the actual register-write M-cycle
  - Replace independent length/envelope/sweep countdowns with hardware frame-sequencer timing tied to DIV. Complete register-driven oscillator details, including the NR43 noise-width setting, and validate channel timing against hardware test ROMs
- MIDI Graph hardware-accuracy updates:
  - Equal-timestamp events are unordered: BinaryHeap compares only audio frames, allowing NoteOn, NoteOff, and pitch events to be reordered (fix: Add a monotonic sequence number as a secondary ordering key)
  - Pitch starts one frame late: Generators reset pitch on NoteOn; same-frame pitch ordering is currently unsafe (fix: Stable event ordering, then queue NoteOn followed by PitchMultiplier at the same frame, or add an atomic note-with-pitch event)
  - Wavetable resampling is stepped: Nearest-sample playback introduces aliasing and roughness (fix: Add linear or higher-quality interpolation with correct loop wrapping)
  - Wavetable replacement may click: CH3 table changes are instantaneous (fix: Preserve playback phase and optionally crossfade only where appropriate; accurate wave RAM timing is preferable)
  - No Game Boy output filtering: DAC bias and hardware high-pass filtering are absent (fix: Add configurable Game Boy model-specific DC removal/high-pass filtering)
- Audio emulation fixes:
  - Envelope period zero is wrong: It currently disables envelope progression, potentially causing permanently audible notes; hardware treats zero as period eight (fix: Clock period-zero envelopes as period eight)
  - CH3 wave RAM access is oversimplified: Active writes replace the entire wavetable, ignoring current sample position and DMG/CGB/AGB access rules (fix: Track CH3’s sample index and implement model-specific active-access behavior)
  - CH3 trigger behavior is incomplete: Missing buffered-last-sample output, initial sample-zero skip, playback delay, and DMG retrigger corruption (fix: Model CH3’s sample buffer and period counter directly)
  - APU clocks are approximate: Length, envelope, and sweep use elapsed-duration counters rather than the DIV-APU frame sequencer (fix: Implement the 512 Hz DIV-APU sequencer and its edge-sensitive register behavior)
  - Sweep and length edge cases remain: Trigger reloads, period-zero handling, direction changes, and overflow behavior are incomplete (fix: Implement documented frame-sequencer edge rules with hardware test ROM coverage)
  - Noise is approximate: The LFSR’s timing and unusual divisor/shift cases are not fully hardware-accurate (fix: Clock the actual 15/7-bit LFSR from NR43 periods, including disabled high shifts)
  - Mixer remains idealized: Hardware DAC polarity, DC offsets, NR32 digital shifting, NR50 behavior, and output-stage coloration are simplified (fix: Mix digital channel outputs through modeled DAC, routing, master-volume, and HPF stages)
  - Potential stuck notes: Unstable MIDI event ordering and period-zero envelopes can both leave sources active (fix: Fix scheduler ordering and envelope semantics; compare NR52 state with MIDI Graph node state when diagnosing remaining cases)

UPDATES:
- Ensure test ROMs can be enabled in the ROM providers
- Update app name, default colour palette, and branding assets
- Update the placeholder graphics on some settings screens
- Refresh standard visual components with better designs
- Build the NES emulation
- Make the current GameBoy homebrew integration (indiscriminately pulling from an API) disabled
- Add a curated list of homebrew ROMs

TESTING:
