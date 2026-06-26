mod activation;
mod focus;
mod list_view;
mod multi_select;
mod picking;
mod scroll;
mod text_input;
pub(crate) mod tree;
mod ui_input;
mod visual_state;

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

pub use focus::*;
pub use list_view::*;
pub use multi_select::*;
pub use picking::*;
pub use scroll::*;
pub use text_input::*;
pub use ui_input::UiInputCapture;
pub use visual_state::*;

/// Ordered UI interaction phases that run inside Bevy's [`Update`] schedule.
///
/// This follows the same broad pattern as Avian's `PhysicsSystems`: callers can
/// schedule systems relative to named phases without depending on the exact
/// implementation systems.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UiSchedule {
    /// Input collection and transient state cleanup before UI interaction logic.
    First,
    /// One-time setup for newly spawned UI entities.
    Prepare,
    /// Pointer marker synchronization from Bevy picking state.
    Pointer,
    /// Focus changes from pointer, keyboard, and gamepad input.
    Focus,
    /// Text input mutation while focus is settled.
    TextInput,
    /// Control activation, selection, and popup open/close state changes.
    Activation,
    /// Widget state updates that depend on activation and focus.
    Widgets,
    /// Scroll metrics and content/thumb positions.
    Scroll,
    /// Visual state derived from the final interaction state for this frame.
    VisualState,
    /// End-of-pass cleanup and remembered focus bookkeeping.
    Last,
}

pub struct UiInteractionsPlugin;

impl Plugin for UiInteractionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<focus::LastFocusedUiElement>()
            .init_resource::<Clipboard>()
            .init_resource::<ui_input::UiInputState>()
            .init_resource::<ui_input::UiInputCapture>()
            .init_resource::<picking::UiPointerPressState>()
            .init_resource::<scroll::ScrollThumbDragState>()
            .add_message::<picking::UiPointerClicked>()
            .add_message::<crate::ui_elements::file_picker::UiFilePickerActivated>()
            .add_message::<crate::ui_elements::file_picker::UiFilePickerResult>()
            .add_observer(focus::bind_focus_nav_ids)
            .configure_sets(
                Update,
                (
                    UiSchedule::First,
                    UiSchedule::Prepare,
                    UiSchedule::Pointer,
                    UiSchedule::Focus,
                    UiSchedule::TextInput,
                    UiSchedule::Activation,
                    UiSchedule::Widgets,
                    UiSchedule::Scroll,
                    UiSchedule::VisualState,
                    UiSchedule::Last,
                )
                    .chain()
                    .after(crate::input::InputSet::Collect),
            )
            .add_systems(
                Update,
                (
                    ui_input::collect_ui_input_state,
                    picking::clear_activation_markers.run_if(has_activated_ui_elements),
                )
                    .chain()
                    .in_set(UiSchedule::First),
            )
            .add_systems(
                Update,
                (
                    picking::setup_pointer_tracking.run_if(pointer_tracking_components_added),
                    scroll::setup_scroll_drag_tracking
                        .run_if(scroll_drag_tracking_components_added),
                    picking::apply_picking_markers.run_if(picking_marker_components_added),
                    crate::ui_elements::file_picker::apply_file_picker_results,
                )
                    .chain()
                    .in_set(UiSchedule::Prepare),
            )
            .add_systems(
                Update,
                (
                    picking::resolve_pointer_activations,
                    picking::sync_pointer_states.run_if(pointer_state_may_have_changed),
                )
                    .chain()
                    .in_set(UiSchedule::Pointer),
            )
            .add_systems(
                Update,
                (
                    focus::ensure_initial_focus.run_if(initial_focus_added),
                    focus::focus_pressed_element,
                    focus::restore_focus_from_input,
                    list_view::navigate_virtual_list_by_keys,
                    focus::navigate_focus,
                    list_view::focus_list_item_on_list_focus,
                    list_view::enter_focused_list_item,
                    list_view::remember_focused_list_item,
                )
                    .chain()
                    .in_set(UiSchedule::Focus),
            )
            .add_systems(
                Update,
                text_input::edit_text_inputs.in_set(UiSchedule::TextInput),
            )
            .add_systems(
                Update,
                (
                    activation::select_virtual_list_rows,
                    activation::activate_controls,
                    activation::dismiss_multi_selects_on_pointer_release,
                )
                    .chain()
                    .in_set(UiSchedule::Activation),
            )
            .add_systems(
                Update,
                (
                    multi_select::update_multi_select_popups
                        .run_if(multi_select_popups_may_need_update),
                    list_view::update_list_cell_text,
                    text_input::update_text_input_text,
                )
                    .chain()
                    .in_set(UiSchedule::Widgets),
            )
            .add_systems(
                Update,
                (
                    scroll::update_dynamic_scroll_metrics.run_if(scroll_metrics_may_need_update),
                    scroll::scroll_focused_scrollbar_by_keys,
                    scroll::scroll_areas,
                    scroll::keep_focused_list_item_visible,
                    list_view::update_list_item_pickability
                        .run_if(list_item_pickability_may_need_update),
                )
                    .chain()
                    .in_set(UiSchedule::Scroll),
            )
            .add_systems(
                Update,
                (
                    scroll::update_scroll_thumb_colours.run_if(focus_markers_changed),
                    list_view::sync_virtual_list_selection
                        .run_if(virtual_list_selection_may_need_update),
                    visual_state::update_interaction_colours
                        .run_if(interaction_colours_may_need_update),
                    crate::ui_elements::file_picker::update_file_picker_hover_colours
                        .run_if(file_picker_hover_colours_may_need_update),
                )
                    .chain()
                    .in_set(UiSchedule::VisualState),
            )
            .add_systems(
                Update,
                (
                    scroll::clear_scroll_thumb_drag_release.run_if(scroll_thumb_was_released),
                    scroll::clear_focus_auto_scroll_suppression
                        .run_if(has_focus_auto_scroll_suppression),
                    focus::remember_focused_element.run_if(focus_markers_changed),
                )
                    .chain()
                    .in_set(UiSchedule::Last),
            );
    }
}

