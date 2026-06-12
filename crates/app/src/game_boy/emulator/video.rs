use bevy::prelude::warn;

use crate::dimensions::{
    GAME_BOY_FRAME_BUFFER_BYTES, GAME_BOY_RGB_CHANNELS, GAME_BOY_SCREEN_WIDTH,
};
use crate::game_boy::frame_buffer::GameBoyFrameRing;

const RANDOM_GREYSCALE_SHADES: [u8; 4] = [0xff, 0xaa, 0x55, 0x00];

#[derive(Debug)]
pub(crate) struct VideoFrameAssembler {
    pixels: Box<[u8]>,
    frame_in_progress: bool,
}

impl Default for VideoFrameAssembler {
    fn default() -> Self {
        Self {
            pixels: vec![0; GAME_BOY_FRAME_BUFFER_BYTES].into_boxed_slice(),
            frame_in_progress: true,
        }
    }
}

impl VideoFrameAssembler {
    pub(crate) fn reset_for_rom_load(&mut self) {
        self.begin_frame();
    }

    pub(crate) fn begin_frame(&mut self) {
        self.frame_in_progress = true;
    }

    pub(crate) fn write_random_greyscale_line(&mut self, line: u8) {
        if !self.frame_in_progress {
            return;
        }

        let line = usize::from(line);
        let line_start = line * GAME_BOY_SCREEN_WIDTH * GAME_BOY_RGB_CHANNELS;
        let line_end = line_start + GAME_BOY_SCREEN_WIDTH * GAME_BOY_RGB_CHANNELS;
        let Some(row) = self.pixels.get_mut(line_start..line_end) else {
            return;
        };

        for pixel in row.chunks_exact_mut(GAME_BOY_RGB_CHANNELS) {
            let shade = RANDOM_GREYSCALE_SHADES[fastrand::usize(..RANDOM_GREYSCALE_SHADES.len())];
            pixel.fill(shade);
        }
    }

    pub(crate) fn publish_frame(&mut self, frames: &mut GameBoyFrameRing) {
        if !self.frame_in_progress {
            return;
        }

        let Some(mut frame) = frames.borrow_next_write_frame() else {
            warn!("Game Boy frame ring is unavailable");
            return;
        };

        let destination = frame.pixels_mut();
        if destination.len() != self.pixels.len() {
            warn!("Game Boy frame ring buffer does not match assembled video frame size");
            return;
        }

        destination.copy_from_slice(&self.pixels);
        frame.publish();
        self.frame_in_progress = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_greyscale_line_writes_rgb_from_two_bit_shades() {
        let mut assembler = VideoFrameAssembler::default();

        assembler.write_random_greyscale_line(0);

        for pixel in assembler.pixels[0..GAME_BOY_SCREEN_WIDTH * GAME_BOY_RGB_CHANNELS]
            .chunks_exact(GAME_BOY_RGB_CHANNELS)
        {
            assert_eq!(pixel[0], pixel[1]);
            assert_eq!(pixel[1], pixel[2]);
            assert!(RANDOM_GREYSCALE_SHADES.contains(&pixel[0]));
        }
    }

    #[test]
    fn publishing_marks_frame_complete() {
        let mut assembler = VideoFrameAssembler::default();
        let mut frames = GameBoyFrameRing::default();

        assembler.publish_frame(&mut frames);

        assert!(frames.latest_written_frame().is_some());
        assert!(!assembler.frame_in_progress);
    }
}
