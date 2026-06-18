
# Notes

## For Theme

Guidelines used for theme colours:
- Align background images to the same average brightness and the same brightness range
  - Benchmark for this: Luminance mean of 0.026 and standard deviation of 0.022
- Match all palette colours for all palettes to the same luminosity
  - Using Value in GIMP; aim for 71
- Pick colours based on split-complementary (with the secondary colour being the high-contrast one)
  - Primary and tertiary are 60 degrees apart, secondary is 150 degrees from either of those

For background image assets, I prompted using this template:
> Draw a <theme>-inspired image with <props>, plus <stylistic effects>, while being fairly minimal yet portraying emotions of <theme emotions>. There should be only a few objects in view, and quite close up, and be fairly dark as if at twilight. Draw in a 1990s SNES style, using various colour hues but with a very narrow color palette (such that gradients are not smooth but with noticeable graduations) typical of a GameBoy Color image. Draw in a resolution of 160 pixels wide by 144 pixels high.

# Credits

The skill for sticking to good conventions was generated based on advice from a YouTube video.

Credit: Meta-programming template for generating domain-specific AI agent guardrails:
https://www.youtube.com/watch?v=LJ-xMF1AbDU

# Emulation architecture review

## Overview

Codex was asked to compare the current emulation architecture against the pandoc at `https://gbdev.io/pandocs/` and the technical reference at `https://gekkio.fi/files/gb-docs/gbctr.pdf`, and to give advice based purely on that comparison regarding what could be done to close gaps in emulation accuracy. The generated advice is listed below.

## Advice

The implementation is instruction-stepped rather than dot/M-cycle stepped: timers, interrupts, serial, PPU, and audio are advanced after each `perform_op`. That pattern is the root of several accuracy gaps, especially where the docs describe edge-triggered or per-M-cycle behavior.

### High Priority

CPU interrupt behavior is not hardware-accurate. `EI` enables `ime` immediately in [execution.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/execution.rs:285), while Pan Docs and GBCTR say it is delayed until after the next instruction/M-cycle. `HALT` and interrupt servicing in [tick.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/systems/tick.rs:157) also miss the HALT bug, non-IME wake behavior, and the 5 M-cycle interrupt acknowledge path. Sources: Pan Docs on [Interrupts](https://gbdev.io/pandocs/Interrupts.html), [HALT](https://gbdev.io/pandocs/halt.html), GBCTR EI timing. ([gekkio.fi](https://gekkio.fi/files/gb-docs/gbctr.pdf))

Timer emulation uses independent counters, not the hardware system counter/falling-edge model. [tick.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/systems/tick.rs:226) reloads TIMA and requests IF immediately on overflow; [bus.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/bus.rs:356) resets only the visible DIV byte. Pan Docs describes DIV as the visible part of a system counter, TAC/DIV writes causing edge-triggered increments, and TIMA overflow reload/interrupt one M-cycle later. ([gbdev.io](https://gbdev.io/pandocs/CPU_Registers_and_Flags.html))

PPU timing is fixed scanline timing, not FIFO/dot timing. [tick.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/systems/tick.rs:33) hardcodes mode 3 as 172 dots and HBlank as 204, but Pan Docs says mode 3 varies from 172-289 dots due to SCX, window, and OBJ penalties, with HBlank shortened accordingly. ([gbdev.io](https://gbdev.io/pandocs/Rendering.html))

OBJ selection/priority differs from hardware. [video.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/video.rs:317) scans/draws all 40 sprites and filters off-screen X before drawing; hardware selects only the first 10 Y-matching OBJs, X-hidden OBJs still count, and DMG drawing priority is by smaller X then OAM order. ([gbdev.io](https://gbdev.io/pandocs/OAM.html))

DMA is effectively instantaneous. [bus.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/bus.rs:482) copies OAM DMA immediately, but Pan Docs says it takes 160 M-cycles and causes CPU/PPU bus conflicts; GBCTR also notes DMA starts after a request delay and uses special source decoding. ([gbdev.io](https://gbdev.io/pandocs/CPU_Registers_and_Flags.html)) ([gekkio.fi](https://gekkio.fi/files/gb-docs/gbctr.pdf))

### Medium Priority

CGB LCDC.0 is treated as BG/window enable. [video.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/video.rs:117) skips CGB BG rendering when bit 0 is clear, but in CGB mode Pan Docs defines LCDC.0 as BG/window master priority, not visibility; OBJ should simply win priority. ([gbdev.io](https://gbdev.io/pandocs/LCDC.html))

Joypad reads and interrupts are too broad. [bus.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/bus.rs:313) returns only the low nibble for selected buttons, and [tick.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/systems/tick.rs:401) requests joypad interrupt on any input state change. Pan Docs says the interrupt is on selected P1 bits 0-3 changing high-to-low. ([gekkio.fi](https://gekkio.fi/files/gb-docs/gbctr.pdf))

Boot ROM behavior is bypassed but not fully reproduced. [cpu.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/cpu.rs:30) starts at `$0100`, and [loader.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/loader.rs:211) does not validate logo/checksum as the boot ROM would. Pan Docs and GBCTR both describe boot execution beginning at `$0000`, boot ROM mapping via `$FF50`, and logo/header checks before handoff. ([gekkio.fi](https://gekkio.fi/files/gb-docs/gbctr.pdf)) ([gekkio.fi](https://gekkio.fi/files/gb-docs/gbctr.pdf))

Several MBC details diverge. MBC2 banking in [bus.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/bus.rs:247) does not use address bit 8 across `$0000-$3FFF`; MBC1 mode 1 does not bank `$0000-$3FFF`; MBC3 RTC latch/ticking is mostly absent; RAM size codes `$04/$05` are rejected despite Pan Docs listing them. ([gbdev.io](https://gbdev.io/pandocs/Tile_Maps.html)) ([gbdev.io](https://gbdev.io/pandocs/Scrolling.html)) ([gbdev.io](https://gbdev.io/pandocs/Palettes.html)) ([gbdev.io](https://gbdev.io/pandocs/Reducing_Power_Consumption.html))

APU is mostly placeholder state. [audio_unit.rs](/home/thomas/projects/shining-emulator-rs/crates/app/src/game_boy/emulator/audio_unit.rs:172) only accumulates ticks, while Pan Docs defines four register-driven channels, mixer, envelope, length, sweep, wave, and noise behavior. ([gbdev.io](https://gbdev.io/pandocs/Audio.html))
