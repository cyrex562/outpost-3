//! TimeCommand — commands for controlling simulation time.

use serde::{Deserialize, Serialize};

/// Commands for controlling the game clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeCommand {
    /// Pause the simulation.
    Pause,
    /// Resume the simulation.
    Resume,
    /// Toggle pause state.
    TogglePause,
    /// Set the speed multiplier (1x, 2x, 5x, 10x, etc.).
    SetSpeed(u32),
    /// Enable idle safety mode (auto-pause on critical depletion).
    EnableIdleSafety,
    /// Disable idle safety mode.
    DisableIdleSafety,
}

impl TimeCommand {
    /// Execute this command on a GameClock.
    pub fn execute(&self, clock: &mut crate::simulation::GameClock) -> Result<(), String> {
        match self {
            TimeCommand::Pause => {
                clock.pause();
                Ok(())
            }
            TimeCommand::Resume => {
                clock.resume();
                Ok(())
            }
            TimeCommand::TogglePause => {
                clock.toggle_pause();
                Ok(())
            }
            TimeCommand::SetSpeed(multiplier) => clock.set_speed(*multiplier),
            TimeCommand::EnableIdleSafety => {
                clock.enable_idle_safety();
                Ok(())
            }
            TimeCommand::DisableIdleSafety => {
                clock.disable_idle_safety();
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::GameClock;

    #[test]
    fn test_pause_command() {
        let mut clock = GameClock::new();
        assert!(!clock.paused);

        TimeCommand::Pause.execute(&mut clock).unwrap();
        assert!(clock.paused);
    }

    #[test]
    fn test_resume_command() {
        let mut clock = GameClock::new();
        clock.pause();

        TimeCommand::Resume.execute(&mut clock).unwrap();
        assert!(!clock.paused);
    }

    #[test]
    fn test_toggle_pause_command() {
        let mut clock = GameClock::new();
        assert!(!clock.paused);

        TimeCommand::TogglePause.execute(&mut clock).unwrap();
        assert!(clock.paused);

        TimeCommand::TogglePause.execute(&mut clock).unwrap();
        assert!(!clock.paused);
    }

    #[test]
    fn test_set_speed_command() {
        let mut clock = GameClock::new();

        TimeCommand::SetSpeed(5).execute(&mut clock).unwrap();
        assert_eq!(clock.speed_multiplier, 5);

        TimeCommand::SetSpeed(10).execute(&mut clock).unwrap();
        assert_eq!(clock.speed_multiplier, 10);

        // Invalid speed
        assert!(TimeCommand::SetSpeed(0).execute(&mut clock).is_err());
        assert!(TimeCommand::SetSpeed(100).execute(&mut clock).is_err());
    }

    #[test]
    fn test_idle_safety_commands() {
        let mut clock = GameClock::new();
        assert!(clock.idle_safety_enabled);

        TimeCommand::DisableIdleSafety.execute(&mut clock).unwrap();
        assert!(!clock.idle_safety_enabled);

        TimeCommand::EnableIdleSafety.execute(&mut clock).unwrap();
        assert!(clock.idle_safety_enabled);
    }

    #[test]
    fn test_command_serialization() {
        let cmd = TimeCommand::SetSpeed(5);
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: TimeCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }
}
