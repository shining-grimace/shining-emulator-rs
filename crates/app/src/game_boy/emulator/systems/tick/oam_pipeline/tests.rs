use super::*;

fn sprite_core() -> GameBoyCore {
    let mut core = GameBoyCore {
        gpu_mode: GpuMode::ScanOam,
        ..Default::default()
    };
    core.memory.io_ports[LCDC_IO_INDEX] = 0x93;
    core.memory.oam[..4].copy_from_slice(&[16, 8, 3, 4]);
    core
}

#[test]
fn dma_during_mode_two_hides_objects_without_rewriting_oam() {
    let mut core = sprite_core();
    observe(&mut core, 0, 80, true);
    assert_eq!(line_objects(&core), Vec::new());
    assert_eq!(&core.memory.oam[..4], &[16, 8, 3, 4]);
}

#[test]
fn partial_mode_two_dma_does_not_discard_objects_already_selected() {
    let mut core = sprite_core();
    core.memory.oam[4..8].copy_from_slice(&[16, 16, 5, 6]);
    observe(&mut core, 0, 2, false);
    observe(&mut core, 2, 80, true);
    let objects = line_objects(&core);
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].index, 0);
}

#[test]
fn mode_three_latches_dma_word_at_fetch_without_reselecting_coordinates() {
    let mut core = sprite_core();
    observe(&mut core, 0, 80, false);
    core.gpu_mode = GpuMode::ScanVram;
    core.memory.oam[..4].fill(0); // DMA has changed OAM since selection.
    core.dma.oam.oam_word = [9, 0x20];
    observe(&mut core, 0, 11, true);
    assert_eq!(core.gpu_timing.oam_scanline.bytes[2], 3);
    observe(&mut core, 11, 12, true);
    assert_eq!(&core.gpu_timing.oam_scanline.bytes[..4], &[16, 8, 9, 0x20]);
    core.dma.oam.oam_word = [10, 0x40];
    observe(&mut core, 12, 172, true);
    assert_eq!(&core.gpu_timing.oam_scanline.bytes[..4], &[16, 8, 9, 0x20]);
    assert_eq!(line_objects(&core).len(), 1);
}

#[test]
fn oam_latches_round_trip_and_reject_invalid_saved_data() {
    let mut core = sprite_core();
    observe(&mut core, 0, 80, false);
    let text = serde_json::to_string(&core.gpu_timing.oam_scanline).unwrap();
    let line: OamScanline = serde_json::from_str(&text).unwrap();
    assert_eq!(line.bytes, core.gpu_timing.oam_scanline.bytes);
    assert_eq!(line.selected, 1);
    let mut value = serde_json::to_value(line).unwrap();
    value["bytes"] = serde_json::json!([]);
    assert!(serde_json::from_value::<OamScanline>(value).is_err());
}

#[test]
fn machine_cycle_uses_oam_ownership_before_startup_and_completion() {
    let mut core = sprite_core();
    core.dma.oam.pending_source_high = Some(0x80);
    crate::game_boy::emulator::bus::begin_deferred_oam_dma(&mut core);
    let mut frames = crate::game_boy::frame_buffer::GameBoyFrameRing::default();
    super::super::step_machine_cycle(&mut core, &mut frames);
    assert!(core.dma.oam.active);
    assert_eq!(line_objects(&core).len(), 1); // Startup cycle was still free.

    let mut core = sprite_core();
    core.dma.oam.active = true;
    core.dma.oam.next_index = 159;
    super::super::step_machine_cycle(&mut core, &mut frames);
    assert!(!core.dma.oam.active);
    assert_eq!(line_objects(&core), Vec::new()); // Final copy cycle still owned OAM.
}

fn first_pixel(core: &mut GameBoyCore) -> [u8; 3] {
    let mut frames = crate::game_boy::frame_buffer::GameBoyFrameRing::default();
    core.video_frame.begin_frame();
    super::super::write_visible_line(core, 0);
    core.video_frame.publish_frame(&mut frames);
    frames.latest_written_frame().unwrap().pixels()[..3]
        .try_into()
        .unwrap()
}

#[test]
fn renderer_uses_hidden_mode_two_objects_and_corrupted_mode_three_attributes() {
    let mut core = sprite_core();
    core.memory.tile_set[3 * 64..3 * 64 + 8].fill(1);
    core.memory.tile_set[5 * 64..5 * 64 + 8].fill(2);
    core.palettes.translated_obj[1] = 0xffaa_0000;
    core.palettes.translated_obj[2] = 0xff00_00aa;
    observe(&mut core, 0, 80, true);
    assert_eq!(first_pixel(&mut core), [0, 0, 0]);

    observe(&mut core, 0, 80, false);
    core.gpu_mode = GpuMode::ScanVram;
    core.memory.oam[..4].fill(0);
    core.dma.oam.oam_word = [5, 0];
    observe(&mut core, 0, 172, true);
    assert_eq!(first_pixel(&mut core), [0, 0, 0xaa]);
}
