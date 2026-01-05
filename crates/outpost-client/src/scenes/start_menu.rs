//! Start Menu scene implementation
//!
//! Displays the main menu with options: New Game, Load Game, Settings, About, Exit

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::scenes::Scene;
use crate::storage::{Storage, SaveProfile};
use crate::{Game, StorageManager};

/// Event emitted when a menu button is clicked
#[derive(Event, Debug, Clone)]
pub struct MenuButtonClicked {
    pub button_name: String,
}

/// Resource to track start menu state
#[derive(Resource, Default)]
pub struct StartMenuState {
    pub show_load_list: bool,
    pub available_saves: Vec<SaveProfile>,
    pub selected_save: Option<String>,
    pub error_message: Option<String>,
}

/// System that renders the start menu UI
pub fn start_menu_ui_system(
    mut contexts: EguiContexts,
    mut scene: ResMut<Scene>,
    mut menu_state: ResMut<StartMenuState>,
    mut game: ResMut<Game>,
    mut storage_mgr: ResMut<StorageManager>,
    mut menu_events: EventWriter<MenuButtonClicked>,
    mut app_exit: EventWriter<AppExit>,
) {
    // Only show start menu when in StartMenu scene
    if *scene != Scene::StartMenu {
        return;
    }

    let ctx = contexts.ctx_mut();

    // Center panel with menu
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);

            // Title
            ui.heading(egui::RichText::new("OUTPOST 3").size(48.0));
            ui.add_space(20.0);
            ui.label(egui::RichText::new("Desktop/WASM Edition").size(16.0));
            ui.add_space(60.0);

            // Menu buttons
            let button_width = 200.0;
            let button_height = 40.0;

            // New Game
            if ui.add_sized([button_width, button_height], egui::Button::new("New Game")).clicked() {
                info!("Start menu button clicked: New Game");
                menu_events.send(MenuButtonClicked { button_name: "New Game".to_string() });

                // Reset game state to defaults
                *game = Game::default();

                // Transition to GamePlay scene
                scene.transition_to(Scene::GamePlay);
            }

            ui.add_space(10.0);

            // Load Game
            if ui.add_sized([button_width, button_height], egui::Button::new("Load Game")).clicked() {
                info!("Start menu button clicked: Load Game");
                menu_events.send(MenuButtonClicked { button_name: "Load Game".to_string() });

                // Toggle load list and refresh available saves
                menu_state.show_load_list = !menu_state.show_load_list;
                if menu_state.show_load_list {
                    match storage_mgr.store.list_profiles() {
                        Ok(profiles) => {
                            menu_state.available_saves = profiles;
                            menu_state.error_message = None;
                        }
                        Err(e) => {
                            menu_state.error_message = Some(format!("Failed to list saves: {}", e));
                        }
                    }
                }
            }

            ui.add_space(10.0);

            // Settings
            if ui.add_sized([button_width, button_height], egui::Button::new("Settings")).clicked() {
                info!("Start menu button clicked: Settings");
                menu_events.send(MenuButtonClicked { button_name: "Settings".to_string() });
                scene.transition_to(Scene::Settings);
            }

            ui.add_space(10.0);

            // About
            if ui.add_sized([button_width, button_height], egui::Button::new("About")).clicked() {
                info!("Start menu button clicked: About");
                menu_events.send(MenuButtonClicked { button_name: "About".to_string() });
                scene.transition_to(Scene::About);
            }

            ui.add_space(10.0);

            // Exit
            if ui.add_sized([button_width, button_height], egui::Button::new("Exit")).clicked() {
                info!("Start menu button clicked: Exit");
                menu_events.send(MenuButtonClicked { button_name: "Exit".to_string() });

                // Send app exit event
                app_exit.send(AppExit);
            }

            ui.add_space(20.0);

            // Show error message if any
            if let Some(err) = &menu_state.error_message {
                ui.colored_label(egui::Color32::RED, err);
            }
        });
    });

    // Load Game modal window
    if menu_state.show_load_list {
        let mut open = menu_state.show_load_list;
        egui::Window::new("Load Game")
            .open(&mut open)
            .default_width(400.0)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Select a saved game to load:");
                ui.separator();

                if menu_state.available_saves.is_empty() {
                    ui.label("No saved games found.");
                } else {
                    let mut clicked_profile: Option<String> = None;
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for profile in &menu_state.available_saves {
                                let is_selected = menu_state.selected_save.as_ref() == Some(&profile.id);
                                let display = if !profile.label.is_empty() {
                                    format!("{} ({})", profile.label, profile.id)
                                } else {
                                    profile.id.clone()
                                };
                                if ui.selectable_label(is_selected, display).clicked() {
                                    clicked_profile = Some(profile.id.clone());
                                }
                            }
                        });
                    if let Some(id) = clicked_profile {
                        menu_state.selected_save = Some(id);
                    }
                }

                ui.separator();
                ui.horizontal(|ui| {
                    // Load button
                    let can_load = menu_state.selected_save.is_some();
                    if ui.add_enabled(can_load, egui::Button::new("Load")).clicked() {
                        if let Some(profile) = &menu_state.selected_save {
                            match storage_mgr.store.load(profile) {
                                Ok(loaded_state) => {
                                    info!("Loaded save: {}", profile);
                                    game.state = loaded_state.clone();
                                    game.history.clear();
                                    game.history.push_back(loaded_state);
                                    storage_mgr.profile_id = profile.clone();

                                    // Transition to gameplay
                                    scene.transition_to(Scene::GamePlay);
                                    menu_state.show_load_list = false;
                                    menu_state.error_message = None;
                                }
                                Err(e) => {
                                    menu_state.error_message = Some(format!("Failed to load: {}", e));
                                }
                            }
                        }
                    }

                    // Cancel button
                    if ui.button("Cancel").clicked() {
                        menu_state.show_load_list = false;
                        menu_state.selected_save = None;
                        menu_state.error_message = None;
                    }
                });
            });
        menu_state.show_load_list = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_menu_state_defaults() {
        let state = StartMenuState::default();
        assert!(!state.show_load_list);
        assert!(state.available_saves.is_empty());
        assert!(state.selected_save.is_none());
        assert!(state.error_message.is_none());
    }

    #[test]
    fn test_menu_button_clicked_event() {
        let event = MenuButtonClicked {
            button_name: "New Game".to_string(),
        };
        assert_eq!(event.button_name, "New Game");
    }

    // Integration test: verify scene transitions work correctly
    #[test]
    fn test_scene_transitions_from_start_menu() {
        let mut scene = Scene::StartMenu;

        // Can transition to GamePlay
        scene.transition_to(Scene::GamePlay);
        assert_eq!(scene, Scene::GamePlay);

        // Reset for next test
        scene = Scene::StartMenu;

        // Can transition to Settings
        scene.transition_to(Scene::Settings);
        assert_eq!(scene, Scene::Settings);

        // Reset for next test
        scene = Scene::StartMenu;

        // Can transition to About
        scene.transition_to(Scene::About);
        assert_eq!(scene, Scene::About);
    }
}
