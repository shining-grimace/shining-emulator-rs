use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use crate::app_assets::AppAssets;
use crate::app_state::AppState;
use crate::app_theme::ActiveTheme;
use crate::input::mappings::RuntimeInputMappings;
use crate::ui_elements::action_hint::action_hints;
use crate::ui_elements::button::button;
use crate::ui_elements::choice_popup::{
    ChoicePopupConfig, ChoicePopupOption, DismissChoicePopupOnOutsideClick, choice_popup,
};
use crate::ui_elements::description::description;
use crate::ui_elements::heading::heading;
use crate::ui_elements::info_message::{InfoMessage, info_message, set_latest_info_message};
use crate::ui_elements::interactions::{
    ActivatedUiElement, DefaultFocusTarget, FocusedUiElement, InitialFocus, SelectedUiElement,
    UiElementKind, UiFocusNav, UiListCellText,
};
use crate::ui_elements::list_view::{ListColumn, ListRow, ListViewConfig, list_view};
use crate::ui_elements::multi_select::{MultiSelectConfig, multi_select};
use crate::ui_elements::styles::{UI_MAX_CONTENT_WIDTH, UI_PANEL_GAP, UI_SCREEN_PADDING};

const HOME_CONTENT_GAP: f32 = 32.0;
const HOME_MESSAGE_GAP: f32 = 12.0;
const HOME_SIDE_PANEL_WIDTH: f32 = 280.0;
const HOME_SIDE_TOP_GAP: f32 = 50.0;
const HOME_SIDE_GROUP_GAP: f32 = 46.0;
const HOME_SORT_LABEL_GAP: f32 = 12.0;
const HOME_SORT_GROUP_GAP: f32 = 24.0;
const AUTO_SAVE_POPUP_WIDTH: f32 = 260.0;
const AUTO_SAVE_POPUP_LEFT: f32 = 760.0;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct SettingsButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct RomDataButton;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct HomeRomList;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct PrimarySortSelect;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct SecondarySortSelect;

#[derive(Clone, Copy, Component, Debug)]
struct HomeRomRow {
    slot: usize,
    rom_index: usize,
}

#[derive(Clone, Copy, Component, Debug, Default)]
struct HomeRomRowsBound;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
struct HomePopupRoot;

pub struct HomeScenePlugin;

impl Plugin for HomeScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Home), spawn_home_scene)
            .add_systems(Update, bind_home_rom_rows.run_if(in_state(AppState::Home)))
            .add_systems(
                PostUpdate,
                (sync_sorted_rom_rows, clear_invisible_home_row_state)
                    .run_if(in_state(AppState::Home)),
            )
            .add_observer(handle_home_activation);
    }
}

fn spawn_home_scene(
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    input_mappings: Res<RuntimeInputMappings>,
) {
    commands.spawn_scene(home_scene(&assets, *theme, &input_mappings));
}

fn handle_home_activation(
    activated: On<Add, ActivatedUiElement>,
    mut commands: Commands,
    assets: Res<AppAssets>,
    theme: Res<ActiveTheme>,
    settings_buttons: Query<(), With<SettingsButton>>,
    rom_data_buttons: Query<(), With<RomDataButton>>,
    popup_options: Query<&ChoicePopupOption>,
    rom_rows: Query<&HomeRomRow>,
    row_nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    lists: Query<(&ComputedNode, &UiGlobalTransform), With<HomeRomList>>,
    popup_roots: Query<Entity, With<HomePopupRoot>>,
    focused: Query<Entity, With<FocusedUiElement>>,
    mut messages: Query<(&mut Text, &mut TextColor, &mut InfoMessage)>,
    state: Res<State<AppState>>,
) {
    if *state.get() != AppState::Home {
        return;
    }

    let entity = activated.entity;
    if settings_buttons.get(entity).is_ok() {
        set_latest_info_message(&mut messages, "Settings screen is not implemented yet.");
    } else if rom_data_buttons.get(entity).is_ok() {
        set_latest_info_message(&mut messages, "ROM data screen is not implemented yet.");
    } else if let Ok(option) = popup_options.get(entity) {
        despawn_home_popups(&mut commands, &popup_roots);
        match option.option_index {
            0 => set_latest_info_message(&mut messages, "Resume Auto-save selected."),
            1 => set_latest_info_message(&mut messages, "Cold Boot selected."),
            _ => set_latest_info_message(&mut messages, "Launch cancelled."),
        }
    } else if let Ok(row) = rom_rows.get(entity) {
        if home_row_visible(entity, &row_nodes, &lists) {
            despawn_home_popups(&mut commands, &popup_roots);
            for entity in &focused {
                commands.entity(entity).remove::<FocusedUiElement>();
            }
            commands.spawn_scene(auto_save_popup_scene(
                &assets,
                *theme,
                ROMS[row.rom_index].name,
            ));
        }
    }
}

