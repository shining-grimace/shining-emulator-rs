use bevy::prelude::*;

pub(crate) fn contains_entity(
    children: &Children,
    target: Entity,
    child_query: &Query<&Children>,
) -> bool {
    for child in children {
        if *child == target {
            return true;
        }
        if child_query
            .get(*child)
            .is_ok_and(|grandchildren| contains_entity(grandchildren, target, child_query))
        {
            return true;
        }
    }
    false
}
