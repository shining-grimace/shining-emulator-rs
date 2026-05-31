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

use bevy::prelude::*;

pub use focus::*;
pub use list_view::*;
pub use multi_select::*;
pub use picking::*;
pub use scroll::*;
pub use text_input::*;
pub use visual_state::*;

pub struct UiInteractionsPlugin;

impl Plugin for UiInteractionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<focus::LastFocusedUiElement>()
            .init_resource::<Clipboard>()
            .init_resource::<ui_input::UiInputState>()
            .init_resource::<scroll::ScrollThumbDragState>()
            .add_message::<picking::UiPointerClicked>()
            .add_message::<crate::ui_elements::file_picker::UiFilePickerActivated>()
            .add_observer(focus::bind_focus_nav_ids)
            .add_systems(
                Update,
                (
                    ui_input::collect_ui_input_state.after(crate::input::InputSet::Collect),
                    (
                        focus::ensure_initial_focus,
                        focus::focus_pressed_element,
                        focus::navigate_focus,
                        picking::setup_pointer_tracking,
                        picking::apply_picking_markers,
                        picking::sync_pointer_states,
                        picking::clear_activation_markers,
                        visual_state::update_interaction_colours,
                        crate::ui_elements::file_picker::update_file_picker_hover_colours,
                        text_input::edit_text_inputs,
                        activation::activate_controls,
                        activation::dismiss_multi_selects_on_pointer_release,
                        focus::restore_focus_from_input,
                    )
                        .chain(),
                    (
                        crate::ui_elements::file_picker::drain_file_picker_activations,
                        list_view::focus_list_item_on_list_focus,
                        list_view::enter_focused_list_item,
                        list_view::remember_focused_list_item,
                        list_view::update_list_cell_text,
                        multi_select::update_multi_select_popups,
                        scroll::update_dynamic_scroll_metrics,
                        list_view::update_list_item_pickability,
                        scroll::update_scroll_thumb_colours,
                        scroll::scroll_focused_scrollbar_by_keys,
                        scroll::scroll_areas,
                        scroll::track_scroll_thumb_drag,
                        scroll::drag_scroll_thumbs,
                        scroll::keep_focused_list_item_visible,
                        scroll::clear_focus_auto_scroll_suppression,
                        text_input::update_text_input_text,
                        focus::remember_focused_element,
                    )
                        .chain(),
                )
                    .chain(),
            );
    }
}
