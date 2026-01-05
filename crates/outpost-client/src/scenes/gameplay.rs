//! GamePlay scene - Main game view with hex grid and UI
//!
//! This module contains the main gameplay interface including:
//! - Resource bars and status displays
//! - Hex grid interaction (hover, selection, building placement)
//! - Construction and building detail modals
//! - Chart windows (power, resources, population)
//! - Camera controls and shortcuts

use bevy::prelude::*;
use bevy_egui::egui;
use crate::scenes::Scene;

/// Minimum width per resource column in the categorized resources panel.
pub const RESOURCE_COLUMN_MIN_WIDTH: f32 = 220.0;
/// Spacing between resource columns in the categorized resources panel.
pub const RESOURCE_COLUMN_SPACING: f32 = 12.0;
/// Guardrail for max width (matching 1920x1080 overflow requirement).
pub const RESOURCE_PANEL_MAX_WIDTH: f32 = 1920.0;

/// A UI-friendly grouping of resources by category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceCategory {
    pub name: &'static str,
    pub resources: &'static [&'static str],
}

/// Returns the catalog of resource categories, including the new energy/mineral resources.
pub fn resource_catalog() -> Vec<ResourceCategory> {
    vec![
        ResourceCategory {
            name: "Currency",
            resources: &["Credits"],
        },
        ResourceCategory {
            name: "Energy",
            resources: &["Energy", "Solar Power", "Nuclear Fuel", "Battery Pack", "Hydrogen Cell"],
        },
        ResourceCategory {
            name: "Raw Materials - Basic",
            resources: &["Iron Ore", "Copper Ore", "Rare Metals", "Water", "Timber", "Food"],
        },
        ResourceCategory {
            name: "Raw Materials - Minerals",
            resources: &["Nickel", "Cobalt", "Zinc", "Silver", "Gold", "Lithium", "Graphite"],
        },
        ResourceCategory {
            name: "Raw Materials - Advanced",
            resources: &["Coal", "Uranium", "Silicon", "Titanium", "Platinum", "Aluminum", "Crystal Ore", "Gas", "Oil"],
        },
        ResourceCategory {
            name: "Processed Goods - Basic",
            resources: &["Steel", "Electronics", "Machinery"],
        },
        ResourceCategory {
            name: "Processed Goods - Advanced",
            resources: &["Concrete", "Plastics", "Fuel", "Chemicals", "Advanced Components", "Medicine"],
        },
        ResourceCategory {
            name: "Specialized",
            resources: &["Research", "Consumer Goods", "Luxuries"],
        },
    ]
}

/// Compute recommended number of columns for the resources grid, respecting a max width.
pub fn recommend_resource_columns(max_width: f32) -> usize {
    let usable_width = max_width.clamp(RESOURCE_COLUMN_MIN_WIDTH, RESOURCE_PANEL_MAX_WIDTH);
    let column_span = RESOURCE_COLUMN_MIN_WIDTH + RESOURCE_COLUMN_SPACING;
    let cols = (usable_width / column_span).floor().max(1.0) as usize;
    cols.max(1)
}

/// Estimate panel width given a number of columns (used for tests/guardrails).
pub fn estimate_panel_width(columns: usize) -> f32 {
    if columns == 0 {
        return 0.0;
    }
    (columns as f32 * RESOURCE_COLUMN_MIN_WIDTH)
        + ((columns.saturating_sub(1)) as f32 * RESOURCE_COLUMN_SPACING)
}

