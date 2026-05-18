pub mod banking_commands;
pub mod colony_commands;
pub mod trading_commands;
pub mod construction_commands;
pub mod labor_commands;
pub mod toggle_building_power;

pub use banking_commands::*;
pub use colony_commands::*;
pub use trading_commands::*;
pub use construction_commands::*;
pub use labor_commands::{AssignLabor, DeallocateLabor, LaborError};
pub use toggle_building_power::{ToggleBuildingPower, PowerToggleError};


use crate::events::EventType;

pub trait Command {
    type Error;

    fn validate(&self) -> Result<(), Self::Error>;
    fn execute(&self) -> Result<Vec<EventType>, Self::Error>;
}
