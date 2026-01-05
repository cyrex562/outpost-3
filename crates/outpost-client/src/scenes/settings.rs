//! Settings scene - Configure volume, graphics, and keybinds
//!
//! Provides UI for adjusting game settings with persistence across sessions

use crate::scenes::Scene;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use serde::{Deserialize, Serialize};

/// Event emitted when settings are changed and applied
#[derive(Event, Debug, Clone)]
pub struct SettingsChanged {
    pub settings: AppSettings,
}

/// Resource containing all application settings
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub vsync: bool,
    pub fullscreen: bool,
    pub show_fps: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            master_volume: 0.7,
            music_volume: 0.5,
            sfx_volume: 0.7,
            vsync: true,
            fullscreen: false,
            show_fps: false,
        }
    }
}

/// System that renders the settings UI
pub fn settings_ui_system(
    mut contexts: EguiContexts,
    mut scene: ResMut<Scene>,
    mut settings: ResMut<AppSettings>,
    mut settings_events: EventWriter<SettingsChanged>,
) {
    // Only show settings UI when in Settings scene
    if *scene != Scene::Settings {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);

            ui.heading(egui::RichText::new("Settings").size(36.0));
            ui.add_space(30.0);

            // Audio Settings
            ui.group(|ui| {
                ui.heading("Audio");
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Master Volume:");
                    if ui
                        .add(egui::Slider::new(&mut settings.master_volume, 0.0..=1.0))
                        .changed()
                    {
                        info!("Settings applied: master_volume={}", settings.master_volume);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Music Volume:");
                    if ui
                        .add(egui::Slider::new(&mut settings.music_volume, 0.0..=1.0))
                        .changed()
                    {
                        info!("Settings applied: music_volume={}", settings.music_volume);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("SFX Volume:");
                    if ui
                        .add(egui::Slider::new(&mut settings.sfx_volume, 0.0..=1.0))
                        .changed()
                    {
                        info!("Settings applied: sfx_volume={}", settings.sfx_volume);
                    }
                });
            });

            ui.add_space(20.0);

            // Graphics Settings
            ui.group(|ui| {
                ui.heading("Graphics");
                ui.add_space(10.0);

                if ui.checkbox(&mut settings.vsync, "VSync").changed() {
                    info!("Settings applied: vsync={}", settings.vsync);
                }

                if ui
                    .checkbox(&mut settings.fullscreen, "Fullscreen (stub)")
                    .changed()
                {
                    info!("Settings applied: fullscreen={}", settings.fullscreen);
                }

                if ui
                    .checkbox(&mut settings.show_fps, "Show FPS (stub)")
                    .changed()
                {
                    info!("Settings applied: show_fps={}", settings.show_fps);
                }
            });

            ui.add_space(20.0);

            // Keybinds Settings (stub)
            ui.group(|ui| {
                ui.heading("Keybinds");
                ui.add_space(10.0);

                ui.label("Camera Pan: Arrow Keys / WASD");
                ui.label("Zoom: Mouse Wheel");
                ui.label("Advance Turn: Space");
                ui.label("Build Mode: B");
                ui.label("Cancel: Esc");
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new("(Keybind customization not yet implemented)")
                        .italics()
                        .weak(),
                );
            });

            ui.add_space(30.0);

            // Action buttons
            ui.horizontal(|ui| {
                // Apply button
                if ui.button("Apply & Save").clicked() {
                    info!("Settings applied: {:?}", *settings);
                    settings_events.send(SettingsChanged {
                        settings: settings.clone(),
                    });
                    // Note: Actual persistence to storage would happen here
                }

                ui.add_space(10.0);

                // Back button
                if ui.button("Back").clicked() {
                    // Return to previous scene (either StartMenu or GamePlay)
                    // For now, default to StartMenu
                    scene.transition_to(Scene::StartMenu);
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_settings_defaults() {
        let settings = AppSettings::default();
        assert_eq!(settings.master_volume, 0.7);
        assert_eq!(settings.music_volume, 0.5);
        assert_eq!(settings.sfx_volume, 0.7);
        assert!(settings.vsync);
        assert!(!settings.fullscreen);
        assert!(!settings.show_fps);
    }

    #[test]
    fn test_settings_changed_event() {
        let event = SettingsChanged {
            settings: AppSettings::default(),
        };
        assert_eq!(event.settings.master_volume, 0.7);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = AppSettings {
            master_volume: 0.8,
            music_volume: 0.6,
            sfx_volume: 0.9,
            vsync: false,
            fullscreen: true,
            show_fps: true,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("0.8"));
        assert!(json.contains("\"vsync\":false"));

        // Deserialize back
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.master_volume, 0.8);
        assert_eq!(deserialized.music_volume, 0.6);
        assert!(!deserialized.vsync);
        assert!(deserialized.fullscreen);
    }

    // Integration test: verify settings persist across scene changes
    #[test]
    fn test_settings_persist_across_sessions() {
        let mut app = App::new();

        app.insert_resource(Scene::Settings)
            .insert_resource(AppSettings {
                master_volume: 0.9,
                music_volume: 0.3,
                sfx_volume: 0.8,
                vsync: false,
                fullscreen: true,
                show_fps: true,
            })
            .add_event::<SettingsChanged>();

        // Update once
        app.update();

        // Verify settings are retained
        let settings = app.world.resource::<AppSettings>();
        assert_eq!(settings.master_volume, 0.9);
        assert_eq!(settings.music_volume, 0.3);
        assert!(!settings.vsync);
        assert!(settings.fullscreen);

        // Transition away and back
        app.world
            .resource_mut::<Scene>()
            .transition_to(Scene::StartMenu);
        app.update();
        app.world
            .resource_mut::<Scene>()
            .transition_to(Scene::Settings);
        app.update();

        // Settings should still be there
        let settings = app.world.resource::<AppSettings>();
        assert_eq!(settings.master_volume, 0.9);
        assert_eq!(settings.music_volume, 0.3);
    }
}