fn home_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    input_mappings: &RuntimeInputMappings,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();

    bsn! {
        #HomeScene
        DespawnOnExit::<AppState>(AppState::Home)
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
                    row_gap: px(HOME_CONTENT_GAP),
                }
                Children [
                    heading(font.clone(), theme, "Shining Emulator"),
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
                                Node {
                                    flex_grow: 1.0,
                                    flex_shrink: 1.0,
                                    min_height: px(0.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(HOME_MESSAGE_GAP),
                                }
                                Children [
                                    (
                                        #RomList
                                        list_view(font.clone(), theme, rom_list_config())
                                        HomeRomList
                                        InitialFocus { enabled: true }
                                        DefaultFocusTarget
                                        UiFocusNav { up: {Entity::PLACEHOLDER}, right: #AllSettings, down: {Entity::PLACEHOLDER}, left: {Entity::PLACEHOLDER} }
                                    ),
                                    info_message(font.clone(), theme, "Local provider could not be read: /home/thomas/roms", true),
                                    info_message(font.clone(), theme, "2 other providers could not be read", true),
                                    info_message(font.clone(), theme, "", false)
                                ]
                            ),
                            (
                                Node {
                                    width: px(HOME_SIDE_PANEL_WIDTH),
                                    min_height: px(0.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(HOME_SIDE_GROUP_GAP),
                                }
                                Children [
                                    (
                                        Node {
                                            width: percent(100),
                                            flex_direction: FlexDirection::Column,
                                            row_gap: px(HOME_SORT_GROUP_GAP),
                                        }
                                        Children [
                                            (
                                                #AllSettings
                                                button(font.clone(), "All Settings", theme, UiFocusNav::default())
                                                SettingsButton
                                                UiFocusNav { up: {Entity::PLACEHOLDER}, right: {Entity::PLACEHOLDER}, down: #RomData, left: #RomList }
                                            ),
                                            (
                                                #RomData
                                                button(font.clone(), "ROM Data", theme, UiFocusNav::default())
                                                RomDataButton
                                                UiFocusNav { up: #AllSettings, right: {Entity::PLACEHOLDER}, down: #PrimarySort, left: #RomList }
                                            ),
                                        ]
                                    ),
                                    (
                                        Node {
                                            width: percent(100),
                                            flex_direction: FlexDirection::Column,
                                            row_gap: px(HOME_SORT_LABEL_GAP),
                                            margin: UiRect::top(px(HOME_SIDE_TOP_GAP)),
                                        }
                                        Children [
                                            description(font.clone(), theme, "Sort ROMs by:"),
                                            (
                                                #PrimarySort
                                                multi_select(font.clone(), theme, sort_select_config(0))
                                                PrimarySortSelect
                                                UiFocusNav { up: #RomData, right: {Entity::PLACEHOLDER}, down: #SecondarySort, left: #RomList }
                                            ),
                                            description(font.clone(), theme, "Then by:"),
                                            (
                                                #SecondarySort
                                                multi_select(font.clone(), theme, sort_select_config(1))
                                                SecondarySortSelect
                                                UiFocusNav { up: #PrimarySort, right: {Entity::PLACEHOLDER}, down: {Entity::PLACEHOLDER}, left: #RomList }
                                            ),
                                        ]
                                    ),
                                ]
                            ),
                        ]
                    ),
                    action_hints(font, assets.icons.clone(), theme, input_mappings),
                ]
            ),
        ]
    }
}

