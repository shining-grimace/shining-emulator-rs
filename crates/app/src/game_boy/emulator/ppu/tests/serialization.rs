use super::*;
use crate::game_boy::emulator::{apply_save_state, encode_save_state};

#[test]
fn mid_fetch_save_restores_fifo_register_latches_and_already_emitted_pixels() {
    let mut original = core();
    sprite(&mut original, 0, 48, 2, 0);
    original.memory.io_ports[0x40] |= 0x60;
    original.memory.io_ports[0x4b] = 87;
    for _ in 0..2000 {
        tick(&mut original, false);
        if original
            .gpu_timing
            .pixel_pipeline
            .object
            .is_some_and(|o| o.phase == 3)
        {
            break;
        }
    }
    assert_eq!(original.gpu_timing.pixel_pipeline.x, 40);
    let bytes = encode_save_state(&original).unwrap();
    let mut restored = core();
    apply_save_state(&mut restored, &bytes).unwrap();
    assert_eq!(finish(&mut original), finish(&mut restored));
    assert_eq!(row(&original), row(&restored));
    assert_eq!(
        serde_json::to_value(original.gpu_timing.pixel_pipeline).unwrap(),
        serde_json::to_value(restored.gpu_timing.pixel_pipeline).unwrap()
    );
}

#[test]
fn legacy_states_without_fifo_or_framebuffer_still_load() {
    let original = core();
    let mut value: serde_json::Value =
        serde_json::from_slice(&encode_save_state(&original).unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("video_frame");
    value["gpu_timing"]
        .as_object_mut()
        .unwrap()
        .remove("pixel_pipeline");
    apply_save_state(&mut core(), &serde_json::to_vec(&value).unwrap()).unwrap();
}

#[test]
fn malformed_fifo_and_framebuffer_are_rejected_before_mutating_the_core() {
    let original = core();
    let value: serde_json::Value =
        serde_json::from_slice(&encode_save_state(&original).unwrap()).unwrap();
    for field in ["fifo", "framebuffer"] {
        let mut changed = value.clone();
        if field == "fifo" {
            changed["gpu_timing"]["pixel_pipeline"]["bg"]["len"] = 17.into();
        } else {
            changed["video_frame"]["pixels"] = serde_json::json!([]);
        }
        let mut target = core();
        target.cpu_registers.a = 0x77;
        assert!(apply_save_state(&mut target, &serde_json::to_vec(&changed).unwrap()).is_err());
        assert_eq!(target.cpu_registers.a, 0x77);
    }
}
