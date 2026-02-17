//! Simulation service managing game state and tick loop.
//!
//! This service:
//! - Holds the GameState in thread-safe storage (Arc<Mutex>)
//! - Runs a background tokio task that processes ticks at regular intervals
//! - Provides methods for controlling simulation (pause/resume/set_speed)
//! - Stores events generated during ticks

use outpost_core::domain::GameState;
use outpost_core::content::ContentLoader;
use outpost_core::events::GameEvent;
use outpost_core::simulation::{process_tick, GameClock, TimeCommand};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

/// Shared state for the simulation.
#[derive(Clone)]
pub struct SimulationService {
    /// Game state (mutable via Mutex for tick processing)
    state: Arc<Mutex<GameState>>,
    /// Content definitions (immutable, loaded at startup)
    content: Arc<ContentLoader>,
    /// Event log (append-only during ticks)
    events: Arc<RwLock<Vec<GameEvent>>>,
}

impl SimulationService {
    /// Create a new simulation service with the given initial state and content.
    pub fn new(initial_state: GameState, content: ContentLoader) -> Self {
        Self {
            state: Arc::new(Mutex::new(initial_state)),
            content: Arc::new(content),
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get a clone of the current game state (read-only).
    pub async fn get_state(&self) -> GameState {
        let state = self.state.lock().await;
        state.clone()
    }

    /// Execute a time command (pause, resume, set speed, etc.)
    pub async fn execute_time_command(&self, command: TimeCommand) -> Result<(), String> {
        let mut state = self.state.lock().await;
        command.execute(&mut state.clock)
    }

    /// Get the current tick count.
    pub async fn current_tick(&self) -> u64 {
        let state = self.state.lock().await;
        state.current_tick()
    }

    /// Check if the simulation is paused.
    pub async fn is_paused(&self) -> bool {
        let state = self.state.lock().await;
        state.is_paused()
    }

    /// Get the current speed multiplier.
    pub async fn speed_multiplier(&self) -> u32 {
        let state = self.state.lock().await;
        state.speed_multiplier()
    }

    /// Get formatted game time (e.g., "Day 5 14:00").
    pub async fn game_time(&self) -> String {
        let state = self.state.lock().await;
        state.clock.format_game_time()
    }

    /// Get a clone of all events generated so far.
    pub async fn get_events(&self) -> Vec<GameEvent> {
        let events = self.events.read().await;
        events.clone()
    }

    /// Get the count of events generated so far.
    pub async fn event_count(&self) -> usize {
        let events = self.events.read().await;
        events.len()
    }

    /// Start the simulation tick loop.
    ///
    /// This spawns a background tokio task that:
    /// 1. Calculates the effective tick rate based on speed multiplier
    /// 2. Processes a tick if not paused
    /// 3. Stores generated events
    /// 4. Checks idle safety (auto-pause on critical resource depletion)
    /// 5. Repeats at the appropriate interval
    ///
    /// The tick rate is recalculated each iteration to respond to speed changes.
    pub fn start_tick_loop(self) {
        tokio::spawn(async move {
            info!("Simulation tick loop started");

            loop {
                // Get effective tick rate (adjusted for speed multiplier)
                let tick_rate_ms = {
                    let state = self.state.lock().await;
                    state.clock.effective_tick_rate_ms()
                };

                // Wait for the appropriate interval
                tokio::time::sleep(Duration::from_millis(tick_rate_ms)).await;

                // Process the tick
                let tick_events = {
                    let mut state = self.state.lock().await;
                    process_tick(&mut *state, &self.content)
                };

                // Log tick processing
                if !tick_events.is_empty() {
                    let tick = self.current_tick().await;
                    debug!("Processed tick {}, generated {} events", tick, tick_events.len());

                    // Store events
                    let mut events = self.events.write().await;
                    events.extend(tick_events);
                }

                // Idle safety check: auto-pause if critical resources depleted
                self.check_idle_safety().await;
            }
        });
    }

    /// Check idle safety conditions and auto-pause if necessary.
    ///
    /// This checks if:
    /// - Idle safety is enabled
    /// - Simulation is not already paused
    /// - Any colony has critical resource depletion (food, water, oxygen, power at zero)
    ///
    /// If conditions are met, the simulation is automatically paused.
    async fn check_idle_safety(&self) {
        let state = self.state.lock().await;

        // Only check if idle safety is enabled and not suppressed
        if !state.clock.idle_safety_enabled || state.clock.suppress_autopause_in_idle {
            return;
        }

        // Don't check if already paused
        if state.is_paused() {
            return;
        }

        // TODO: Implement actual resource checking when colony resource system is ready
        // For now, this is a placeholder that will be implemented with production chains
        //
        // The logic should:
        // 1. Iterate through all colonies in state.galaxy
        // 2. Check each colony's resource stockpiles for:
        //    - Food <= 0
        //    - Water <= 0
        //    - Oxygen <= 0 (if applicable)
        //    - Power deficit > 0
        // 3. If any critical resource is depleted, log warning and pause
        //
        // Example pseudocode (will need mut state when implementing):
        // for system in state.galaxy.systems() {
        //     for site in system.sites() {
        //         let resources = site.resources();
        //         if resources.get(Food) <= 0 || resources.get(Water) <= 0 {
        //             warn!("Critical resource depletion at site {}, auto-pausing", site.id());
        //             drop(state); // Release lock
        //             let mut state = self.state.lock().await;
        //             state.pause();
        //             return;
        //         }
        //     }
        // }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_create_service() {
        let state = GameState::new("Test".to_string(), 42);
        let content = ContentLoader::new();
        let service = SimulationService::new(state, content);

        assert_eq!(service.current_tick().await, 0);
        assert!(!service.is_paused().await);
        assert_eq!(service.speed_multiplier().await, 1);
    }

    #[tokio::test]
    async fn test_execute_time_command_pause() {
        let state = GameState::new("Test".to_string(), 42);
        let content = ContentLoader::new();
        let service = SimulationService::new(state, content);

        service.execute_time_command(TimeCommand::Pause).await.unwrap();
        assert!(service.is_paused().await);
    }

    #[tokio::test]
    async fn test_execute_time_command_set_speed() {
        let state = GameState::new("Test".to_string(), 42);
        let content = ContentLoader::new();
        let service = SimulationService::new(state, content);

        service.execute_time_command(TimeCommand::SetSpeed(5)).await.unwrap();
        assert_eq!(service.speed_multiplier().await, 5);
    }

    #[tokio::test]
    async fn test_get_state() {
        let mut state = GameState::new("Test".to_string(), 42);
        state.advance_tick();
        let content = ContentLoader::new();
        let service = SimulationService::new(state, content);

        let retrieved_state = service.get_state().await;
        assert_eq!(retrieved_state.current_tick(), 1);
    }

    #[tokio::test]
    async fn test_game_time_format() {
        let state = GameState::new("Test".to_string(), 42);
        let content = ContentLoader::new();
        let service = SimulationService::new(state, content);

        let game_time = service.game_time().await;
        assert_eq!(game_time, "Day 0 00:00");
    }

    // Note: Testing the tick loop requires timing-based tests which can be flaky.
    // We test the tick processor separately and trust the integration.
}