fn sort_select_config(selected: usize) -> MultiSelectConfig {
    MultiSelectConfig {
        selected,
        options: vec!["Last played", "Provider priority", "A-Z"],
        nav: UiFocusNav::default(),
    }
}

fn rom_list_config() -> ListViewConfig {
    ListViewConfig {
        nav: UiFocusNav::default(),
        scrollbar_nav: UiFocusNav::default(),
        columns: vec![
            ListColumn {
                heading: "Name",
                width_percent: 31.0,
            },
            ListColumn {
                heading: "Origin",
                width_percent: 20.0,
            },
            ListColumn {
                heading: "Author",
                width_percent: 17.0,
            },
            ListColumn {
                heading: "License",
                width_percent: 14.0,
            },
            ListColumn {
                heading: "Last played",
                width_percent: 18.0,
            },
        ],
        rows: ROMS.iter().map(|rom| rom_row(rom)).collect(),
    }
}

fn rom_row(rom: &HomeRom) -> ListRow {
    ListRow {
        cells: vec![
            rom.name,
            rom.origin,
            rom.author,
            rom.license,
            rom.last_played_label,
        ],
        nav: UiFocusNav::default(),
    }
}

fn bind_home_rom_rows(
    mut commands: Commands,
    lists: Query<(Entity, &Children), (With<HomeRomList>, Without<HomeRomRowsBound>)>,
    kinds: Query<&UiElementKind>,
    child_query: Query<&Children>,
) {
    for (list_entity, children) in &lists {
        let rows = collect_list_items(children, &kinds, &child_query);
        for (slot, row) in rows.into_iter().enumerate() {
            commands.entity(row).insert(HomeRomRow {
                slot,
                rom_index: slot,
            });
        }
        commands.entity(list_entity).insert(HomeRomRowsBound);
    }
}

fn sync_sorted_rom_rows(
    primary: Query<&crate::ui_elements::interactions::UiMultiSelect, With<PrimarySortSelect>>,
    secondary: Query<&crate::ui_elements::interactions::UiMultiSelect, With<SecondarySortSelect>>,
    mut rows: Query<(&mut HomeRomRow, &Children)>,
    mut cells: Query<(&mut UiListCellText, &Children)>,
    mut texts: Query<&mut Text>,
    child_query: Query<&Children>,
) {
    let primary_sort = primary
        .single()
        .map(|select| SortField::from_index(select.selected))
        .unwrap_or(SortField::LastPlayed);
    let secondary_sort = secondary
        .single()
        .map(|select| SortField::from_index(select.selected))
        .unwrap_or(SortField::ProviderPriority);
    let order = sorted_rom_indices(primary_sort, secondary_sort);

    for (mut row, children) in &mut rows {
        let Some(rom_index) = order.get(row.slot).copied() else {
            continue;
        };
        if row.rom_index == rom_index {
            continue;
        }
        row.rom_index = rom_index;
        update_row_cells(
            ROMS[rom_index],
            children,
            &mut cells,
            &mut texts,
            &child_query,
        );
    }
}

fn update_row_cells(
    rom: HomeRom,
    children: &Children,
    cells: &mut Query<(&mut UiListCellText, &Children)>,
    texts: &mut Query<&mut Text>,
    child_query: &Query<&Children>,
) {
    let values = [
        rom.name,
        rom.origin,
        rom.author,
        rom.license,
        rom.last_played_label,
    ];

    for (cell_index, cell_entity) in collect_cell_entities(children, cells, child_query)
        .into_iter()
        .enumerate()
    {
        let Some(value) = values.get(cell_index) else {
            continue;
        };
        let Ok((mut cell, text_children)) = cells.get_mut(cell_entity) else {
            continue;
        };
        cell.value = (*value).to_string();
        for child in text_children {
            if let Ok(mut text) = texts.get_mut(*child) {
                text.0 = (*value).to_string();
            }
        }
    }
}

