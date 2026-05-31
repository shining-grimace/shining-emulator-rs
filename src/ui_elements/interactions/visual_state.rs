use bevy::prelude::*;

use super::focus::FocusedUiElement;
use super::picking::HoveredUiElement;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct ActivatedUiElement;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct SelectedUiElement;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct DisabledUiElement;

#[derive(Clone, Copy, Component, Debug, FromTemplate)]
pub struct UiElementColors {
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub fill: Color,
    pub hover_fill: Color,
}

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiElementLabel;

#[derive(Clone, Copy, Component, Debug, Default, FromTemplate)]
pub struct UiElementAccent;

#[derive(Clone, Copy, Component, Debug, Default, Eq, FromTemplate, PartialEq)]
pub enum UiElementKind {
    #[default]
    Button,
    List,
    ListItem,
    ScrollBar,
    TextInput,
    MultiSelect,
    MultiSelectOption,
}

pub(super) fn update_interaction_colours(
    mut controls: Query<
        (
            &UiElementKind,
            &UiElementColors,
            Option<&mut BorderColor>,
            &mut BackgroundColor,
            Has<HoveredUiElement>,
            Has<FocusedUiElement>,
            Has<SelectedUiElement>,
            Has<DisabledUiElement>,
            Option<&Children>,
        ),
        Without<UiElementAccent>,
    >,
    mut labels: Query<&mut TextColor, With<UiElementLabel>>,
    mut accents: Query<&mut BackgroundColor, (With<UiElementAccent>, Without<UiElementColors>)>,
    child_query: Query<&Children>,
) {
    for (kind, colours, border, mut background, hovered, focused, selected, disabled, children) in
        &mut controls
    {
        let active = focused || selected;
        let active_colour = if active {
            colours.secondary
        } else {
            colours.primary
        };

        if let Some(mut border) = border {
            let next_border = if disabled {
                BorderColor::all(Color::NONE)
            } else {
                BorderColor::all(active_colour)
            };
            if *border != next_border {
                *border = next_border;
            }
        }

        let next_background = if (hovered || active) && !disabled {
            colours.hover_fill
        } else {
            colours.fill
        };
        if background.0 != next_background {
            background.0 = next_background;
        }

        if let Some(children) = children.filter(|_| *kind != UiElementKind::List) {
            update_label_colours(
                children,
                if disabled {
                    colours.tertiary
                } else {
                    active_colour
                },
                &mut labels,
                &child_query,
            );
        }

        if let Some(children) = children {
            update_accent_colours(children, active_colour, &mut accents, &child_query);
        }
    }
}

fn update_label_colours(
    children: &Children,
    colour: Color,
    labels: &mut Query<&mut TextColor, With<UiElementLabel>>,
    child_query: &Query<&Children>,
) {
    for child in children {
        if let Ok(mut text_colour) = labels.get_mut(*child) {
            if text_colour.0 != colour {
                text_colour.0 = colour;
            }
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            update_label_colours(grandchildren, colour, labels, child_query);
        }
    }
}

fn update_accent_colours(
    children: &Children,
    colour: Color,
    accents: &mut Query<&mut BackgroundColor, (With<UiElementAccent>, Without<UiElementColors>)>,
    child_query: &Query<&Children>,
) {
    for child in children {
        if let Ok(mut background) = accents.get_mut(*child) {
            if background.0 != colour {
                background.0 = colour;
            }
        }
        if let Ok(grandchildren) = child_query.get(*child) {
            update_accent_colours(grandchildren, colour, accents, child_query);
        }
    }
}
