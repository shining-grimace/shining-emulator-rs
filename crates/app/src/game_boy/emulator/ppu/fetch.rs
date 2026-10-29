//! Addresses are driven on T1 and VRAM data is captured on T2. The fixed
//! CGB target is CPU CGB E; it does not have CGB D's low-plane alias quirk.
use super::{Context, PixelPipeline, io};

pub(super) fn address_tile(core: &Context<'_>, p: &mut PixelPipeline) {
    let lcd = io(core, 0x40);
    let y = if p.window_active {
        p.window_line
    } else {
        io(core, 0x44).wrapping_add(io(core, 0x42))
    };
    let x = if p.window_active {
        p.fetcher.tile_x
    } else {
        (io(core, 0x43) / 8).wrapping_add(p.fetcher.tile_x) & 31
    };
    let map = if lcd & (if p.window_active { 0x40 } else { 8 }) != 0 {
        0x1c00
    } else {
        0x1800
    };
    p.fetcher.address = Some(map + u16::from(y / 8) * 32 + u16::from(x));
    // CGB E samples the entire Y coordinate here, before the tile read.
    p.fetcher.row = y & 7;
}

pub(super) fn read_tile(core: &Context<'_>, p: &mut PixelPipeline) {
    // Legacy save states may resume at T2 without an address latch.
    if p.fetcher.address.is_none() {
        address_tile(core, p);
    }
    let address = usize::from(p.fetcher.address.take().unwrap_or_default());
    p.fetcher.tile = core.memory.vram[address];
    p.fetcher.attributes = if core.cgb {
        core.memory.vram[address + 0x2000]
    } else {
        0
    };
}

pub(super) fn address_plane(core: &Context<'_>, p: &mut PixelPipeline, high: bool) {
    let f = &mut p.fetcher;
    // DMG re-samples SCY for each plane. E keeps the row from tile T1.
    let mut row = if !core.cgb && !p.window_active {
        io(core, 0x44).wrapping_add(io(core, 0x42)) & 7
    } else {
        f.row
    };
    if f.attributes & 0x40 != 0 {
        row = 7 - row;
    }
    f.unsigned_tiles = io(core, 0x40) & 0x10 != 0;
    let base = if f.unsigned_tiles {
        u16::from(f.tile) * 16
    } else {
        u16::try_from(0x1000_i32 + i32::from(i8::from_ne_bytes([f.tile])) * 16).unwrap_or_default()
    };
    let bank = if f.attributes & 8 != 0 { 0x2000 } else { 0 };
    f.address = Some(bank + base + u16::from(row) * 2 + u16::from(high));
}

pub(super) fn read_plane(core: &Context<'_>, p: &mut PixelPipeline, high: bool) -> u8 {
    if p.fetcher.address.is_none() {
        address_plane(core, p, high);
    }
    let address = usize::from(p.fetcher.address.take().unwrap_or_default());
    let raw = core.memory.vram[address];
    let change = core.cgb.then_some(core.tile_select_change).flatten();
    let glitched = match change {
        Some(false) if p.fetcher.unsigned_tiles && p.fetcher.tile < 0x80 => Some(p.fetcher.tile),
        Some(true) if !p.fetcher.unsigned_tiles => Some(p.cgb_fetch.retained_data),
        _ => None,
    };
    let data = glitched.unwrap_or(raw);
    // A falling TILE_SEL edge still leaves the actual bus read in the latch.
    // Ordinary high-plane reads and sprite high-plane reads also drive it.
    if core.cgb && (change == Some(false) || (high && glitched.is_none())) {
        p.cgb_fetch.retained_data = raw;
    }
    data
}
