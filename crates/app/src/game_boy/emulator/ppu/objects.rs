use super::state::{FetchStep, ObjectFetcher, Pixel};
use super::{Context, PixelPipeline, io};

pub(super) fn tick(core: &mut Context<'_>, p: &mut PixelPipeline, dma_owned: bool) -> bool {
    let enabled = core.cgb || io(core, 0x40) & 2 != 0;
    if !enabled {
        if let Some(object) = p.object.take() {
            core.oam.fetched |= 1 << object.index;
        }
        p.waiting_object = None;
        return false;
    }
    if let Some(mut object) = p.object.take() {
        match object.phase {
            1 => {
                let offset = usize::from(object.index) * 4;
                let word = if dma_owned {
                    core.dma_word
                } else {
                    [core.memory.oam[offset + 2], core.memory.oam[offset + 3]]
                };
                object.tile = word[0];
                object.flags = word[1];
                core.oam.bytes[offset + 2..offset + 4].copy_from_slice(&word);
            }
            3 => object.low = read_plane(core, object, false),
            5 => {
                object.high = read_plane(core, object, true);
                if core.cgb {
                    p.cgb_fetch.retained_data = object.high;
                }
                merge(core, p, object);
                core.oam.fetched |= 1 << object.index;
                return true;
            }
            _ => {}
        }
        object.phase += 1;
        p.object = Some(object);
        return true;
    }
    // Fine-scroll discards don't move the LCD/object FIFO position.
    if p.dummy == 0 && p.fine != 0 {
        return false;
    }
    if p.waiting_object.is_none() {
        p.waiting_object = next(core, p.cursor());
    }
    let Some(index) = p.waiting_object else {
        return false;
    };
    // Synchronise to the real BG fetcher's high-byte read. Its progress, not
    // an X-based penalty table, determines the additional object wait.
    let safe = p.fetcher.step == FetchStep::Push
        || (p.fetcher.step == FetchStep::High && p.fetcher.phase == 1);
    if safe && p.bg.len != 0 {
        p.waiting_object = None;
        p.object = Some(ObjectFetcher {
            index,
            phase: 1,
            ..Default::default()
        });
    } else {
        super::background::tick(core, p);
    }
    true
}

fn next(core: &mut Context<'_>, cursor: i16) -> Option<u8> {
    let cache = &mut *core.oam;
    let mut remaining = cache.selected & !cache.fetched;
    let mut candidate = None;
    while remaining != 0 {
        let index = u8::try_from(remaining.trailing_zeros()).unwrap_or_default();
        remaining &= remaining - 1;
        let x = cache.bytes[usize::from(index) * 4 + 1];
        let left = i16::from(x) - 8;
        if left < cursor || x >= 168 {
            cache.fetched |= 1 << index;
            continue;
        }
        if left == cursor && candidate.is_none_or(|old| index < old) {
            candidate = Some(index);
        }
    }
    candidate
}

fn read_plane(core: &Context<'_>, object: ObjectFetcher, high: bool) -> u8 {
    let offset = usize::from(object.index) * 4;
    let height = if io(core, 0x40) & 4 != 0 { 16 } else { 8 };
    let mut row = io(core, 0x44)
        .wrapping_add(16)
        .wrapping_sub(core.oam.bytes[offset])
        & (height - 1);
    if object.flags & 0x40 != 0 {
        row = height - 1 - row;
    }
    let tile = if height == 16 {
        object.tile & !1
    } else {
        object.tile
    };
    let bank = if core.cgb && object.flags & 8 != 0 {
        0x2000
    } else {
        0
    };
    core.memory.vram[bank + usize::from(tile) * 16 + usize::from(row) * 2 + usize::from(high)]
}

fn merge(core: &Context<'_>, p: &mut PixelPipeline, object: ObjectFetcher) {
    while p.objects.len < 8 {
        p.objects.push(Pixel::default());
    }
    let oam_x = core.oam.bytes[usize::from(object.index) * 4 + 1];
    for x in 0..8_u8 {
        let bit = if object.flags & 0x20 != 0 { x } else { 7 - x };
        let colour = ((object.low >> bit) & 1) | (((object.high >> bit) & 1) << 1);
        if colour == 0 {
            continue;
        }
        let slot = usize::from((p.objects.head + x) & 15);
        let old = p.objects.pixels[slot];
        let higher = if core.cgb {
            object.index < old.index
        } else {
            (oam_x, object.index) < (old.oam_x, old.index)
        };
        if old.colour == 0 || higher {
            p.objects.pixels[slot] = Pixel {
                colour,
                palette: if core.cgb {
                    object.flags & 7
                } else {
                    (object.flags >> 4) & 1
                },
                flags: object.flags,
                index: object.index,
                oam_x,
            };
        }
    }
}
