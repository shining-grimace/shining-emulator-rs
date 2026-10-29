use super::*;
use crate::game_boy::emulator::bus;

fn window_core() -> GameBoyCore {
    let mut result = core();
    result.rom.properties.cgb_flag = true;
    result.line_renderer = LineRenderer::Cgb;
    result.memory.io_ports[0x40] = 0xf1;
    result.memory.io_ports[0x4b] = 7;
    result
}

#[test]
fn disabling_window_finishes_its_tile_and_resumes_without_scx_fine_scroll() {
    let mut core = window_core();
    core.memory.io_ports[0x43] = 3;
    until_x(&mut core, 3);
    bus::write8(&mut core, 0xff40, 0xd1);
    finish(&mut core);
    let pixels = row(&core);
    assert!(pixels[..8].iter().all(|p| *p == rgb(GREEN)));
    assert!(pixels[8..].iter().all(|p| *p == rgb(RED)));
    assert_eq!(core.gpu_timing.pixel_pipeline.window_line, 1);
}

#[test]
fn reactivating_window_with_a_new_wx_uses_the_next_window_row() {
    let mut core = window_core();
    core.memory.vram[34..36].copy_from_slice(&[0xff, 0xff]);
    until_x(&mut core, 3);
    bus::write8(&mut core, 0xff40, 0xd1);
    until_x(&mut core, 16);
    bus::write8(&mut core, 0xff4b, 31);
    bus::write8(&mut core, 0xff40, 0xf1);
    finish(&mut core);
    let pixels = row(&core);
    assert!(pixels[..8].iter().all(|p| *p == rgb(GREEN)));
    assert!(pixels[8..24].iter().all(|p| *p == rgb(RED)));
    assert!(pixels[24..].iter().all(|p| *p == rgb(BLUE)));
    assert_eq!(core.gpu_timing.pixel_pipeline.window_line, 2);
}

#[test]
fn changing_wx_after_activation_inserts_one_low_priority_colour_zero_pixel() {
    let mut core = window_core();
    until_x(&mut core, 16);
    bus::write8(&mut core, 0xff4b, 39);
    finish(&mut core);
    let pixels = row(&core);
    assert!(pixels[..32].iter().all(|p| *p == rgb(GREEN)));
    assert_eq!(pixels[32], rgb(WHITE));
    assert!(pixels[33..].iter().all(|p| *p == rgb(GREEN)));
}

#[test]
fn off_left_window_clipping_does_not_repeat_background_fine_scroll() {
    for wx in 1..7 {
        for fine in 0..8 {
            let mut core = window_core();
            core.memory.io_ports[0x4b] = wx;
            core.memory.io_ports[0x43] = fine;
            assert_eq!(finish(&mut core), 178, "WX={wx} SCX={fine}");
            assert!(row(&core).iter().all(|p| *p == rgb(GREEN)));
        }
    }
}

#[test]
fn wx_zero_with_fractional_scrolling_shortens_transfer_by_one_dot() {
    for fine in 0..8 {
        let mut core = window_core();
        core.memory.io_ports[0x4b] = 0;
        core.memory.io_ports[0x43] = fine;
        dots(&mut core, 1); // Fine scrolling is sampled at scanline start.
        bus::write8(&mut core, 0xff43, (fine + 1) & 7);
        assert_eq!(1 + finish(&mut core), if fine == 0 { 178 } else { 177 });
    }
}

#[test]
fn inserted_pixel_does_not_end_the_current_window_tile_one_dot_early() {
    let mut core = window_core();
    until_x(&mut core, 8);
    bus::write8(&mut core, 0xff4b, 17);
    until_x(&mut core, 12);
    bus::write8(&mut core, 0xff40, 0xd1);
    finish(&mut core);
    let pixels = row(&core);
    assert_eq!(pixels[10], rgb(WHITE));
    assert!(pixels[11..17].iter().all(|p| *p == rgb(GREEN)));
    assert!(pixels[17..].iter().all(|p| *p == rgb(RED)));
}