fn has_activated_ui_elements(activated: Query<(), With<visual_state::ActivatedUiElement>>) -> bool {
    !activated.is_empty()
}

fn pointer_tracking_components_added(
    buttons: Query<(), Added<Button>>,
    scroll_areas: Query<(), Added<scroll::UiScrollArea>>,
    draggables: Query<(), Added<picking::DraggableUiElement>>,
) -> bool {
    !buttons.is_empty() || !scroll_areas.is_empty() || !draggables.is_empty()
}

fn scroll_drag_tracking_components_added(
    scroll_areas: Query<(), Added<scroll::UiScrollArea>>,
    thumbs: Query<
        (),
        (
            Added<scroll::UiScrollThumb>,
            With<picking::DraggableUiElement>,
        ),
    >,
) -> bool {
    !scroll_areas.is_empty() || !thumbs.is_empty()
}

fn picking_marker_components_added(
    ignored: Query<(), Added<picking::IgnorePicking>>,
    blockers: Query<(), Added<picking::BlockPickingOnly>>,
    modals: Query<(), Added<picking::ModalUiElement>>,
    scroll_contents: Query<(), Added<scroll::UiScrollContent>>,
    passive_layouts: Query<
        (),
        (
            Added<Node>,
            Without<Button>,
            Without<scroll::UiScrollArea>,
            Without<multi_select::UiMultiSelectPopup>,
            Without<picking::DraggableUiElement>,
            Without<picking::IgnorePicking>,
            Without<picking::BlockPickingOnly>,
            Without<picking::ModalUiElement>,
        ),
    >,
) -> bool {
    !ignored.is_empty()
        || !blockers.is_empty()
        || !modals.is_empty()
        || !scroll_contents.is_empty()
        || !passive_layouts.is_empty()
}

fn pointer_state_may_have_changed(changed: Query<(), Changed<picking::UiPointerState>>) -> bool {
    !changed.is_empty()
}

fn initial_focus_added(candidates: Query<(), Added<focus::InitialFocus>>) -> bool {
    !candidates.is_empty()
}

fn multi_select_popups_may_need_update(
    opened: Query<(), Added<multi_select::OpenUiElement>>,
    open_popups: Query<(), Added<multi_select::OpenUiMultiSelectPopup>>,
    changed_nodes: Query<(), Changed<Node>>,
    mut closed: RemovedComponents<multi_select::OpenUiElement>,
) -> bool {
    !opened.is_empty()
        || !open_popups.is_empty()
        || !changed_nodes.is_empty()
        || closed.read().next().is_some()
}

