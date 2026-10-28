use super::state::Pixel;
use super::{Context, PixelPipeline, io};
use crate::game_boy::emulator::gpu::LineRenderer;

pub(super) fn emit(core: &mut Context<'_>, p: &mut PixelPipeline, mut bg: Pixel, obj: Pixel) {
    let lcd = io(core, 0x40);
    if !core.cgb && lcd & 1 == 0 {
        bg.colour = 0;
    }
    let obj_visible = lcd & 2 != 0
        && obj.colour != 0
        && (bg.colour == 0
            || (core.cgb && lcd & 1 == 0)
            || (obj.flags & 0x80 == 0 && (!core.cgb || bg.flags & 0x80 == 0)));
    let (colour, mono) = match core.renderer {
        LineRenderer::Cgb => {
            let colour = if obj_visible {
                core.palettes.cgb_obj[usize::from(obj.palette) * 4 + usize::from(obj.colour)]
            } else {
                core.palettes.cgb_bg[usize::from(bg.palette) * 4 + usize::from(bg.colour)]
            };
            (colour, 0)
        }
        LineRenderer::Gb | LineRenderer::Sgb => {
            if obj_visible {
                let index = usize::from(obj.palette) * 4 + usize::from(obj.colour);
                (
                    core.palettes.translated_obj[index],
                    core.palettes.sgb_translation_obj[index],
                )
            } else {
                let index = usize::from(bg.colour);
                (
                    core.palettes.translated_bg[index],
                    core.palettes.sgb_translation_bg[index],
                )
            }
        }
    };
    let line = io(core, 0x44);
    if core.renderer == LineRenderer::Sgb {
        if let Some(pixel) = core
            .sgb
            .mono_data
            .get_mut(usize::from(line) * 160 + usize::from(p.x))
        {
            *pixel = mono;
        }
    } else {
        core.video.write_ppu_pixel(line, p.x, colour);
    }
    if p.window_active {
        p.window_used = true;
    }
    p.x += 1;
}
