//! ToggleBuildingPower command — power a building on or off.
//!
//! Transitions between `Operational` ↔ `Paused` states.
//! Non-essential buildings can be powered off to reduce power consumption
//! during brownouts.

use crate::domain::{BuildingId, SiteId};
use crate::domain::building_instance::BuildingState;
use crate::events::EventType;
use crate::commands::Command;

/// Error types for ToggleBuildingPower.
#[derive(Debug, Clone, PartialEq)]
pub enum PowerToggleError {
    /// Building could not be found at the given site.
    BuildingNotFound(BuildingId),
    /// Building is under construction and cannot be toggled.
    UnderConstruction(BuildingId),
    /// Building is destroyed and cannot be toggled.
    Destroyed(BuildingId),
    /// Building is already in the desired state.
    AlreadyInState { building_id: BuildingId, powered_on: bool },
}

impl std::fmt::Display for PowerToggleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerToggleError::BuildingNotFound(id) =>
                write!(f, "Building {:?} not found", id),
            PowerToggleError::UnderConstruction(id) =>
                write!(f, "Building {:?} is under construction", id),
            PowerToggleError::Destroyed(id) =>
                write!(f, "Building {:?} is destroyed", id),
            PowerToggleError::AlreadyInState { building_id, powered_on } =>
                write!(f, "Building {:?} is already {}",
                    building_id, if *powered_on { "powered on" } else { "powered off" }),
        }
    }
}

/// Toggle a building's power state (on/off) at a given site.
///
/// Power-off transitions `Operational → Paused`.
/// Power-on transitions `Paused → Operational`.
/// Damaged buildings can also be toggled.
#[derive(Debug, Clone)]
pub struct ToggleBuildingPower {
    pub site_id: SiteId,
    pub building_id: BuildingId,
    /// true = power on (Paused → Operational), false = power off (Operational → Paused)
    pub power_on: bool,
}

impl ToggleBuildingPower {
    pub fn power_off(site_id: SiteId, building_id: BuildingId) -> Self {
        Self { site_id, building_id, power_on: false }
    }

    pub fn power_on(site_id: SiteId, building_id: BuildingId) -> Self {
        Self { site_id, building_id, power_on: true }
    }

    /// Apply the toggle to mutable building state.
    /// Returns the new state or an error.
    pub fn apply(&self, current_state: BuildingState) -> Result<BuildingState, PowerToggleError> {
        match current_state {
            BuildingState::UnderConstruction { .. } => {
                Err(PowerToggleError::UnderConstruction(self.building_id))
            }
            BuildingState::Destroyed => {
                Err(PowerToggleError::Destroyed(self.building_id))
            }
            BuildingState::Operational | BuildingState::Damaged { .. } => {
                if self.power_on {
                    Err(PowerToggleError::AlreadyInState {
                        building_id: self.building_id,
                        powered_on: true,
                    })
                } else {
                    Ok(BuildingState::Paused)
                }
            }
            BuildingState::Paused => {
                if !self.power_on {
                    Err(PowerToggleError::AlreadyInState {
                        building_id: self.building_id,
                        powered_on: false,
                    })
                } else {
                    Ok(BuildingState::Operational)
                }
            }
        }
    }

    /// Validate without site access (just internal logic).
    pub fn validate_local(&self) -> Result<(), PowerToggleError> {
        // No local validation needed — all validation requires the site
        Ok(())
    }
}

impl Command for ToggleBuildingPower {
    type Error = PowerToggleError;

    fn validate(&self) -> Result<(), Self::Error> {
        // Full validation requires access to site state; done in handler or processor
        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        // Generates BuildingPowerToggled event; actual state mutation happens in the handler
        Ok(vec![EventType::BuildingPowerToggled {
            site_id: self.site_id,
            building_id: self.building_id,
            powered_on: self.power_on,
            tick: 0, // Assigned by handler with actual tick
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{SiteId, BuildingId};

    #[test]
    fn test_power_off_from_operational() {
        let cmd = ToggleBuildingPower::power_off(SiteId::new(), BuildingId::new());
        let new_state = cmd.apply(BuildingState::Operational).unwrap();
        assert_eq!(new_state, BuildingState::Paused);
    }

    #[test]
    fn test_power_on_from_paused() {
        let cmd = ToggleBuildingPower::power_on(SiteId::new(), BuildingId::new());
        let new_state = cmd.apply(BuildingState::Paused).unwrap();
        assert_eq!(new_state, BuildingState::Operational);
    }

    #[test]
    fn test_power_off_already_paused_is_error() {
        let cmd = ToggleBuildingPower::power_off(SiteId::new(), BuildingId::new());
        let result = cmd.apply(BuildingState::Paused);
        assert!(matches!(result, Err(PowerToggleError::AlreadyInState { .. })));
    }

    #[test]
    fn test_power_on_already_operational_is_error() {
        let cmd = ToggleBuildingPower::power_on(SiteId::new(), BuildingId::new());
        let result = cmd.apply(BuildingState::Operational);
        assert!(matches!(result, Err(PowerToggleError::AlreadyInState { .. })));
    }

    #[test]
    fn test_cannot_toggle_under_construction() {
        let cmd = ToggleBuildingPower::power_off(SiteId::new(), BuildingId::new());
        let result = cmd.apply(BuildingState::UnderConstruction { progress: 50 });
        assert!(matches!(result, Err(PowerToggleError::UnderConstruction(_))));
    }

    #[test]
    fn test_cannot_toggle_destroyed() {
        let cmd = ToggleBuildingPower::power_on(SiteId::new(), BuildingId::new());
        let result = cmd.apply(BuildingState::Destroyed);
        assert!(matches!(result, Err(PowerToggleError::Destroyed(_))));
    }

    #[test]
    fn test_can_power_off_damaged_building() {
        let cmd = ToggleBuildingPower::power_off(SiteId::new(), BuildingId::new());
        let new_state = cmd.apply(BuildingState::Damaged { severity: 30 }).unwrap();
        assert_eq!(new_state, BuildingState::Paused);
    }
}
