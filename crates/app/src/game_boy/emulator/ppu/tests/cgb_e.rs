use super::*;
use crate::game_boy::emulator::{apply_save_state, bus, encode_save_state};

fn cgb() -> GameBoyCore {
    let mut result = core();
    result.rom.properties.cgb_flag = true;
    result.line_renderer = LineRenderer::Cgb;
    result
}

#[test]
fn e_falling_tile_select_edge_uses_tile_index_on_either_plane() {
    for high in [false, true] {
        let mut core = cgb();
        dots(&mut core, if high { 9 } else { 7 });
        bus::write8(&mut core, 0xff40, 0x81);
        dots(&mut core, 1);
        let f = core.gpu_timing.pixel_pipeline.fetcher;
        assert_eq!(if high { f.high } else { f.low }, 1);
        finish(&mut core);
        let pixels = row(&core);
        if high {
            assert!(pixels[..7].iter().all(|p| *p == rgb(RED)));
            assert_eq!(pixels[7], rgb(BLUE));
        } else {
            assert!(pixels[..7].iter().all(|p| *p == rgb(WHITE)));
            assert_eq!(pixels[7], rgb(RED));
        }
    }
}

#[test]
fn tiles_shared_by_both_addressing_modes_do_not_get_the_falling_edge_glitch() {
    let mut core = cgb();
    core.memory.vram[0x1800..0x1c00].fill(0x81);
    tile(&mut core, 0x81, 2);
    dots(&mut core, 9);
    bus::write8(&mut core, 0xff40, 0x81);
    finish(&mut core);
    assert!(row(&core).iter().all(|p| *p == rgb(GREEN)));
}

#[test]
fn rising_edge_reuses_the_previous_high_plane_and_preserves_it_through_a_save() {
    let mut core = cgb();
    core.memory.io_ports[0x40] = 0x81;
    core.memory.vram[0x1011] = 0x5a;
    dots(&mut core, 7);
    bus::write8(&mut core, 0xff40, 0x91);
    let saved = encode_save_state(&core).unwrap();
    let mut restored = cgb();
    apply_save_state(&mut restored, &saved).unwrap();
    for candidate in [&mut core, &mut restored] {
        dots(candidate, 1);
        assert_eq!(candidate.gpu_timing.pixel_pipeline.fetcher.low, 0x5a);
        finish(candidate);
        let pixels = row(candidate);
        for (x, pixel) in pixels[..8].iter().enumerate() {
            assert_eq!(
                *pixel,
                rgb(if 0x5a & (0x80 >> x) != 0 { RED } else { WHITE })
            );
        }
    }
    assert_eq!(row(&core), row(&restored));
}

#[test]
fn sprite_high_plane_drives_the_retained_bus_data() {
    let mut core = cgb();
    core.memory.io_ports[0x40] = 0x81;
    sprite(&mut core, 0, 0, 2, 0);
    core.memory.vram[33] = 0xa5;
    for _ in 0..100 {
        tick(&mut core, false);
        if core.gpu_timing.oam_scanline.fetched & 1 != 0 {
            break;
        }
    }
    assert_eq!(core.gpu_timing.oam_scanline.fetched & 1, 1);
    assert_eq!(core.gpu_timing.pixel_pipeline.cgb_fetch.retained_data, 0xa5);
    bus::write8(&mut core, 0xff40, 0x93);
    dots(&mut core, 1);
    assert_eq!(core.gpu_timing.pixel_pipeline.fetcher.high, 0xa5);
}

#[test]
fn writes_during_a_sprite_stall_do_not_glitch_a_later_background_read() {
    let mut core = cgb();
    core.memory.io_ports[0x40] = 0x81;
    core.memory.vram[0x1011] = 0x3c;
    sprite(&mut core, 0, 0, 2, 0);
    for _ in 0..100 {
        tick(&mut core, false);
        if core
            .gpu_timing
            .pixel_pipeline
            .object
            .is_some_and(|o| o.phase == 3)
        {
            break;
        }
    }
    assert!(core.gpu_timing.pixel_pipeline.object.is_some());
    bus::write8(&mut core, 0xff40, 0x93);
    while core.gpu_timing.pixel_pipeline.object.is_some() {
        dots(&mut core, 1);
    }
    dots(&mut core, 1);
    // The address was latched before the sprite stall, while TILE_SEL was 0.
    assert_eq!(core.gpu_timing.pixel_pipeline.fetcher.high, 0x3c);
}

#[test]
fn e_latches_scroll_and_tile_map_address_on_t1_not_t2() {
    let mut core = cgb();
    core.memory.vram[0x1820..0x1840].fill(2);
    dots(&mut core, 5); // Address setup of the first visible tile.
    bus::write8(&mut core, 0xff42, 8);
    finish(&mut core);
    let pixels = row(&core);
    assert!(pixels[..8].iter().all(|p| *p == rgb(RED)));
    assert!(pixels[8..].iter().all(|p| *p == rgb(GREEN)));
}

#[test]
fn a_transition_on_address_setup_uses_the_new_address_without_a_read_glitch() {
    let mut core = cgb();
    core.memory.vram[0x1010..0x1020].fill(0xff);
    dots(&mut core, 6);
    bus::write8(&mut core, 0xff40, 0x81);
    finish(&mut core);
    assert!(row(&core).iter().all(|p| *p == rgb(BLUE)));
}

#[test]
fn retained_data_survives_scanline_and_frame_boundaries() {
    let mut core = cgb();
    core.memory.vram[17] = 0x96;
    finish(&mut core);
    frame_start(&mut core);
    begin(&mut core);
    assert_eq!(core.gpu_timing.pixel_pipeline.cgb_fetch.retained_data, 0x96);
    assert_eq!(core.gpu_timing.pixel_pipeline.window_line, 0);
    bus::write8(&mut core, 0xff40, 0x81);
    dots(&mut core, 1); // New signed address, but no new high-plane bus data.
    bus::write8(&mut core, 0xff40, 0x91);
    dots(&mut core, 1);
    assert_eq!(core.gpu_timing.pixel_pipeline.fetcher.low, 0x96);
}

#[test]
fn e_unusable_oam_echoes_the_address_nibble_only_when_oam_is_accessible() {
    let mut core = cgb();
    for address in 0xfea0..=0xfeff {
        core.memory_access.oam = true;
        let expected = u8::try_from((address >> 4) & 15).unwrap() * 0x11;
        assert_eq!(bus::read8(&core, address), expected);
        bus::write8(&mut core, address, !expected);
        assert_eq!(bus::read8(&core, address), expected);
        core.memory_access.oam = false;
        assert_eq!(bus::read8(&core, address), 0xff);
    }
}
