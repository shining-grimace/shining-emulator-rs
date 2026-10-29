use super::*;

mod background;
mod cgb_e;
mod objects;
mod serialization;
mod window;

const WHITE: u32 = 0xffff_ffff;
const RED: u32 = 0xffaa_0000;
const GREEN: u32 = 0xff00_aa00;
const BLUE: u32 = 0xff00_00aa;

fn core() -> GameBoyCore {
    let mut core = GameBoyCore::default();
    core.memory.io_ports[0x40] = 0x91;
    core.memory.vram[0x1800..0x1c00].fill(1);
    core.memory.vram[0x1c00..0x2000].fill(2);
    tile(&mut core, 1, 1);
    tile(&mut core, 2, 2);
    core.palettes.translated_bg = [WHITE, RED, GREEN, BLUE];
    core.palettes.translated_obj = [WHITE, BLUE, GREEN, RED, WHITE, BLUE, GREEN, RED];
    core.palettes.cgb_bg[..4].copy_from_slice(&[WHITE, RED, GREEN, BLUE]);
    core.palettes.cgb_obj[..4].copy_from_slice(&[WHITE, BLUE, GREEN, RED]);
    core.palettes.sgb_translation_bg = [0, 1, 2, 3];
    core.palettes.sgb_translation_obj = [0, 1, 2, 3, 0, 1, 2, 3];
    core.gpu_mode = crate::game_boy::emulator::gpu::GpuMode::ScanVram;
    core
}

fn tile(core: &mut GameBoyCore, number: usize, colour: u8) {
    for row in 0..8 {
        core.memory.write_vram(
            0,
            u16::try_from(number * 16 + row * 2).unwrap(),
            if colour & 1 == 0 { 0 } else { 0xff },
        );
        core.memory.write_vram(
            0,
            u16::try_from(number * 16 + row * 2 + 1).unwrap(),
            if colour & 2 == 0 { 0 } else { 0xff },
        );
    }
}

fn dots(core: &mut GameBoyCore, count: usize) {
    for _ in 0..count {
        tick(core, false);
    }
}

fn until_x(core: &mut GameBoyCore, x: u8) {
    for _ in 0..2000 {
        if core.gpu_timing.pixel_pipeline.x >= x {
            return;
        }
        tick(core, false);
    }
    panic!("LCD did not reach pixel {x}");
}

fn finish(core: &mut GameBoyCore) -> usize {
    for elapsed in 1..2000 {
        if tick(core, false) {
            return elapsed;
        }
    }
    panic!("pixel transfer did not finish");
}

fn rgb(colour: u32) -> [u8; 3] {
    let [_, r, g, b] = colour.to_be_bytes();
    [r, g, b]
}

fn row(core: &GameBoyCore) -> Vec<[u8; 3]> {
    core.video_frame.snapshot().pixels[..480]
        .as_chunks::<3>()
        .0
        .to_vec()
}

fn sprite(core: &mut GameBoyCore, index: usize, x: u8, tile: u8, flags: u8) {
    core.memory.io_ports[0x40] |= 2;
    core.memory.oam[index * 4..index * 4 + 4].copy_from_slice(&[16, x, tile, flags]);
}