fn collect_list_items(
    children: &Children,
    kinds: &Query<&UiElementKind>,
    child_query: &Query<&Children>,
) -> Vec<Entity> {
    let mut items = Vec::new();
    collect_list_items_recursive(children, kinds, child_query, &mut items);
    items
}

fn collect_list_items_recursive(
    children: &Children,
    kinds: &Query<&UiElementKind>,
    child_query: &Query<&Children>,
    items: &mut Vec<Entity>,
) {
    for child in children {
        if kinds
            .get(*child)
            .is_ok_and(|kind| *kind == UiElementKind::ListItem)
        {
            items.push(*child);
            continue;
        }

        if let Ok(grandchildren) = child_query.get(*child) {
            collect_list_items_recursive(grandchildren, kinds, child_query, items);
        }
    }
}

fn collect_cell_entities(
    children: &Children,
    cells: &Query<(&mut UiListCellText, &Children)>,
    child_query: &Query<&Children>,
) -> Vec<Entity> {
    let mut entities = Vec::new();
    collect_cell_entities_recursive(children, cells, child_query, &mut entities);
    entities
}

fn collect_cell_entities_recursive(
    children: &Children,
    cells: &Query<(&mut UiListCellText, &Children)>,
    child_query: &Query<&Children>,
    entities: &mut Vec<Entity>,
) {
    for child in children {
        if cells.contains(*child) {
            entities.push(*child);
            continue;
        }

        if let Ok(grandchildren) = child_query.get(*child) {
            collect_cell_entities_recursive(grandchildren, cells, child_query, entities);
        }
    }
}

fn auto_save_popup_scene(
    assets: &AppAssets,
    theme: ActiveTheme,
    rom_name: &'static str,
) -> impl Scene {
    let font = assets.ubuntu_mono_font.clone();

    bsn! {
        #AutoSavePopup
        DespawnOnExit::<AppState>(AppState::Home)
        Node {
            position_type: PositionType::Absolute,
            left: px(AUTO_SAVE_POPUP_LEFT),
            bottom: px(126.0),
        }
        HomePopupRoot
        DismissChoicePopupOnOutsideClick
        Children [
            choice_popup(font, theme, ChoicePopupConfig {
                title: rom_name,
                width: AUTO_SAVE_POPUP_WIDTH,
                options: ["Resume Auto-save", "Cold Boot", "Cancel"],
            })
        ]
    }
}

fn despawn_home_popups(commands: &mut Commands, popup_roots: &Query<Entity, With<HomePopupRoot>>) {
    for popup in popup_roots {
        commands.entity(popup).despawn();
    }
}

fn clear_invisible_home_row_state(
    mut commands: Commands,
    rows: Query<
        (
            Entity,
            Has<SelectedUiElement>,
            Has<FocusedUiElement>,
            Has<ActivatedUiElement>,
        ),
        With<HomeRomRow>,
    >,
    row_nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    lists: Query<(&ComputedNode, &UiGlobalTransform), With<HomeRomList>>,
) {
    for (entity, selected, focused, activated) in &rows {
        if home_row_visible(entity, &row_nodes, &lists) {
            continue;
        }

        let mut entity_commands = commands.entity(entity);
        if selected {
            entity_commands.remove::<SelectedUiElement>();
        }
        if focused {
            entity_commands.remove::<FocusedUiElement>();
        }
        if activated {
            entity_commands.remove::<ActivatedUiElement>();
        }
    }
}

