use bevy::prelude::*;
use crate::components::simulation::Simulation;
use crate::resources::game_state::GameState;
use crate::systems::simulation::TurnEndEvent;

#[derive(Component)]
pub struct EndTurnButton;

#[derive(Component)]
pub struct TurnDisplay;

#[derive(Component)]
pub struct DateDisplay;

pub fn setup_ui(mut commands: Commands, _asset_server: Res<AssetServer>) {
    // Main UI container
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Stretch,
                ..default()
            },
            ..default()
        },
        UiCameraConfig { show_ui: true },
    )).with_children(|parent| {
        // Top bar with date and turn info
        parent.spawn(
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(80.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.7).into(),
                ..default()
            }
        ).with_children(|top_bar| {
            // Date display
            top_bar.spawn(
                TextBundle {
                    text: Text::from_section(
                        "2070 January 1",
                        TextStyle {
                            font_size: 24.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    ..default()
                }
            ).insert(DateDisplay);
            
            // Turn display
            top_bar.spawn(
                TextBundle {
                    text: Text::from_section(
                        "Turn: 1",
                        TextStyle {
                            font_size: 24.0,
                            color: Color::YELLOW,
                            ..default()
                        },
                    ),
                    ..default()
                }
            ).insert(TurnDisplay);
        });
        
        // Bottom bar with controls
        parent.spawn(
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.7).into(),
                ..default()
            }
        ).with_children(|bottom_bar| {
            // End Turn button
            bottom_bar.spawn(
                ButtonBundle {
                    style: Style {
                        width: Val::Px(150.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: Color::DARK_GRAY.into(),
                    ..default()
                }
            ).insert(EndTurnButton)
            .with_children(|button| {
                button.spawn(
                    TextBundle {
                        text: Text::from_section(
                            "End Turn",
                            TextStyle {
                                font_size: 20.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        ),
                        ..default()
                    }
                );
            });
        });
    });
}

pub fn update_ui_displays(
    mut queries: ParamSet<(
        Query<&mut Text, With<DateDisplay>>,
        Query<&mut Text, With<TurnDisplay>>,
    )>,
    game_state: Res<GameState>,
    simulation: Res<Simulation>,
) {
    // Update date display
    if let Ok(mut text) = queries.p0().get_single_mut() {
        text.sections[0].value = game_state.get_formatted_date();
    }
    
    // Update turn display
    if let Ok(mut text) = queries.p1().get_single_mut() {
        text.sections[0].value = format!("Turn: {}", simulation.current_turn);
    }
}

pub fn handle_end_turn_button(
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>, With<EndTurnButton>)>,
    mut turn_events: EventWriter<TurnEndEvent>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::ORANGE_RED.into();
                turn_events.send(TurnEndEvent);
            }
            Interaction::Hovered => {
                *color = Color::ORANGE.into();
            }
            Interaction::None => {
                *color = Color::DARK_GRAY.into();
            }
        }
    }
}
