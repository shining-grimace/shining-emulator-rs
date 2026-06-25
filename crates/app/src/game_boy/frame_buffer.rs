use bevy::prelude::*;

use crate::dimensions::GAME_BOY_FRAME_BUFFER_BYTES;

pub(crate) const GAME_BOY_FRAME_RING_LEN: usize = 3;
pub(crate) const GAME_BOY_FRAME_RATE_HZ: f64 = 4_194_304.0 / 70_224.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct GameBoyFrameSequence(u64);

#[derive(Debug)]
pub(crate) struct GameBoyFrame {
    sequence: GameBoyFrameSequence,
    pixels: Box<[u8]>,
}

impl GameBoyFrame {
    pub(crate) fn sequence(&self) -> GameBoyFrameSequence {
        self.sequence
    }

    pub(crate) fn pixels(&self) -> &[u8] {
        self.pixels.as_ref()
    }
}

impl Default for GameBoyFrame {
    fn default() -> Self {
        Self {
            sequence: GameBoyFrameSequence::default(),
            pixels: vec![0; GAME_BOY_FRAME_BUFFER_BYTES].into_boxed_slice(),
        }
    }
}

#[derive(Debug, Resource)]
pub(crate) struct GameBoyFrameRing {
    frames: Vec<GameBoyFrame>,
    next_write_index: usize,
    latest_written_index: Option<usize>,
    latest_sequence: GameBoyFrameSequence,
}

impl Default for GameBoyFrameRing {
    fn default() -> Self {
        let mut frames = Vec::with_capacity(GAME_BOY_FRAME_RING_LEN);
        for _ in 0..GAME_BOY_FRAME_RING_LEN {
            frames.push(GameBoyFrame::default());
        }

        Self {
            frames,
            next_write_index: 0,
            latest_written_index: None,
            latest_sequence: GameBoyFrameSequence::default(),
        }
    }
}

impl GameBoyFrameRing {
    pub(crate) fn clear_to_black(&mut self) {
        for frame in &mut self.frames {
            frame.pixels.fill(0);
        }
        self.next_write_index = 0;
        self.latest_written_index = None;
        self.latest_sequence = GameBoyFrameSequence::default();
    }

    pub(crate) fn borrow_next_write_frame(&mut self) -> Option<WritableGameBoyFrame<'_>> {
        let Self {
            frames,
            next_write_index,
            latest_written_index,
            latest_sequence,
        } = self;

        let index = *next_write_index;
        let ring_len = frames.len();
        let sequence = GameBoyFrameSequence(latest_sequence.0.saturating_add(1));
        let frame = frames.get_mut(index)?;
        frame.sequence = sequence;

        Some(WritableGameBoyFrame {
            frame,
            index,
            ring_len,
            next_write_index,
            latest_written_index,
            latest_sequence,
            sequence,
        })
    }

    pub(crate) fn latest_written_frame(&self) -> Option<&GameBoyFrame> {
        self.latest_written_index
            .and_then(|index| self.frames.get(index))
    }
}

pub(crate) struct WritableGameBoyFrame<'a> {
    frame: &'a mut GameBoyFrame,
    index: usize,
    ring_len: usize,
    next_write_index: &'a mut usize,
    latest_written_index: &'a mut Option<usize>,
    latest_sequence: &'a mut GameBoyFrameSequence,
    sequence: GameBoyFrameSequence,
}

impl WritableGameBoyFrame<'_> {
    pub(crate) fn pixels_mut(&mut self) -> &mut [u8] {
        self.frame.pixels.as_mut()
    }

    pub(crate) fn publish(self) {
        *self.latest_written_index = Some(self.index);
        *self.latest_sequence = self.sequence;
        *self.next_write_index = (self.index + 1) % self.ring_len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrowed_frames_publish_in_ring_order() {
        let mut ring = GameBoyFrameRing::default();

        let mut first_frame = ring.borrow_next_write_frame().unwrap();
        first_frame.pixels_mut()[0] = 0x11;
        first_frame.publish();

        let mut second_frame = ring.borrow_next_write_frame().unwrap();
        second_frame.pixels_mut()[0] = 0x22;
        second_frame.publish();

        assert_eq!(ring.latest_written_index, Some(1));
        assert_eq!(ring.next_write_index, 2);
        assert_eq!(ring.frames[0].pixels[0], 0x11);
        assert_eq!(ring.frames[1].pixels[0], 0x22);
    }

    #[test]
    fn borrowed_frame_wraps_to_ring_start() {
        let mut ring = GameBoyFrameRing::default();

        for _ in 0..GAME_BOY_FRAME_RING_LEN {
            ring.borrow_next_write_frame().unwrap().publish();
        }

        assert_eq!(ring.latest_written_index, Some(GAME_BOY_FRAME_RING_LEN - 1));
        assert_eq!(ring.next_write_index, 0);
    }

    #[test]
    fn clear_to_black_removes_latest_frame_and_resets_ring() {
        let mut ring = GameBoyFrameRing::default();
        let mut frame = ring.borrow_next_write_frame().unwrap();
        frame.pixels_mut()[0] = 0xff;
        frame.publish();

        ring.clear_to_black();

        assert!(ring.latest_written_frame().is_none());
        assert_eq!(ring.next_write_index, 0);
        assert_eq!(ring.latest_sequence, GameBoyFrameSequence::default());
        assert!(
            ring.frames
                .iter()
                .all(|frame| frame.pixels.iter().all(|pixel| *pixel == 0))
        );
    }
}
