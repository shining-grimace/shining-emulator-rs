use super::*;

#[test]
fn transfer_finishes_after_pixels_and_fine_scroll_are_consumed() {
    for fine in 0..8 {
        let mut core = core();
        core.memory.io_ports[0x43] = fine;
        assert_eq!(finish(&mut core), 172 + usize::from(fine));
        assert_eq!(core.gpu_timing.pixel_pipeline.x, 160);
    }
}

#[test]
fn palette_writes_change_only_pixels_not_yet_emitted() {
    let mut core = core();
    until_x(&mut core, 40);
    crate::game_boy::emulator::bus::write8(&mut core, 0xff47, 0);
    let new_colour = core.palettes.translated_bg[1];
    finish(&mut core);
    let row = row(&core);
    assert!(row[..40].iter().all(|p| *p == rgb(RED)));
    assert!(row[40..].iter().all(|p| *p == rgb(new_colour)));
}

#[test]
fn tile_address_selection_is_sampled_separately_for_each_bitplane() {
    let mut core = core();
    core.memory.vram[0x1010..0x1020]
        .as_chunks_mut::<2>()
        .0
        .iter_mut()
        .for_each(|p| p.copy_from_slice(&[0, 0xff]));
    dots(&mut core, 8); // Second fetch has read low, but not high.
    crate::game_boy::emulator::bus::write8(&mut core, 0xff40, 0x81);
    finish(&mut core);
    let row = row(&core);
    assert!(row[..8].iter().all(|p| *p == rgb(BLUE)));
    assert!(row[8..].iter().all(|p| *p == rgb(GREEN)));
}

#[test]
fn dmg_scy_changes_can_mix_rows_but_cgb_latches_the_tile_row() {
    for cgb in [false, true] {
        let mut core = core();
        core.rom.properties.cgb_flag = cgb;
        if cgb {
            core.line_renderer = LineRenderer::Cgb;
        }
        core.memory.vram[18..20].copy_from_slice(&[0, 0xff]);
        dots(&mut core, 8);
        crate::game_boy::emulator::bus::write8(&mut core, 0xff42, 1);
        finish(&mut core);
        let row = row(&core);
        assert!(
            row[..8]
                .iter()
                .all(|p| *p == rgb(if cgb { RED } else { BLUE }))
        );
        assert!(row[8..].iter().all(|p| *p == rgb(GREEN)));
    }
}

#[test]
fn cgb_fetches_attributes_and_tile_pixels_from_the_selected_vram_bank() {
    let mut core = core();
    core.rom.properties.cgb_flag = true;
    core.line_renderer = LineRenderer::Cgb;
    core.memory.vram[0x3800..0x3c00].fill(8 | 0x20 | 0x40 | 3);
    core.memory.vram[0x201e..0x2020].copy_from_slice(&[0x80, 0]);
    core.palettes.cgb_bg[3 * 4 + 1] = BLUE;
    core.palettes.cgb_bg[3 * 4] = WHITE;
    finish(&mut core);
    assert_eq!(row(&core)[7], rgb(BLUE));
    assert_eq!(row(&core)[0], rgb(WHITE));
}

#[test]
fn moving_wx_mid_line_restarts_the_fetcher_at_the_new_position() {
    let mut core = core();
    core.memory.io_ports[0x40] = 0xf1;
    core.memory.io_ports[0x4b] = 87;
    until_x(&mut core, 32);
    crate::game_boy::emulator::bus::write8(&mut core, 0xff4b, 39);
    finish(&mut core);
    let row = row(&core);
    assert!(row[..32].iter().all(|p| *p == rgb(RED)));
    assert!(row[32..].iter().all(|p| *p == rgb(GREEN)));
}

#[test]
fn hidden_window_lines_do_not_advance_its_internal_y_counter() {
    let mut core = core();
    core.memory.io_ports[0x40] = 0xf1;
    core.memory.io_ports[0x4b] = 7;
    assert_eq!(finish(&mut core), 178);
    assert_eq!(core.gpu_timing.pixel_pipeline.window_line, 1);
    core.memory.io_ports[0x44] = 1;
    core.memory.io_ports[0x40] = 0x91;
    begin(&mut core);
    finish(&mut core);
    assert_eq!(core.gpu_timing.pixel_pipeline.window_line, 1);
    core.memory.io_ports[0x44] = 2;
    core.memory.io_ports[0x40] = 0xf1;
    begin(&mut core);
    finish(&mut core);
    assert_eq!(core.gpu_timing.pixel_pipeline.window_line, 2);
}

#[test]
fn sgb_receives_pixels_through_the_same_dot_pipeline() {
    let mut core = core();
    core.line_renderer = LineRenderer::Sgb;
    finish(&mut core);
    assert!(core.sgb.mono_data[..160].iter().all(|p| *p == 1));
}
