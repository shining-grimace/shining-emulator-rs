use bevy::prelude::*;

use crate::app_state::AppState;
use crate::binary_text::constants::{
    BINARY_TEXT_GROUP_MAX_DIGITS, BINARY_TEXT_GROUP_MIN_DIGITS,
    BINARY_TEXT_GROUP_SPAWN_MAX_SECONDS, BINARY_TEXT_GROUP_SPAWN_MIN_SECONDS,
    BINARY_TEXT_INFLUENCE_DIGITS, BINARY_TEXT_WAVE_SECONDS_PER_DIGIT,
};
use crate::visual_effects::ACTIVE_SCREEN_RECT_ANIMATION_SECONDS;

#[derive(Resource, Debug)]
pub(super) struct BinaryTextEffects {
    pub columns: usize,
    pub rows: usize,
    active_screen: Option<AppState>,
    settled_seconds: f32,
    pub spawn_delay: f32,
    next_group_id: u64,
    pub groups: Vec<BinaryTextGroup>,
}

impl BinaryTextEffects {
    pub fn reset_grid(&mut self, columns: usize, rows: usize) {
        if self.columns == columns && self.rows == rows {
            return;
        }

        self.columns = columns;
        self.rows = rows;
        self.groups.clear();
        self.spawn_delay = random_spawn_delay();
    }

    pub fn update(&mut self, delta_seconds: f32, active_screen: Option<AppState>) {
        if self.active_screen != active_screen {
            self.active_screen = active_screen;
            self.settled_seconds = 0.0;
            self.groups.clear();
        }

        if active_screen.is_none() || self.columns == 0 || self.rows == 0 {
            self.groups.clear();
            self.spawn_delay = random_spawn_delay();
            return;
        }

        self.settled_seconds += delta_seconds;
        if !self.is_settled() {
            return;
        }

        for group in &mut self.groups {
            group.elapsed_seconds += delta_seconds;
        }
        self.groups.retain(|group| !group.is_finished());

        self.spawn_delay -= delta_seconds;
        while self.spawn_delay <= 0.0 {
            self.groups.push(BinaryTextGroup::random(
                self.next_group_id,
                self.columns,
                self.rows,
            ));
            self.next_group_id = self.next_group_id.wrapping_add(1);
            self.spawn_delay += random_spawn_delay();
        }
    }

    pub fn group(&self, id: u64) -> Option<&BinaryTextGroup> {
        self.groups.iter().find(|group| group.id == id)
    }

    pub fn is_settled(&self) -> bool {
        self.settled_seconds >= ACTIVE_SCREEN_RECT_ANIMATION_SECONDS
    }
}

impl Default for BinaryTextEffects {
    fn default() -> Self {
        Self {
            columns: 0,
            rows: 0,
            active_screen: None,
            settled_seconds: 0.0,
            spawn_delay: random_spawn_delay(),
            next_group_id: 1,
            groups: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct BinaryTextGroup {
    pub id: u64,
    pub row: usize,
    pub start_column: usize,
    pub digit_count: usize,
    elapsed_seconds: f32,
}

impl BinaryTextGroup {
    fn random(id: u64, columns: usize, rows: usize) -> Self {
        let max_digit_count = BINARY_TEXT_GROUP_MAX_DIGITS.min(columns.max(1));
        let min_digit_count = BINARY_TEXT_GROUP_MIN_DIGITS.min(max_digit_count);
        let digit_count = random_usize(min_digit_count, max_digit_count);
        let start_column = random_usize(0, columns.saturating_sub(digit_count));

        Self {
            id,
            row: random_usize(0, rows.saturating_sub(1)),
            start_column,
            digit_count,
            elapsed_seconds: 0.0,
        }
    }

    fn duration(&self) -> f32 {
        (self.digit_count as f32 + BINARY_TEXT_INFLUENCE_DIGITS)
            * BINARY_TEXT_WAVE_SECONDS_PER_DIGIT
    }

    fn is_finished(&self) -> bool {
        self.elapsed_seconds >= self.duration()
    }

    pub fn digit_opacity_multiplier(&self, digit_index: usize) -> f32 {
        if digit_index >= self.digit_count {
            return 0.0;
        }

        let digit = digit_index as f32;
        let wave_position = self.elapsed_seconds / BINARY_TEXT_WAVE_SECONDS_PER_DIGIT;
        let distance = (wave_position - digit).abs();
        if distance >= BINARY_TEXT_INFLUENCE_DIGITS {
            return 0.0;
        }

        1.0 - distance / BINARY_TEXT_INFLUENCE_DIGITS
    }
}

fn random_spawn_delay() -> f32 {
    random_range(
        BINARY_TEXT_GROUP_SPAWN_MIN_SECONDS,
        BINARY_TEXT_GROUP_SPAWN_MAX_SECONDS,
    )
}

fn random_range(min: f32, max: f32) -> f32 {
    min + (max - min) * fastrand::f32()
}

fn random_usize(min: usize, max: usize) -> usize {
    if min >= max {
        return min;
    }

    min + fastrand::usize(..=(max - min))
}
