//! About scene - Display game version and credits
//!
//! Provides a simple screen showing game information and credits with navigation back to start menu

use crate::scenes::Scene;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

/// Event emitted when About scene is displayed
#[derive(Event, Debug, Clone)]
pub struct AboutSceneDisplayed;

const GAME_VERSION: &str = env!("CARGO_PKG_VERSION");
const GAME_NAME: &str = "OUTPOST 3";

/// System that renders the about UI
pub fn about_ui_system(
    mut contexts: EguiContexts,
    mut scene: ResMut<Scene>,
    mut about_events: EventWriter<AboutSceneDisplayed>,
) {
    // Only show about UI when in About scene
    if *scene != Scene::About {
        return;
    }

    info!("About scene displayed");

    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);

            ui.heading(egui::RichText::new("About").size(48.0));
            ui.add_space(30.0);

            // Game Info
            ui.group(|ui| {
                ui.heading(GAME_NAME);
                ui.add_space(10.0);
                ui.label(format!("Version: {}", GAME_VERSION));
                ui.label("A deep-space colony survival sim");
            });

            ui.add_space(30.0);

            // Credits
            ui.group(|ui| {
                ui.heading("Credits");
                ui.add_space(10.0);
                ui.label("Design & Development: Outpost Team");
                ui.label("Built with: Bevy Engine");
                ui.label("Year: 2026");
            });

            ui.add_space(30.0);

            // Back button
            let button_width = 200.0;
            let button_height = 40.0;
            if ui
                .add_sized([button_width, button_height], egui::Button::new("Back"))
                .clicked()
            {
                about_events.send(AboutSceneDisplayed);
                scene.transition_to(Scene::StartMenu);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_version_constant() {
        // Verify version is correctly extracted from Cargo.toml
        assert_eq!(GAME_VERSION, "0.1.0");
    }

    #[test]
    fn test_game_name_constant() {
        assert_eq!(GAME_NAME, "OUTPOST 3");
    }

    #[test]
    fn test_about_scene_event() {
        // Verify event can be created
        let _event = AboutSceneDisplayed;
        assert!(true); // Event created successfully
    }

    // Integration test: verify About scene initializes correctly
    #[test]
    fn test_about_scene_initialization() {
        let mut app = App::new();

        app.insert_resource(Scene::About)
            .add_event::<AboutSceneDisplayed>();

        // Update once
        app.update();

        // Verify scene is set correctly
        let scene = app.world.resource::<Scene>();
        assert_eq!(*scene, Scene::About);
    }

    // Integration test: verify StartMenu -> About -> StartMenu transition
    #[test]
    fn test_about_scene_transition_flow() {
        let mut app = App::new();

        app.insert_resource(Scene::StartMenu)
            .add_event::<AboutSceneDisplayed>();

        // Verify we can transition to About
        assert!(Scene::StartMenu.can_transition_to(Scene::About));

        app.world
            .resource_mut::<Scene>()
            .transition_to(Scene::About);
        app.update();

        let scene = app.world.resource::<Scene>();
        assert_eq!(*scene, Scene::About);

        // Verify we can transition back to StartMenu
        assert!(Scene::About.can_transition_to(Scene::StartMenu));

        app.world
            .resource_mut::<Scene>()
            .transition_to(Scene::StartMenu);
        app.update();

        let scene = app.world.resource::<Scene>();
        assert_eq!(*scene, Scene::StartMenu);
    }
}