/// Render the categorized resources UI block with simple stock placeholders derived from total stock.
pub fn render_resource_categories(ui: &mut egui::Ui, total_stock: i64, max_width: f32) {
    let categories = resource_catalog();
    let columns = recommend_resource_columns(max_width);
    let per_resource_stock = total_stock.max(0) / (categories.iter().map(|c| c.resources.len() as i64).sum::<i64>().max(1));

    ui.vertical(|ui| {
        ui.heading("Resources by Category");
        ui.separator();
        ui.columns(columns, |mut column_uis| {
            for (idx, category) in categories.iter().enumerate() {
                let column = &mut column_uis[idx % columns];
                column.group(|ui| {
                    ui.label(egui::RichText::new(category.name).strong());
                    ui.separator();
                    info!("Rendering {} resources in {}", category.resources.len(), category.name);
                    for res in category.resources {
                        ui.horizontal(|ui| {
                            ui.label(*res);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.monospace(format!("{}", per_resource_stock));
                            });
                        });
                    }
                });
            }
        });
    });
}

/// Event emitted when the GamePlay scene is loaded and initialized
#[derive(Event, Debug, Clone)]
pub struct GamePlaySceneLoaded {
    pub turn: u64,
}

/// System that logs when GamePlay scene becomes active
pub fn gameplay_scene_init_system(
    scene: Res<Scene>,
    game: Res<crate::Game>,
    mut events: EventWriter<GamePlaySceneLoaded>,
    mut initialized: Local<bool>,
) {
    if *scene == Scene::GamePlay && !*initialized {
        info!("GamePlay scene initialized with turn {}", game.state.turn);
        events.send(GamePlaySceneLoaded {
            turn: game.state.turn,
        });
        *initialized = true;
    } else if *scene != Scene::GamePlay && *initialized {
        // Reset when leaving GamePlay scene
        *initialized = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gameplay_scene_loaded_event() {
        let event = GamePlaySceneLoaded { turn: 42 };
        assert_eq!(event.turn, 42);
    }

    // Integration test: verify scene init emits event
    #[test]
    fn test_gameplay_init_emits_event() {
        let mut app = App::new();

        app.insert_resource(Scene::StartMenu)
            .insert_resource(crate::Game::default())
            .add_event::<GamePlaySceneLoaded>()
            .add_systems(Update, gameplay_scene_init_system);

        // Run once in StartMenu - should not emit
        app.update();

        // Transition to GamePlay
        app.world.resource_mut::<Scene>().transition_to(Scene::GamePlay);
        app.update();

        // Check event was emitted
        let events = app.world.resource::<Events<GamePlaySceneLoaded>>();
        let mut reader = events.get_reader();
        let emitted: Vec<_> = reader.read(&events).collect();
        assert_eq!(emitted.len(), 1, "Should emit event on GamePlay init");
        assert_eq!(emitted[0].turn, 0, "Default game has turn 0");

        // Run again - should not re-emit (event will auto-clear after 2 frames)
        app.update();
        app.update(); // Clear events

        let events = app.world.resource::<Events<GamePlaySceneLoaded>>();
        let event_count = events.iter_current_update_events().count();
        assert_eq!(event_count, 0, "Should not re-emit on subsequent frames");
    }

    // Test that shortcuts are accessible (basic smoke test)
    #[test]
    fn test_shortcuts_constants() {
        // This is a placeholder for testing that shortcuts work
        // In practice, we verify Space, B, Esc work in the actual systems
        // For now, just ensure the test module compiles
        assert!(true, "Shortcuts test placeholder");
    }

    #[test]
    fn resource_catalog_includes_new_energy_and_minerals() {
        let catalog = resource_catalog();
        let energy = catalog
            .iter()
            .find(|c| c.name == "Energy")
            .expect("Energy category present");
        assert!(energy.resources.contains(&"Solar Power"));
        assert!(energy.resources.contains(&"Hydrogen Cell"));

        let minerals = catalog
            .iter()
            .find(|c| c.name == "Raw Materials - Minerals")
            .expect("Minerals category present");
        assert!(minerals.resources.contains(&"Lithium"));
        assert!(minerals.resources.contains(&"Graphite"));
    }

    #[test]
    fn resource_panel_width_respects_full_hd() {
        let cols = recommend_resource_columns(1920.0);
        let width = estimate_panel_width(cols);
        assert!(width <= RESOURCE_PANEL_MAX_WIDTH, "Panel width should not overflow 1920px");
    }
}
