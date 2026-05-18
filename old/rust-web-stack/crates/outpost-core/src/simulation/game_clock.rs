//! GameClock — time tracking and control for the simulation.
//!
//! The GameClock manages the simulation's time state including:
//! - Current tick count
//! - Pause/resume state
//! - Speed multiplier
//! - Tick rate configuration

use serde::{Deserialize, Serialize};

/// Configuration for the game clock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockConfig {
    /// Tick rate in milliseconds (real-time per tick).
    pub tick_rate_ms: u64,
    /// Default speed multiplier.
    pub default_speed_multiplier: u32,
    /// Maximum allowed speed multiplier.
    pub max_speed_multiplier: u32,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: 60_000, // 1 minute real-time = 1 tick (1 game hour)
            default_speed_multiplier: 1,
            max_speed_multiplier: 10,
        }
    }
}

/// The game clock tracking simulation time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameClock {
    /// Current simulation tick (increments with each time step).
    pub current_tick: u64,
    /// Whether the simulation is paused.
    pub paused: bool,
    /// Speed multiplier (1 = normal, 2 = 2x, etc.).
    pub speed_multiplier: u32,
    /// Clock configuration.
    pub config: ClockConfig,
    /// Idle safety mode enabled (auto-pause on critical resource depletion).
    pub idle_safety_enabled: bool,
    /// Whether auto-pause is suppressed while in idle mode.
    pub suppress_autopause_in_idle: bool,
}

impl GameClock {
    /// Create a new game clock with default configuration.
    pub fn new() -> Self {
        Self::with_config(ClockConfig::default())
    }

    /// Create a new game clock with custom configuration.
    pub fn with_config(config: ClockConfig) -> Self {
        Self {
            current_tick: 0,
            paused: false,
            speed_multiplier: config.default_speed_multiplier,
            config,
            idle_safety_enabled: true,
            suppress_autopause_in_idle: true,
        }
    }

    /// Advance the clock by one tick.
    /// Returns the new tick count, or None if paused.
    pub fn advance(&mut self) -> Option<u64> {
        if self.paused {
            None
        } else {
            self.current_tick += 1;
            Some(self.current_tick)
        }
    }

    /// Advance the clock by multiple ticks.
    /// Returns the number of ticks actually advanced (0 if paused).
    pub fn advance_by(&mut self, count: u64) -> u64 {
        if self.paused {
            0
        } else {
            self.current_tick += count;
            count
        }
    }

    /// Pause the simulation.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume the simulation.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Toggle pause state.
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Set the speed multiplier.
    /// Clamps to valid range [1, max_speed_multiplier].
    pub fn set_speed(&mut self, multiplier: u32) -> Result<(), String> {
        if multiplier < 1 {
            return Err("Speed multiplier must be at least 1".to_string());
        }
        if multiplier > self.config.max_speed_multiplier {
            return Err(format!(
                "Speed multiplier {} exceeds maximum {}",
                multiplier, self.config.max_speed_multiplier
            ));
        }
        self.speed_multiplier = multiplier;
        Ok(())
    }

    /// Get the effective tick rate in milliseconds (accounting for speed multiplier).
    pub fn effective_tick_rate_ms(&self) -> u64 {
        self.config.tick_rate_ms / self.speed_multiplier as u64
    }

    /// Enable idle safety mode.
    pub fn enable_idle_safety(&mut self) {
        self.idle_safety_enabled = true;
    }

    /// Disable idle safety mode.
    pub fn disable_idle_safety(&mut self) {
        self.idle_safety_enabled = false;
    }

    /// Convert ticks to game hours (default: 1 tick = 1 game hour).
    pub fn ticks_to_hours(&self, ticks: u64) -> u64 {
        ticks
    }

    /// Convert ticks to game days (24 ticks = 1 game day).
    pub fn ticks_to_days(&self, ticks: u64) -> u64 {
        ticks / 24
    }

    /// Get current game time as (days, hours).
    pub fn current_game_time(&self) -> (u64, u64) {
        let days = self.ticks_to_days(self.current_tick);
        let hours = self.current_tick % 24;
        (days, hours)
    }

    /// Format current game time as string.
    pub fn format_game_time(&self) -> String {
        let (days, hours) = self.current_game_time();
        format!("Day {} {:02}:00", days, hours)
    }
}

