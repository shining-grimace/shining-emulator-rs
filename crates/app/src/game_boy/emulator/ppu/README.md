# Game Boy LCD pipeline

`systems/tick.rs` drives this module once per LCD dot. Mode 2 selects objects
from OAM; mode 3 consumes that selection without rescanning the final OAM.
The background fetcher reads tile IDs/attributes and each bitplane at separate
two-dot stages. Its push stage waits for FIFO space. The LCD discards the
startup and fine-scroll pixels, then emits 160 visible pixels. There is no
calculated mode-3 duration or scanline render at HBlank.

The object fetcher waits for background fetch synchronization, pauses that
fetcher for its six-dot transfer, and merges nontransparent pixels into the
object FIFO. Mixing applies DMG X/OAM or CGB OAM priority and reads the live
palette/LCDC settings when the pixel reaches the LCD. Window activation clears
the background FIFO and restarts tile fetching; its internal line advances
only when window pixels were emitted. Window disable finishes the current tile;
reactivation at a new WX advances to the next window row. Off-left clipping,
the WX-zero fractional-scroll timing adjustment, and the extra colour-zero
pixel after changing an active window's WX are handled by the FIFO.

The fixed CGB hardware target is **CPU CGB E**. Addresses are latched on the
first dot of a fetch and data is read on the second. Y is sampled at tile
address setup, so E cannot mix SCY rows between bitplanes. A falling TILE_SEL
edge on a data-read dot substitutes the tile index (except for tiles shared by
both addressing modes); a rising edge reuses retained bus data. Sprite high
planes and normal background high planes update that latch, which persists
across window restarts, scanlines, and frames. A write during a stalled fetch
expires after its own dot. CGB D's special bitplane alias is deliberately absent.

Fetcher phases/addresses, retained bus data, queued pixels, window latches, and the partial framebuffer are
serialized. Older saves without these fields restart the current transfer.

Timing references:

- [Pan Docs pixel FIFO](https://gbdev.io/pandocs/pixel_fifo.html)
- [Pan Docs rendering](https://gbdev.io/pandocs/Rendering.html)
- [Mealybug PPU documentation](https://github.com/mattcurrie/mealybug-tearoom-tests/blob/master/the-comprehensive-game-boy-ppu-documentation.md)

Tests cover dot counts, individual bitplane register sampling, live palette and
window changes, object timing/priority and DMA ownership, SGB output, and exact
continuation after a mid-fetch save. The Mooneye acceptance baseline remains
an independent regression check; it does not exhaustively verify PPU pixels.