fn scroll_metrics_may_need_update(
    wheel_events: MessageReader<MouseWheel>,
    changed_areas: Query<(), Changed<scroll::UiScrollArea>>,
    changed_area_nodes: Query<(), (With<scroll::UiScrollArea>, Changed<ComputedNode>)>,
    changed_content_nodes: Query<(), (With<scroll::UiScrollContent>, Changed<ComputedNode>)>,
    changed_scrollbars: Query<(), (With<scroll::UiScrollbar>, Changed<ComputedNode>)>,
    added_scroll_content: Query<(), Added<scroll::UiScrollContent>>,
    added_scrollbar: Query<(), Added<scroll::UiScrollbar>>,
) -> bool {
    !wheel_events.is_empty()
        || !changed_areas.is_empty()
        || !changed_area_nodes.is_empty()
        || !changed_content_nodes.is_empty()
        || !changed_scrollbars.is_empty()
        || !added_scroll_content.is_empty()
        || !added_scrollbar.is_empty()
}

fn list_item_pickability_may_need_update(
    changed_areas: Query<
        (),
        (
            With<scroll::UiScrollArea>,
            Or<(Changed<ComputedNode>, Changed<UiGlobalTransform>)>,
        ),
    >,
    changed_items: Query<
        (),
        (
            With<visual_state::UiElementKind>,
            Or<(Changed<ComputedNode>, Changed<UiGlobalTransform>)>,
        ),
    >,
) -> bool {
    !changed_areas.is_empty() || !changed_items.is_empty()
}

fn virtual_list_selection_may_need_update(
    changed_selections: Query<(), Changed<crate::ui_elements::list_view::VirtualListSelection>>,
    changed_rows: Query<(), Changed<crate::ui_elements::list_view::VirtualListRow>>,
    added_selected: Query<(), Added<visual_state::SelectedUiElement>>,
    mut removed_selected: RemovedComponents<visual_state::SelectedUiElement>,
) -> bool {
    !changed_selections.is_empty()
        || !changed_rows.is_empty()
        || !added_selected.is_empty()
        || removed_selected.read().next().is_some()
}

fn focus_markers_changed(
    added: Query<(), Added<focus::FocusedUiElement>>,
    mut removed: RemovedComponents<focus::FocusedUiElement>,
) -> bool {
    !added.is_empty() || removed.read().next().is_some()
}

fn interaction_colours_may_need_update(
    changed_controls: Query<
        (),
        Or<(
            Added<visual_state::UiElementColors>,
            Changed<visual_state::UiElementColors>,
            Added<picking::HoveredUiElement>,
            Added<focus::FocusedUiElement>,
            Added<visual_state::SelectedUiElement>,
            Added<visual_state::DisabledUiElement>,
        )>,
    >,
    mut removed_hover: RemovedComponents<picking::HoveredUiElement>,
    mut removed_focus: RemovedComponents<focus::FocusedUiElement>,
    mut removed_selected: RemovedComponents<visual_state::SelectedUiElement>,
    mut removed_disabled: RemovedComponents<visual_state::DisabledUiElement>,
) -> bool {
    !changed_controls.is_empty()
        || removed_hover.read().next().is_some()
        || removed_focus.read().next().is_some()
        || removed_selected.read().next().is_some()
        || removed_disabled.read().next().is_some()
}

fn file_picker_hover_colours_may_need_update(
    changed_pickers: Query<
        (),
        (
            With<crate::ui_elements::file_picker::UiFilePicker>,
            Or<(
                Changed<picking::HoveredUiElement>,
                Changed<focus::FocusedUiElement>,
            )>,
        ),
    >,
    added_pickers: Query<(), Added<crate::ui_elements::file_picker::UiFilePicker>>,
    added_hover_fills: Query<(), Added<crate::ui_elements::file_picker::UiFilePickerHoverFill>>,
    mut removed_hover: RemovedComponents<picking::HoveredUiElement>,
    mut removed_focus: RemovedComponents<focus::FocusedUiElement>,
) -> bool {
    !changed_pickers.is_empty()
        || !added_pickers.is_empty()
        || !added_hover_fills.is_empty()
        || removed_hover.read().next().is_some()
        || removed_focus.read().next().is_some()
}

fn scroll_thumb_was_released(state: Res<scroll::ScrollThumbDragState>) -> bool {
    state.released_this_frame()
}

fn has_focus_auto_scroll_suppression(
    suppressed: Query<(), With<scroll::SuppressFocusAutoScroll>>,
) -> bool {
    !suppressed.is_empty()
}
