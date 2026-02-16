//! Tick processing for the simulation.
//!
//! The tick processor is a pure function that takes GameState and produces Events.
//! All game systems are processed here: production, consumption, population needs,
//! random events, etc.

use crate::domain::GameState;
use crate::events::{EventType, GameEvent};

/// Process a single simulation tick.
///
/// This is the main simulation step function. It:
/// 1. Checks if the simulation is paused  
/// 2. Advances the game clock by one tick
/// 3. Processes all game systems (production, population, events, etc.)
/// 4. Returns all events generated during the tick
///
/// # Arguments
/// * `state` - Mutable reference to game state to update
///
/// # Returns
/// Vector of events generated during the tick. Returns empty vec if paused.
///
/// # Note
/// Event IDs are managed by the caller (typically EventStore in the server layer).
/// This function creates events with placeholder event_id=0.
pub fn process_tick(state: &mut GameState) -> Vec<GameEvent> {
    // Don't process if paused
    if state.is_paused() {
        return vec![];
    }

    let mut events = vec![];

    // Advance the clock
    state.advance_tick();
    let new_tick = state.current_tick();

    // Generate tick event (event_id will be assigned by EventStore)
    events.push(GameEvent::new(
        0, // Placeholder event_id
        new_tick,
        EventType::TurnAdvanced {
            turn_number: new_tick,
        },
    ));

    // TODO: Process game systems in sequence:
    // - Production chains (buildings produce resources)
    // - Population consumption (colonists consume food, water, oxygen)
    // - Power grid balancing (batteries charge/discharge)
    // - Research progress (if tech system implemented)
    // - Random events (anomalies, failures, conflicts)
    // - Idle safety checks (auto-pause if critical resources depleted)

    events
}

/// Process multiple simulation ticks in batch.
///
/// This is useful for fast-forward scenarios or catching up after loading.
/// Each tick is processed sequentially with full event generation.
///
/// # Arguments
/// * `state` - Mutable reference to game state to update
/// * `count` - Number of ticks to process
///
/// # Returns
/// Vector of all events generated across all ticks.
pub fn process_ticks(state: &mut GameState, count: u64) -> Vec<GameEvent> {
    let mut all_events = vec![];
    
    for _ in 0..count {
        let tick_events = process_tick(state);
        all_events.extend(tick_events);
    }
    
    all_events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_tick_advances_clock() {
        let mut state = GameState::new("Test".to_string(), 42);
        let initial_tick = state.current_tick();
        
        let events = process_tick(&mut state);
        
        assert_eq!(state.current_tick(), initial_tick + 1);
        assert_eq!(events.len(), 1);
        match &events[0].event_type {
            EventType::TurnAdvanced { turn_number } => {
                assert_eq!(*turn_number, initial_tick + 1);
            }
            _ => panic!("Expected TurnAdvanced event"),
        }
    }

    #[test]
    fn test_process_tick_when_paused() {
        let mut state = GameState::new("Test".to_string(), 42);
        state.pause();
        let initial_tick = state.current_tick();
        
        let events = process_tick(&mut state);
        
        assert_eq!(state.current_tick(), initial_tick); // No change
        assert_eq!(events.len(), 0); // No events
    }

    #[test]
    fn test_process_multiple_ticks() {
        let mut state = GameState::new("Test".to_string(), 42);
        let initial_tick = state.current_tick();
        let count = 10;
        
        let events = process_ticks(&mut state, count);
        
        assert_eq!(state.current_tick(), initial_tick + count);
        assert_eq!(events.len(), count as usize); // One event per tick
    }

    #[test]
    fn test_process_ticks_respects_pause() {
        let mut state = GameState::new("Test".to_string(), 42);
        let initial_tick = state.current_tick();
        
        // Process 3 ticks normally
        process_ticks(&mut state, 3);
        assert_eq!(state.current_tick(), initial_tick + 3);
        
        // Pause and try to process more
        state.pause();
        let events = process_ticks(&mut state, 5);
        
        // Should not have advanced
        assert_eq!(state.current_tick(), initial_tick + 3);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_event_contains_turn_number() {
        let mut state = GameState::new("Test".to_string(), 42);
        
        let events = process_tick(&mut state);
        
        assert_eq!(events[0].turn_number, 1);
        match &events[0].event_type {
            EventType::TurnAdvanced { turn_number } => {
                assert_eq!(*turn_number, 1);
            }
            _ => panic!("Expected TurnAdvanced event"),
        }
    }

    #[test]
    fn test_multiple_ticks_increment_turn_number() {
        let mut state = GameState::new("Test".to_string(), 42);
        
        // Process 5 ticks
        let events = process_ticks(&mut state, 5);
        
        // Each event should have incrementing turn number
        for (i, event) in events.iter().enumerate() {
            let expected_turn = (i + 1) as u64;
            assert_eq!(event.turn_number, expected_turn);
            match &event.event_type {
                EventType::TurnAdvanced { turn_number } => {
                    assert_eq!(*turn_number, expected_turn);
                }
                _ => panic!("Expected TurnAdvanced event"),
            }
        }
    }
}
