use bevy::ecs::template::EntityTemplate;
use bevy::prelude::*;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::input::mappings::RuntimeInputMappings;
use crate::ui_elements::action_hint::action_hints;
use crate::ui_elements::button::button;
use crate::ui_elements::description::description;
use crate::ui_elements::file_picker::file_picker;
use crate::ui_elements::heading::heading;
use crate::ui_elements::interactions::{
    DefaultFocusTarget, DisabledUiElement, InitialFocus, UiFocusNav,
};
use crate::ui_elements::list_view::{ListColumn, ListRow, ListViewConfig, list_view};
use crate::ui_elements::multi_select::{MultiSelectConfig, multi_select};
use crate::ui_elements::scroll_view::{ScrollViewConfig, scroll_view};
use crate::ui_elements::select_popup::{SelectPopupConfig, SelectPopupOption, select_popup};
use crate::ui_elements::styles::{UI_MAX_CONTENT_WIDTH, UI_PANEL_GAP, UI_SCREEN_PADDING};
use crate::ui_elements::text_input::text_input;

pub struct InterfaceDemoScenePlugin;

impl Plugin for InterfaceDemoScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InterfaceDemo), spawn_interface_demo_scene);
    }
}

fn spawn_interface_demo_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    input_mappings: Res<RuntimeInputMappings>,
) {
    commands.spawn_scene(interface_demo_scene(&assets, *theme, &input_mappings));
}