impl Default for GameClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_clock() {
        let clock = GameClock::new();
        assert_eq!(clock.current_tick, 0);
        assert!(!clock.paused);
        assert_eq!(clock.speed_multiplier, 1);
        assert!(clock.idle_safety_enabled);
    }

    #[test]
    fn test_advance_tick() {
        let mut clock = GameClock::new();
        
        let tick = clock.advance();
        assert_eq!(tick, Some(1));
        assert_eq!(clock.current_tick, 1);

        let tick = clock.advance();
        assert_eq!(tick, Some(2));
        assert_eq!(clock.current_tick, 2);
    }

    #[test]
    fn test_advance_by() {
        let mut clock = GameClock::new();
        
        let advanced = clock.advance_by(10);
        assert_eq!(advanced, 10);
        assert_eq!(clock.current_tick, 10);

        let advanced = clock.advance_by(5);
        assert_eq!(advanced, 5);
        assert_eq!(clock.current_tick, 15);
    }

    #[test]
    fn test_pause_prevents_advance() {
        let mut clock = GameClock::new();
        
        clock.pause();
        assert!(clock.paused);

        let tick = clock.advance();
        assert_eq!(tick, None);
        assert_eq!(clock.current_tick, 0);

        let advanced = clock.advance_by(10);
        assert_eq!(advanced, 0);
        assert_eq!(clock.current_tick, 0);

        clock.resume();
        assert!(!clock.paused);

        let tick = clock.advance();
        assert_eq!(tick, Some(1));
        assert_eq!(clock.current_tick, 1);
    }

    #[test]
    fn test_toggle_pause() {
        let mut clock = GameClock::new();
        assert!(!clock.paused);

        clock.toggle_pause();
        assert!(clock.paused);

        clock.toggle_pause();
        assert!(!clock.paused);
    }

    #[test]
    fn test_set_speed() {
        let mut clock = GameClock::new();
        
        assert!(clock.set_speed(5).is_ok());
        assert_eq!(clock.speed_multiplier, 5);

        assert!(clock.set_speed(10).is_ok());
        assert_eq!(clock.speed_multiplier, 10);

        // Below minimum
        assert!(clock.set_speed(0).is_err());

        // Above maximum
        assert!(clock.set_speed(100).is_err());
    }

    #[test]
    fn test_effective_tick_rate() {
        let mut clock = GameClock::new();
        
        assert_eq!(clock.effective_tick_rate_ms(), 60_000);

        clock.set_speed(2).unwrap();
        assert_eq!(clock.effective_tick_rate_ms(), 30_000);

        clock.set_speed(10).unwrap();
        assert_eq!(clock.effective_tick_rate_ms(), 6_000);
    }

    #[test]
    fn test_game_time_conversion() {
        let mut clock = GameClock::new();
        
        assert_eq!(clock.ticks_to_hours(10), 10);
        assert_eq!(clock.ticks_to_days(24), 1);
        assert_eq!(clock.ticks_to_days(48), 2);
        assert_eq!(clock.ticks_to_days(25), 1);

        clock.advance_by(25);
        let (days, hours) = clock.current_game_time();
        assert_eq!(days, 1);
        assert_eq!(hours, 1);
    }

    #[test]
    fn test_format_game_time() {
        let mut clock = GameClock::new();
        
        assert_eq!(clock.format_game_time(), "Day 0 00:00");

        clock.advance_by(5);
        assert_eq!(clock.format_game_time(), "Day 0 05:00");

        clock.advance_by(19);
        assert_eq!(clock.format_game_time(), "Day 1 00:00");

        clock.advance_by(12);
        assert_eq!(clock.format_game_time(), "Day 1 12:00");
    }

    #[test]
    fn test_idle_safety_toggle() {
        let mut clock = GameClock::new();
        assert!(clock.idle_safety_enabled);

        clock.disable_idle_safety();
        assert!(!clock.idle_safety_enabled);

        clock.enable_idle_safety();
        assert!(clock.idle_safety_enabled);
    }

    #[test]
    fn test_custom_config() {
        let config = ClockConfig {
            tick_rate_ms: 30_000,
            default_speed_multiplier: 2,
            max_speed_multiplier: 50,
        };

        let clock = GameClock::with_config(config.clone());
        assert_eq!(clock.config.tick_rate_ms, 30_000);
        assert_eq!(clock.speed_multiplier, 2);
        assert_eq!(clock.config.max_speed_multiplier, 50);
    }

    #[test]
    fn test_serialization() {
        let clock = GameClock::new();
        
        let json = serde_json::to_string(&clock).unwrap();
        let deserialized: GameClock = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.current_tick, clock.current_tick);
        assert_eq!(deserialized.paused, clock.paused);
        assert_eq!(deserialized.speed_multiplier, clock.speed_multiplier);
    }
}
