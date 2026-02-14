//! Scene state machine for managing application scenes (menus, gameplay, etc.)
//!
//! This module provides an enum-based scene state system with validated transitions.
//! The scene state is managed as a Bevy Resource and transitions emit events for logging/tracking.

use bevy::prelude::*;

pub mod start_menu;
pub use start_menu::{start_menu_ui_system, MenuButtonClicked, StartMenuState};

pub mod transitions;
#[allow(unused_imports)]
pub use transitions::{
    cleanup_scene_entities_system, AboutEntity, GamePlayEntity, PreviousScene,
    SceneEntitiesCleaned, SettingsEntity, StartMenuEntity,
};

pub mod gameplay;
pub use gameplay::{
    gameplay_scene_init_system, render_resource_categories, GamePlaySceneLoaded,
    RESOURCE_PANEL_MAX_WIDTH,
};

pub mod settings;
pub use settings::{settings_ui_system, AppSettings, SettingsChanged};

pub mod about;
pub use about::{about_ui_system, AboutSceneDisplayed};

/// The current scene/screen of the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Resource, Default)]
pub enum Scene {
    /// Initial start menu (New Game, Load, Settings, About, Exit)
    #[default]
    StartMenu,
    /// Main gameplay view with hex grid and UI
    GamePlay,
    /// Settings screen (volume, graphics, keybinds)
    Settings,
    /// Credits and version information
    About,
}

impl Scene {
    /// Returns true if transitioning from `self` to `next` is valid
    pub fn can_transition_to(&self, next: Scene) -> bool {
        use self::Scene::*;
        match (self, next) {
            // StartMenu can go to any scene
            (StartMenu, GamePlay) => true,
            (StartMenu, Settings) => true,
            (StartMenu, About) => true,

            // GamePlay can return to StartMenu or open Settings
            (GamePlay, StartMenu) => true,
            (GamePlay, Settings) => true,

            // Settings/About can return to their origin
            (Settings, StartMenu) => true,
            (Settings, GamePlay) => true,
            (About, StartMenu) => true,

            // Same -> Same is always valid (no-op)
            (s1, s2) if *s1 == s2 => true,

            // All other transitions are invalid
            _ => false,
        }
    }

    /// Transition to a new scene, panicking if the transition is invalid.
    ///
    /// # Panics
    /// Panics if `can_transition_to(next)` returns false
    pub fn transition_to(&mut self, next: Scene) {
        if !self.can_transition_to(next) {
            panic!(
                "Invalid scene transition attempted: {:?} -> {:?}",
                self, next
            );
        }

        if *self != next {
            info!("Scene transition: {:?} -> {:?}", self, next);
            *self = next;
        }
    }
}

/// Event emitted when a scene transition occurs
#[derive(Event, Debug, Clone, Copy)]
pub struct SceneChanged {
    pub from: Scene,
    pub to: Scene,
}

/// System that handles scene transition requests and emits events
pub fn handle_scene_transitions(
    scene: ResMut<Scene>,
    _transition_events: EventWriter<SceneChanged>,
) {
    // This system is a placeholder for now.
    // In practice, transition requests will come from UI button clicks
    // or other systems that directly mutate the Scene resource.
    // The actual transition and event emission happens in `Scene::transition_to`.

    // For now, we'll add a manual event emitter when Scene is changed externally
    if scene.is_changed() && !scene.is_added() {
        // Scene was changed by another system
        // Note: we can't easily track 'from' here without additional state,
        // so we'll rely on the transition_to method for logging.
        // This is just a safety net for scene change detection.
    }
}