fn interface_demo_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    input_mappings: &RuntimeInputMappings,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();
    let left_column_font = font.clone();
    let right_column_font = font.clone();

    bsn! {
        #InterfaceDemoScene
        DespawnOnExit::<AppState>(AppState::InterfaceDemo)
        Node {
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(UI_SCREEN_PADDING)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Children [
            (
                Node {
                    width: percent(100),
                    max_width: px(UI_MAX_CONTENT_WIDTH),
                    height: percent(100),
                    min_height: px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(38.0),
                }
                Children [
                    (
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(8.0),
                        }
                        Children [
                            heading(font.clone(), theme, "Interface Demo"),
                        ]
                    ),
                    (
                        Node {
                            width: percent(100),
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            min_height: px(0.0),
                            flex_direction: FlexDirection::Row,
                            column_gap: px(UI_PANEL_GAP),
                        }
                        Children [
                            (
                                #LeftScrollBar
                                scroll_view(
                                    theme,
                                    #LeftScrollBar,
                                    ScrollViewConfig {
                                        width: percent(46),
                                        min_height: px(0.0),
                                        thumb_height: 112.0,
                                    },
                                    move |scroll_bar| control_column(left_column_font, theme, scroll_bar),
                                )
                                UiFocusNav { up: {Entity::PLACEHOLDER}, right: #ListView, down: {Entity::PLACEHOLDER}, left: {Entity::PLACEHOLDER} }
                            ),
                            (
                                Node {
                                    width: percent(54),
                                    min_height: px(0.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(40.0),
                                }
                                Children [
                                    (
                                        #ListView
                                        list_view(right_column_font, theme, demo_list_config())
                                        UiFocusNav { up: {Entity::PLACEHOLDER}, right: {Entity::PLACEHOLDER}, down: #FilePicker, left: #LeftScrollBar }
                                    ),
                                    (
                                        #FilePicker
                                        file_picker(font.clone(), "Choose a file to add...", theme, UiFocusNav::default())
                                        UiFocusNav { up: #ListView, right: {Entity::PLACEHOLDER}, down: {Entity::PLACEHOLDER}, left: {Entity::PLACEHOLDER} }
                                    )
                                ]
                            ),
                        ]
                    ),
                    action_hints(font.clone(), assets.icons.clone(), theme, input_mappings),
                ]
            ),
        ]
    }
}

fn control_column(
    font: Handle<Font>,
    theme: ActiveTheme,
    scroll_bar_target: EntityTemplate,
) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(24.0),
            padding: UiRect::bottom(px(150.0)),
        }
        Children [
            (
                Node {
                    width: percent(100),
                    column_gap: px(42.0),
                }
                Children [
                    (
                        #EnabledButton
                        button(font.clone(), "Enabled Button", theme, UiFocusNav::default())
                        InitialFocus { enabled: true }
                        DefaultFocusTarget
                        UiFocusNav { up: {Entity::PLACEHOLDER}, right: {scroll_bar_target}, down: #TextInput, left: {Entity::PLACEHOLDER} }
                    ),
                    (
                        #DisabledButton
                        button(font.clone(), "Disabled Button", theme, UiFocusNav::default())
                        DisabledUiElement
                        UiFocusNav { up: {Entity::PLACEHOLDER}, right: {scroll_bar_target}, down: #TextInput, left: #EnabledButton }
                    )
                ]
            ),
            description(font.clone(), theme, "This text describes the buttons above and may need to wrap around multiple lines on some screens, especially on narrower windows such as mobile in portrait orientation."),
            (
                #TextInput
                text_input(font.clone(), "Enter your text here...", theme, UiFocusNav::default())
                UiFocusNav { up: #EnabledButton, right: {scroll_bar_target}, down: #MultiSelect, left: {Entity::PLACEHOLDER} }
            ),
            description(font.clone(), theme, "This selection allows you to choose an option which you would like to have selected."),
            (
                #MultiSelect
                multi_select(font.clone(), theme, demo_multi_select_config())
                UiFocusNav { up: #TextInput, right: {scroll_bar_target}, down: {Entity::PLACEHOLDER}, left: {Entity::PLACEHOLDER} }
            ),
            (
                Node { display: Display::None }
                Children [
                    select_popup(font.clone(), theme, demo_select_popup_config())
                ]
            ),
            description(font, theme, "This text fills vertical space for testing the scrollbar.")
        ]
    }
}

fn demo_select_popup_config() -> SelectPopupConfig {
    SelectPopupConfig {
        title: "Select something",
        width: 242.0,
        options: vec![
            SelectPopupOption {
                label: "Anything",
                focused: true,
            },
            SelectPopupOption {
                label: "Something else",
                focused: false,
            },
        ],
    }
}

fn demo_multi_select_config() -> MultiSelectConfig {
    MultiSelectConfig {
        selected: 0,
        options: vec!["Anything", "Something else"],
        nav: UiFocusNav::default(),
    }
}

fn demo_list_config() -> ListViewConfig {
    ListViewConfig {
        nav: UiFocusNav::default(),
        scrollbar_nav: UiFocusNav::default(),
        columns: vec![
            ListColumn {
                heading: "Item Name",
                width_percent: 48.0,
            },
            ListColumn {
                heading: "Author",
                width_percent: 26.0,
            },
            ListColumn {
                heading: "Downloaded",
                width_percent: 26.0,
            },
        ],
        rows: vec![
            ListRow {
                cells: list_cells(vec!["Item One Has a Rather Long Name", "", "Yes"]),
                nav: UiFocusNav::default(),
            },
            ListRow {
                cells: list_cells(vec!["Item Two", "Alice Bobson", "No"]),
                nav: UiFocusNav::default(),
            },
            ListRow {
                cells: list_cells(vec!["Item Three", "", "No"]),
                nav: UiFocusNav::default(),
            },
            ListRow {
                cells: list_cells(vec!["Item Four", "Cathy Donaldson-Smith", "Yes"]),
                nav: UiFocusNav::default(),
            },
            ListRow {
                cells: list_cells(vec!["Item Five", "", "No"]),
                nav: UiFocusNav::default(),
            },
        ],
        virtual_total_rows: None,
    }
}

fn list_cells(cells: Vec<&'static str>) -> Vec<String> {
    cells.into_iter().map(str::to_string).collect()
}
