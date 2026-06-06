use bevy::prelude::*;

use crate::dimensions::GAME_BOY_RGB_CHANNELS;
use crate::game_boy::frame_buffer::GameBoyFrameRing;

pub(super) fn write_placeholder_game_boy_frame(mut frames: ResMut<GameBoyFrameRing>) {
    let Some(mut frame) = frames.borrow_next_write_frame() else {
        warn!("Game Boy frame ring is unavailable");
        return;
    };

    fill_with_random_greyscale(frame.pixels_mut());
    frame.publish();
}

fn fill_with_random_greyscale(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(GAME_BOY_RGB_CHANNELS) {
        let shade = fastrand::u8(..);
        pixel.fill(shade);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimensions::GAME_BOY_FRAME_BUFFER_BYTES;

    #[test]
    fn generated_placeholder_frame_is_rgb_greyscale() {
        let mut pixels = vec![0; GAME_BOY_FRAME_BUFFER_BYTES];

        fill_with_random_greyscale(&mut pixels);

        for pixel in pixels.chunks_exact(GAME_BOY_RGB_CHANNELS) {
            assert_eq!(pixel[0], pixel[1]);
            assert_eq!(pixel[1], pixel[2]);
        }
    }
}
