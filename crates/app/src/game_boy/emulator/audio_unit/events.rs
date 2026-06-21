#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GameBoyAudioChannel {
    Pulse1,
    Pulse2,
    Wave,
    Noise,
}

impl GameBoyAudioChannel {
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Pulse1 => 0,
            Self::Pulse2 => 1,
            Self::Wave => 2,
            Self::Noise => 3,
        }
    }

    pub(super) fn status_bit(self) -> u8 {
        1 << self.index()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum GameBoyAudioBalance {
    Both,
    Left,
    Right,
    Pan(f32),
}

impl Default for GameBoyAudioBalance {
    fn default() -> Self {
        Self::Both
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum GameBoyAudioCommand {
    NoteOn {
        frequency_hz: f32,
        volume: f32,
        balance: GameBoyAudioBalance,
    },
    NoteOff,
    Frequency {
        frequency_hz: f32,
    },
    Volume(f32),
    Balance(GameBoyAudioBalance),
    Wavetable([f32; 16]),
    AllNotesOff,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GameBoyAudioEvent {
    pub(crate) tick: u64,
    pub(crate) channel: Option<GameBoyAudioChannel>,
    pub(crate) command: GameBoyAudioCommand,
}
