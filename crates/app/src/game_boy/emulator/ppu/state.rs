use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub(crate) struct Pixel {
    pub(crate) colour: u8,
    pub(crate) palette: u8,
    pub(crate) flags: u8,
    pub(crate) index: u8,
    pub(crate) oam_x: u8,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub(crate) struct Fifo {
    pub(crate) pixels: [Pixel; 16],
    pub(crate) head: u8,
    pub(crate) len: u8,
}

impl Fifo {
    pub(crate) fn pop(&mut self) -> Option<Pixel> {
        if self.len == 0 {
            return None;
        }
        let pixel = self.pixels[usize::from(self.head)];
        self.head = (self.head + 1) & 15;
        self.len -= 1;
        Some(pixel)
    }

    pub(crate) fn push(&mut self, pixel: Pixel) {
        if self.len < 16 {
            self.pixels[usize::from((self.head + self.len) & 15)] = pixel;
            self.len += 1;
        }
    }

    pub(crate) fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }
    pub(crate) fn valid(&self) -> bool {
        self.head < 16
            && self.len <= 16
            && self.pixels.iter().all(|p| p.colour < 4 && p.palette < 8)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) enum FetchStep {
    #[default]
    Tile,
    Low,
    High,
    Push,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct BackgroundFetcher {
    pub(crate) step: FetchStep,
    pub(crate) phase: u8,
    pub(crate) tile_x: u8,
    pub(crate) tile: u8,
    pub(crate) attributes: u8,
    pub(crate) row: u8,
    pub(crate) address: Option<u16>,
    pub(crate) unsigned_tiles: bool,
    pub(crate) low: u8,
    pub(crate) high: u8,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub(crate) struct ObjectFetcher {
    pub(crate) index: u8,
    pub(crate) phase: u8,
    pub(crate) tile: u8,
    pub(crate) flags: u8,
    pub(crate) low: u8,
    pub(crate) high: u8,
}

/// CPU CGB E bus latches, retained across scanlines and FIFO restarts.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct CgbFetchLatches {
    pub(crate) retained_data: u8,
    pub(crate) tile_select_change: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
// These independent hardware latches must survive mid-transfer save states.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct PixelPipeline {
    #[serde(default)]
    pub(crate) cgb_fetch: CgbFetchLatches,
    pub(crate) active: bool,
    pub(crate) x: u8,
    pub(crate) dummy: u8,
    pub(crate) fine: u8,
    pub(crate) bg: Fifo,
    pub(crate) objects: Fifo,
    pub(crate) fetcher: BackgroundFetcher,
    pub(crate) object: Option<ObjectFetcher>,
    pub(crate) waiting_object: Option<u8>,
    pub(crate) window_y_triggered: bool,
    pub(crate) window_line: u8,
    pub(crate) window_active: bool,
    pub(crate) window_used: bool,
    #[serde(default)]
    pub(crate) window_pixels: u8,
    #[serde(default)]
    pub(crate) window_wx: Option<u8>,
    #[serde(default)]
    pub(crate) window_glitch_x: Option<u8>,
    pub(crate) dummy_tile: bool,
}

impl PixelPipeline {
    pub(crate) fn cursor(&self) -> i16 {
        if self.dummy > 0 {
            -i16::from(self.dummy)
        } else {
            i16::from(self.x)
        }
    }
    pub(crate) fn valid(&self) -> bool {
        self.x <= 160
            && self.dummy <= 8
            && self.fine <= 7
            && self.bg.valid()
            && self.objects.valid()
            && self.fetcher.phase < 2
            && self.fetcher.tile_x < 32
            && self.fetcher.row < 8
            && self.fetcher.address.is_none_or(|a| {
                a < if self.fetcher.step == FetchStep::Tile {
                    0x2000
                } else {
                    0x4000
                }
            })
            && self.window_pixels < 8
            && self.object.is_none_or(|o| o.index < 40 && o.phase < 6)
            && self.waiting_object.is_none_or(|i| i < 40)
    }
}
