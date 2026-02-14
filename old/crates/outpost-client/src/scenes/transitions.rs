//! Scene transition system with entity cleanup
//!
//! This module manages smooth transitions between scenes by cleaning up entities
//! that belong to the previous scene, preventing entity leaks.

use bevy::prelude::*;
use crate::scenes::Scene;

/// Event emitted when scene entities are cleaned up during a transition
#[derive(Event, Debug, Clone)]
pub struct SceneEntitiesCleaned {
    pub from_scene: Scene,
    pub entity_count: usize,
}

/// Marker component for entities that belong to the StartMenu scene
#[derive(Component, Debug, Clone, Copy)]
pub struct StartMenuEntity;

/// Marker component for entities that belong to the GamePlay scene
#[derive(Component, Debug, Clone, Copy)]
pub struct GamePlayEntity;

/// Marker component for entities that belong to the Settings scene
#[derive(Component, Debug, Clone, Copy)]
pub struct SettingsEntity;

/// Marker component for entities that belong to the About scene
#[derive(Component, Debug, Clone, Copy)]
pub struct AboutEntity;

/// Resource to track the previous scene for detecting transitions
#[derive(Resource, Default)]
pub struct PreviousScene {
    pub scene: Option<Scene>,
}

/// System that detects scene changes and cleans up entities from the previous scene
pub fn cleanup_scene_entities_system(
    current_scene: Res<Scene>,
    mut previous_scene: ResMut<PreviousScene>,
    mut commands: Commands,
    mut cleanup_events: EventWriter<SceneEntitiesCleaned>,
    // Query all entities with scene markers
    start_menu_entities: Query<Entity, With<StartMenuEntity>>,
    gameplay_entities: Query<Entity, With<GamePlayEntity>>,
    settings_entities: Query<Entity, With<SettingsEntity>>,
    about_entities: Query<Entity, With<AboutEntity>>,
) {
    // Check if scene has changed
    if let Some(prev) = previous_scene.scene {
        if prev != *current_scene {
            // Scene transition detected - clean up entities from previous scene
            let count = match prev {
                Scene::StartMenu => {
                    let mut count = 0;
                    for entity in start_menu_entities.iter() {
                        commands.entity(entity).despawn_recursive();
                        count += 1;
                    }
                    count
                }
                Scene::GamePlay => {
                    let mut count = 0;
                    for entity in gameplay_entities.iter() {
                        commands.entity(entity).despawn_recursive();
                        count += 1;
                    }
                    count
                }
                Scene::Settings => {
                    let mut count = 0;
                    for entity in settings_entities.iter() {
                        commands.entity(entity).despawn_recursive();
                        count += 1;
                    }
                    count
                }
                Scene::About => {
                    let mut count = 0;
                    for entity in about_entities.iter() {
                        commands.entity(entity).despawn_recursive();
                        count += 1;
                    }
                    count
                }
            };

            if count > 0 {
                info!("Despawning {} entities for scene {:?}", count, prev);
                cleanup_events.send(SceneEntitiesCleaned {
                    from_scene: prev,
                    entity_count: count,
                });
            }
        }
    }

    // Update previous scene
    previous_scene.scene = Some(*current_scene);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_previous_scene_default() {
        let prev = PreviousScene::default();
        assert!(prev.scene.is_none());
    }

    #[test]
    fn test_scene_entities_cleaned_event() {
        let event = SceneEntitiesCleaned {
            from_scene: Scene::StartMenu,
            entity_count: 5,
        };
        assert_eq!(event.from_scene, Scene::StartMenu);
        assert_eq!(event.entity_count, 5);
    }

    #[test]
    fn test_marker_components_exist() {
        // Test that all marker components can be instantiated
        let _start = StartMenuEntity;
        let _game = GamePlayEntity;
        let _settings = SettingsEntity;
        let _about = AboutEntity;
    }

    // Integration test: verify cleanup system despawns entities
    #[test]
    fn test_cleanup_system_despawns_entities() {
        let mut app = App::new();

        // Add minimal plugins and resources
        app.insert_resource(Scene::StartMenu)
            .insert_resource(PreviousScene::default())
            .add_event::<SceneEntitiesCleaned>()
            .add_systems(Update, cleanup_scene_entities_system);

        // Spawn some StartMenu entities
        app.world.spawn(StartMenuEntity);
        app.world.spawn(StartMenuEntity);
        app.world.spawn(StartMenuEntity);

        // Run once to establish previous scene
        app.update();

        // Verify entities exist
        let start_count: usize = app.world.query::<&StartMenuEntity>().iter(&app.world).count();
        assert_eq!(start_count, 3, "Should have 3 StartMenu entities");

        // Change scene to GamePlay
        app.world.resource_mut::<Scene>().transition_to(Scene::GamePlay);

        // Run update to trigger cleanup
        app.update();

        // Verify StartMenu entities were despawned
        let remaining: usize = app.world.query::<&StartMenuEntity>().iter(&app.world).count();
        assert_eq!(remaining, 0, "All StartMenu entities should be despawned");

        // Verify event was emitted
        let mut events = app.world.resource_mut::<Events<SceneEntitiesCleaned>>();
        let mut reader = events.get_reader();
        let event_count = reader.read(&events).count();
        assert_eq!(event_count, 1, "Should have emitted one cleanup event");
    }

    // Property test: verify no entities leak across multiple transitions
    #[test]
    fn test_no_entity_leaks_across_transitions() {
        let mut app = App::new();

        app.insert_resource(Scene::StartMenu)
            .insert_resource(PreviousScene::default())
            .add_event::<SceneEntitiesCleaned>()
            .add_systems(Update, cleanup_scene_entities_system);

        // Valid transition path: StartMenu -> GamePlay -> Settings -> StartMenu -> About -> StartMenu
        let transition_path = [
            Scene::StartMenu,
            Scene::GamePlay,
            Scene::Settings,
            Scene::StartMenu,
            Scene::About,
            Scene::StartMenu,
        ];

        for (i, scene) in transition_path.iter().enumerate() {
            // Spawn entities for current scene
            match scene {
                Scene::StartMenu => {
                    app.world.spawn(StartMenuEntity);
                    app.world.spawn(StartMenuEntity);
                }
                Scene::GamePlay => {
                    app.world.spawn(GamePlayEntity);
                    app.world.spawn(GamePlayEntity);
                }
                Scene::Settings => {
                    app.world.spawn(SettingsEntity);
                    app.world.spawn(SettingsEntity);
                }
                Scene::About => {
                    app.world.spawn(AboutEntity);
                    app.world.spawn(AboutEntity);
                }
            }

            // Transition to this scene
            if i > 0 {
                app.world.resource_mut::<Scene>().transition_to(*scene);
            }

            app.update();

            // Verify only current scene's entities exist
            let start_count: usize = app.world.query::<&StartMenuEntity>().iter(&app.world).count();
            let game_count: usize = app.world.query::<&GamePlayEntity>().iter(&app.world).count();
            let settings_count: usize = app.world.query::<&SettingsEntity>().iter(&app.world).count();
            let about_count: usize = app.world.query::<&AboutEntity>().iter(&app.world).count();

            match scene {
                Scene::StartMenu => {
                    assert!(start_count >= 2, "Should have StartMenu entities");
                    assert_eq!(game_count, 0, "Should have no GamePlay entities");
                    assert_eq!(settings_count, 0, "Should have no Settings entities");
                    assert_eq!(about_count, 0, "Should have no About entities");
                }
                Scene::GamePlay => {
                    assert_eq!(start_count, 0, "Should have no StartMenu entities");
                    assert_eq!(game_count, 2, "Should have GamePlay entities");
                    assert_eq!(settings_count, 0, "Should have no Settings entities");
                    assert_eq!(about_count, 0, "Should have no About entities");
                }
                Scene::Settings => {
                    assert_eq!(start_count, 0, "Should have no StartMenu entities");
                    assert_eq!(game_count, 0, "Should have no GamePlay entities");
                    assert_eq!(settings_count, 2, "Should have Settings entities");
                    assert_eq!(about_count, 0, "Should have no About entities");
                }
                Scene::About => {
                    assert_eq!(start_count, 0, "Should have no StartMenu entities");
                    assert_eq!(game_count, 0, "Should have no GamePlay entities");
                    assert_eq!(settings_count, 0, "Should have no Settings entities");
                    assert_eq!(about_count, 2, "Should have About entities");
                }
            }
        }
    }

    // Test that entities without markers are not despawned
    #[test]
    fn test_unmarked_entities_persist() {
        let mut app = App::new();

        app.insert_resource(Scene::StartMenu)
            .insert_resource(PreviousScene::default())
            .add_event::<SceneEntitiesCleaned>()
            .add_systems(Update, cleanup_scene_entities_system);

        // Spawn marked and unmarked entities
        app.world.spawn(StartMenuEntity);
        app.world.spawn(()); // Unmarked entity

        app.update();

        // Transition to GamePlay
        app.world.resource_mut::<Scene>().transition_to(Scene::GamePlay);
        app.update();

        // Verify unmarked entity still exists (total entities > 0)
        let total_entities = app.world.entities().len();
        assert!(total_entities > 0, "Unmarked entities should persist");
    }
}
