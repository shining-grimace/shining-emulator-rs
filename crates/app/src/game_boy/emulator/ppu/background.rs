use super::state::{BackgroundFetcher, FetchStep, Pixel};
use super::{Context, PixelPipeline, io};

pub(super) fn begin(core: &Context<'_>, pipeline: &mut PixelPipeline) {
    // The initial tile index is on the fetcher's address latch at the
    // mode-2/mode-3 boundary. Its first row is discarded by the LCD pipeline.
    pipeline.fetcher = BackgroundFetcher::default();
    get_tile(core, pipeline);
    pipeline.fetcher.step = FetchStep::Low;
    pipeline.dummy_tile = true;
}

pub(super) fn restart(pipeline: &mut PixelPipeline, tile_x: u8) {
    pipeline.bg.clear();
    pipeline.fetcher = BackgroundFetcher {
        tile_x: tile_x & 31,
        ..Default::default()
    };
    pipeline.dummy_tile = false;
}

pub(super) fn tick(core: &Context<'_>, p: &mut PixelPipeline) {
    if p.fetcher.step == FetchStep::Push {
        push(p);
        return;
    }
    p.fetcher.phase ^= 1;
    if p.fetcher.phase != 0 {
        return;
    }
    match p.fetcher.step {
        FetchStep::Tile => {
            get_tile(core, p);
            p.fetcher.step = FetchStep::Low;
        }
        FetchStep::Low => {
            p.fetcher.low = read_plane(core, p, false);
            p.fetcher.step = FetchStep::High;
        }
        FetchStep::High => {
            p.fetcher.high = read_plane(core, p, true);
            p.fetcher.step = FetchStep::Push;
            push(p);
        }
        FetchStep::Push => {}
    }
}

fn get_tile(core: &Context<'_>, p: &mut PixelPipeline) {
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
    let address = map + usize::from(y / 8) * 32 + usize::from(x);
    p.fetcher.tile = core.memory.vram[address];
    p.fetcher.attributes = if core.cgb {
        core.memory.vram[address + 0x2000]
    } else {
        0
    };
    p.fetcher.row = y & 7;
}

fn read_plane(core: &Context<'_>, p: &PixelPipeline, high: bool) -> u8 {
    let f = &p.fetcher;
    // DMG re-samples SCY for each bitplane; CGB D/E latch it with the tile.
    let mut row = if !core.cgb && !p.window_active {
        io(core, 0x44).wrapping_add(io(core, 0x42)) & 7
    } else {
        f.row
    };
    if f.attributes & 0x40 != 0 {
        row = 7 - row;
    }
    let tile_base = if io(core, 0x40) & 0x10 != 0 {
        usize::from(f.tile) * 16
    } else {
        usize::try_from(0x1000_i32 + i32::from(i8::from_ne_bytes([f.tile])) * 16)
            .unwrap_or_default()
    };
    let bank = if f.attributes & 8 != 0 { 0x2000 } else { 0 };
    core.memory.vram[bank + tile_base + usize::from(row) * 2 + usize::from(high)]
}

fn push(p: &mut PixelPipeline) {
    if p.bg.len != 0 {
        return;
    }
    for x in 0..8 {
        let bit = if p.fetcher.attributes & 0x20 != 0 {
            x
        } else {
            7 - x
        };
        let colour = ((p.fetcher.low >> bit) & 1) | (((p.fetcher.high >> bit) & 1) << 1);
        p.bg.push(Pixel {
            colour,
            palette: p.fetcher.attributes & 7,
            flags: p.fetcher.attributes,
            ..Default::default()
        });
    }
    if p.dummy_tile {
        p.dummy_tile = false;
    } else {
        p.fetcher.tile_x = (p.fetcher.tile_x + 1) & 31;
    }
    p.fetcher.step = FetchStep::Tile;
}
