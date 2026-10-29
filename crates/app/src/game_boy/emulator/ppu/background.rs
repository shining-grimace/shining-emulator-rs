use super::state::{BackgroundFetcher, FetchStep, Pixel};
use super::{Context, PixelPipeline, fetch};

pub(super) fn begin(core: &Context<'_>, pipeline: &mut PixelPipeline) {
    // The initial tile index is on the fetcher's address latch at the
    // mode-2/mode-3 boundary. Its first row is discarded by the LCD pipeline.
    pipeline.fetcher = BackgroundFetcher::default();
    fetch::address_tile(core, pipeline);
    fetch::read_tile(core, pipeline);
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
    if p.fetcher.phase == 0 {
        match p.fetcher.step {
            FetchStep::Tile => fetch::address_tile(core, p),
            FetchStep::Low => fetch::address_plane(core, p, false),
            FetchStep::High => fetch::address_plane(core, p, true),
            FetchStep::Push => {}
        }
        p.fetcher.phase = 1;
        return;
    }
    p.fetcher.phase = 0;
    match p.fetcher.step {
        FetchStep::Tile => {
            fetch::read_tile(core, p);
            p.fetcher.step = FetchStep::Low;
        }
        FetchStep::Low => {
            p.fetcher.low = fetch::read_plane(core, p, false);
            p.fetcher.step = FetchStep::High;
        }
        FetchStep::High => {
            p.fetcher.high = fetch::read_plane(core, p, true);
            p.fetcher.step = FetchStep::Push;
            push(p);
        }
        FetchStep::Push => {}
    }
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
            index: x, // Position in this tile, also used for window tile completion.
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
