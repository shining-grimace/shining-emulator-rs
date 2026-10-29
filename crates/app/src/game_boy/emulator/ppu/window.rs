use super::state::Pixel;
use super::{Context, PixelPipeline, background, io};

pub(super) fn tick(core: &Context<'_>, p: &mut PixelPipeline) -> bool {
    let lcd = io(core, 0x40);
    let enabled = lcd & 0x20 != 0 && (core.cgb || lcd & 1 != 0);
    // Finish the visible window tile. Any prefetched window row is discarded
    // when returning to the background on a tile boundary (no SCX fine shift).
    if p.window_active && !enabled && p.window_pixels == 0 && p.window_used {
        p.window_active = false;
        p.window_glitch_x = None;
        p.fine = 0;
        background::restart(p, p.x / 8);
        return true;
    }
    let wx = io(core, 0x4b);
    if p.window_active {
        if p.window_wx != Some(wx) {
            p.window_wx = Some(wx);
            p.window_glitch_x = Some(wx);
        }
        if p.dummy == 0
            && p.fine == 0
            && p.window_glitch_x == Some(p.x.saturating_add(7))
            && p.bg.len < 16
        {
            p.window_glitch_x = None;
            // The extra pixel has neither a palette attribute nor BG priority.
            p.bg.head = p.bg.head.wrapping_sub(1) & 15;
            p.bg.pixels[usize::from(p.bg.head)] = Pixel {
                index: u8::MAX,
                ..Default::default()
            };
            p.bg.len += 1;
        }
        return false;
    }
    let start = i16::from(wx) - 7;
    if enabled
        && p.window_y_triggered
        && wx < 167
        && p.bg.len != 0
        && p.cursor() == start
        && (p.dummy != 0 || p.fine == 0)
        && p.window_wx != Some(wx)
    {
        if p.window_used {
            p.window_line = p.window_line.wrapping_add(1);
        }
        p.window_active = true;
        p.window_wx = Some(wx);
        p.window_pixels = 7_u8.saturating_sub(wx);
        let fractional_scroll = p.fine != 0;
        p.fine = 0;
        if wx == 0 && fractional_scroll {
            // WX=0 with fractional scrolling skips one of the startup dots.
            p.dummy = p.dummy.saturating_sub(1);
            p.window_pixels = 6;
        }
        background::restart(p, 0);
        return true;
    }
    false
}
