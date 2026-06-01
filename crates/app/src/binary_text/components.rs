use bevy::prelude::*;

#[derive(Component)]
pub(super) struct BinaryTextLayer;

#[derive(Component)]
pub(super) struct BinaryTextDigit {
    pub group_id: Option<u64>,
    pub digit_index: usize,
}