fn home_row_visible(
    row: Entity,
    row_nodes: &Query<(&ComputedNode, &UiGlobalTransform)>,
    lists: &Query<(&ComputedNode, &UiGlobalTransform), With<HomeRomList>>,
) -> bool {
    let Ok((row_node, row_transform)) = row_nodes.get(row) else {
        return false;
    };
    let Some((list_node, list_transform)) = lists.iter().next() else {
        return true;
    };

    let (_, _, row_center) = row_transform.to_scale_angle_translation();
    let (_, _, list_center) = list_transform.to_scale_angle_translation();
    let row_half_height = row_node.size().y * 0.5;
    let list_half_height = list_node.size().y * 0.5;
    let list_top = list_center.y - list_half_height;
    let list_bottom = list_center.y + list_half_height;
    let row_top = row_center.y - row_half_height;
    let row_bottom = row_center.y + row_half_height;

    row_bottom > list_top && row_top < list_bottom
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SortField {
    LastPlayed,
    ProviderPriority,
    Alphabetical,
}

impl SortField {
    fn from_index(index: usize) -> Self {
        match index {
            1 => Self::ProviderPriority,
            2 => Self::Alphabetical,
            _ => Self::LastPlayed,
        }
    }
}

fn sorted_rom_indices(primary: SortField, secondary: SortField) -> Vec<usize> {
    let mut indices = (0..ROMS.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| compare_roms(ROMS[*left], ROMS[*right], primary, secondary));
    indices
}

fn compare_roms(
    left: HomeRom,
    right: HomeRom,
    primary: SortField,
    secondary: SortField,
) -> std::cmp::Ordering {
    compare_field(left, right, primary)
        .then_with(|| compare_field(left, right, secondary))
        .then_with(|| compare_field(left, right, SortField::Alphabetical))
}

fn compare_field(left: HomeRom, right: HomeRom, field: SortField) -> std::cmp::Ordering {
    match field {
        SortField::LastPlayed => right.last_played_rank.cmp(&left.last_played_rank),
        SortField::ProviderPriority => left
            .provider_priority
            .cmp(&right.provider_priority)
            .then_with(|| left.origin.cmp(right.origin)),
        SortField::Alphabetical => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
    }
}

#[derive(Clone, Copy)]
struct HomeRom {
    name: &'static str,
    origin: &'static str,
    author: &'static str,
    license: &'static str,
    last_played_label: &'static str,
    last_played_rank: u64,
    provider_priority: u8,
}

const ROMS: [HomeRom; 10] = [
    HomeRom {
        name: "ALIEN BARRAGE",
        origin: "Local Folder",
        author: "",
        license: "",
        last_played_label: "Yesterday 1:32PM",
        last_played_rank: 202605241332,
        provider_priority: 1,
    },
    HomeRom {
        name: "Cabbage Dodge",
        origin: "Homebrew Hub",
        author: "Alice Bobson",
        license: "GNU 2.0",
        last_played_label: "Mar 21, 2026",
        last_played_rank: 202603210000,
        provider_priority: 2,
    },
    HomeRom {
        name: "Extremely Frustrating Gauntlet",
        origin: "Homebrew Hub",
        author: "Cathy Donaldson-Smith",
        license: "",
        last_played_label: "Mar 10, 2025",
        last_played_rank: 202503100000,
        provider_priority: 2,
    },
    HomeRom {
        name: "hope.gb",
        origin: "Local Folder",
        author: "",
        license: "",
        last_played_label: "",
        last_played_rank: 0,
        provider_priority: 1,
    },
    HomeRom {
        name: "Igloo Jaunt",
        origin: "Homebrew Hub",
        author: "",
        license: "",
        last_played_label: "",
        last_played_rank: 0,
        provider_priority: 2,
    },
    HomeRom {
        name: "Kangaroo Leapathon",
        origin: "Homebrew Hub",
        author: "",
        license: "",
        last_played_label: "",
        last_played_rank: 0,
        provider_priority: 2,
    },
    HomeRom {
        name: "Marshmallow Nebula",
        origin: "Homebrew Hub",
        author: "",
        license: "",
        last_played_label: "",
        last_played_rank: 0,
        provider_priority: 2,
    },
    HomeRom {
        name: "Orangutan Panic",
        origin: "Homebrew Hub",
        author: "",
        license: "",
        last_played_label: "",
        last_played_rank: 0,
        provider_priority: 2,
    },
    HomeRom {
        name: "Queen Rita",
        origin: "Homebrew Hub",
        author: "",
        license: "",
        last_played_label: "",
        last_played_rank: 0,
        provider_priority: 2,
    },
    HomeRom {
        name: "Sorry Thunder",
        origin: "Homebrew Hub",
        author: "",
        license: "",
        last_played_label: "",
        last_played_rank: 0,
        provider_priority: 2,
    },
];
