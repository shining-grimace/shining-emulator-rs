use super::*;

#[test]
fn sprite_stalls_follow_background_fetcher_progress() {
    for (x, duration) in [(0, 183), (1, 182), (8, 183), (13, 178), (16, 183)] {
        let mut core = core();
        sprite(&mut core, 0, x, 1, 0);
        assert_eq!(finish(&mut core), duration, "OAM X={x}");
    }
    let mut core = core();
    sprite(&mut core, 0, 8, 1, 0);
    sprite(&mut core, 1, 8, 2, 0);
    assert_eq!(finish(&mut core), 189); // The second fetch doesn't repeat the BG wait.
}

#[test]
fn object_fifo_preserves_dmg_x_priority_and_cgb_oam_priority() {
    for cgb in [false, true] {
        let mut core = core();
        core.rom.properties.cgb_flag = cgb;
        if cgb {
            core.line_renderer = LineRenderer::Cgb;
        }
        sprite(&mut core, 0, 10, 1, 0);
        sprite(&mut core, 1, 8, 2, 0);
        finish(&mut core);
        let row = row(&core);
        assert_eq!(row[0], rgb(GREEN));
        assert_eq!(row[2], rgb(if cgb { BLUE } else { GREEN }));
    }
}

#[test]
fn background_priority_masks_the_winning_object_without_revealing_a_lower_one() {
    let mut core = core();
    sprite(&mut core, 0, 8, 1, 0x80);
    sprite(&mut core, 1, 8, 2, 0);
    finish(&mut core);
    assert_eq!(row(&core)[0], rgb(RED));
}

#[test]
fn dma_word_is_read_by_the_object_fetcher_and_survives_later_oam_changes() {
    let mut core = core();
    sprite(&mut core, 0, 8, 1, 0);
    begin(&mut core);
    core.memory.oam[..4].fill(0);
    core.dma.oam.oam_word = [2, 0];
    for _ in 0..2000 {
        if tick(&mut core, true) {
            break;
        }
    }
    assert_eq!(core.gpu_timing.pixel_pipeline.x, 160);
    assert_eq!(row(&core)[0], rgb(GREEN));
    assert_eq!(core.gpu_timing.oam_scanline.bytes[2], 2);
}

#[test]
fn dma_hidden_mode_two_objects_do_not_reappear_during_rendering() {
    let mut core = core();
    sprite(&mut core, 0, 8, 2, 0);
    core.gpu_timing.oam_scanline.valid = true;
    core.gpu_timing.oam_scanline.selected = 0;
    finish(&mut core);
    assert_eq!(row(&core)[0], rgb(RED));
}

#[test]
fn disabling_objects_during_a_fetch_cancels_without_waiting_a_full_sprite() {
    let mut core = core();
    sprite(&mut core, 0, 8, 1, 0);
    let mut elapsed = 0;
    while core
        .gpu_timing
        .pixel_pipeline
        .object
        .is_none_or(|o| o.phase != 3)
    {
        tick(&mut core, false);
        elapsed += 1;
        assert!(elapsed < 100);
    }
    crate::game_boy::emulator::bus::write8(&mut core, 0xff40, 0x91);
    let duration = elapsed + finish(&mut core);
    assert!((172..183).contains(&duration));
    assert_eq!(row(&core)[0], rgb(RED));
}