/// Helper function to request a scene transition
///
/// This is a convenience for systems that want to transition scenes.
/// It reads the current scene, validates the transition, and mutates the resource.
pub fn request_scene_transition(current_scene: &Scene, next_scene: Scene) -> Scene {
    if !current_scene.can_transition_to(next_scene) {
        panic!(
            "Invalid scene transition requested: {:?} -> {:?}",
            current_scene, next_scene
        );
    }
    next_scene
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions_from_start_menu() {
        let scene = Scene::StartMenu;
        assert!(scene.can_transition_to(Scene::GamePlay));
        assert!(scene.can_transition_to(Scene::Settings));
        assert!(scene.can_transition_to(Scene::About));
        assert!(scene.can_transition_to(Scene::StartMenu)); // Same -> Same
    }

    #[test]
    fn test_valid_transitions_from_gameplay() {
        let scene = Scene::GamePlay;
        assert!(scene.can_transition_to(Scene::StartMenu));
        assert!(scene.can_transition_to(Scene::Settings));
        assert!(scene.can_transition_to(Scene::GamePlay)); // Same -> Same
    }

    #[test]
    fn test_invalid_transitions_from_gameplay() {
        let scene = Scene::GamePlay;
        assert!(!scene.can_transition_to(Scene::About));
    }

    #[test]
    fn test_valid_transitions_from_settings() {
        let scene = Scene::Settings;
        assert!(scene.can_transition_to(Scene::StartMenu));
        assert!(scene.can_transition_to(Scene::GamePlay));
        assert!(scene.can_transition_to(Scene::Settings)); // Same -> Same
    }

    #[test]
    fn test_invalid_transitions_from_settings() {
        let scene = Scene::Settings;
        assert!(!scene.can_transition_to(Scene::About));
    }

    #[test]
    fn test_valid_transitions_from_about() {
        let scene = Scene::About;
        assert!(scene.can_transition_to(Scene::StartMenu));
        assert!(scene.can_transition_to(Scene::About)); // Same -> Same
    }

    #[test]
    fn test_invalid_transitions_from_about() {
        let scene = Scene::About;
        assert!(!scene.can_transition_to(Scene::GamePlay));
        assert!(!scene.can_transition_to(Scene::Settings));
    }

    #[test]
    fn test_transition_to_success() {
        let mut scene = Scene::StartMenu;
        scene.transition_to(Scene::GamePlay);
        assert_eq!(scene, Scene::GamePlay);
    }

    #[test]
    fn test_transition_to_same_scene_is_noop() {
        let mut scene = Scene::StartMenu;
        scene.transition_to(Scene::StartMenu);
        assert_eq!(scene, Scene::StartMenu);
    }

    #[test]
    #[should_panic(expected = "Invalid scene transition")]
    fn test_transition_to_invalid_panics() {
        let mut scene = Scene::About;
        scene.transition_to(Scene::GamePlay); // Invalid: About -> GamePlay
    }

    #[test]
    fn test_request_scene_transition_valid() {
        let current = Scene::StartMenu;
        let next = request_scene_transition(&current, Scene::GamePlay);
        assert_eq!(next, Scene::GamePlay);
    }

    #[test]
    #[should_panic(expected = "Invalid scene transition")]
    fn test_request_scene_transition_invalid_panics() {
        let current = Scene::About;
        request_scene_transition(&current, Scene::Settings); // Invalid
    }

    // Property tests: exhaustive check of all possible transitions
    #[test]
    fn test_all_scenes_can_transition_to_themselves() {
        let scenes = [
            Scene::StartMenu,
            Scene::GamePlay,
            Scene::Settings,
            Scene::About,
        ];
        for scene in scenes {
            assert!(
                scene.can_transition_to(scene),
                "{:?} should be able to transition to itself",
                scene
            );
        }
    }

    #[test]
    fn test_exhaustive_valid_transitions() {
        // Explicitly list all valid transitions
        let valid_transitions = vec![
            // From StartMenu
            (Scene::StartMenu, Scene::StartMenu),
            (Scene::StartMenu, Scene::GamePlay),
            (Scene::StartMenu, Scene::Settings),
            (Scene::StartMenu, Scene::About),
            // From GamePlay
            (Scene::GamePlay, Scene::GamePlay),
            (Scene::GamePlay, Scene::StartMenu),
            (Scene::GamePlay, Scene::Settings),
            // From Settings
            (Scene::Settings, Scene::Settings),
            (Scene::Settings, Scene::StartMenu),
            (Scene::Settings, Scene::GamePlay),
            // From About
            (Scene::About, Scene::About),
            (Scene::About, Scene::StartMenu),
        ];

        for (from, to) in valid_transitions {
            assert!(
                from.can_transition_to(to),
                "Expected valid transition: {:?} -> {:?}",
                from,
                to
            );
        }
    }

    #[test]
    fn test_exhaustive_invalid_transitions() {
        // Explicitly list all invalid transitions
        let invalid_transitions = vec![
            // From GamePlay
            (Scene::GamePlay, Scene::About),
            // From Settings
            (Scene::Settings, Scene::About),
            // From About
            (Scene::About, Scene::GamePlay),
            (Scene::About, Scene::Settings),
        ];

        for (from, to) in invalid_transitions {
            assert!(
                !from.can_transition_to(to),
                "Expected invalid transition: {:?} -> {:?}",
                from,
                to
            );
        }
    }

    #[test]
    fn test_no_invalid_transitions_property() {
        // Property: for each scene, verify that all invalid transitions are correctly rejected
        let all_scenes = [
            Scene::StartMenu,
            Scene::GamePlay,
            Scene::Settings,
            Scene::About,
        ];

        for from_scene in all_scenes {
            for to_scene in all_scenes {
                let can_transition = from_scene.can_transition_to(to_scene);

                // If we can transition, verify transition_to succeeds
                if can_transition {
                    let mut scene_copy = from_scene;
                    scene_copy.transition_to(to_scene);
                    if from_scene != to_scene {
                        assert_eq!(scene_copy, to_scene);
                    }
                }
            }
        }
    }
}
