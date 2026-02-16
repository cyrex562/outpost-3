pub mod banking_commands;
pub mod colony_commands;
pub mod trading_commands;

pub use banking_commands::*;
pub use colony_commands::*;
pub use trading_commands::*;

use crate::events::EventType;

pub trait Command {
    type Error;

    fn validate(&self) -> Result<(), Self::Error>;
    fn execute(&self) -> Result<Vec<EventType>, Self::Error>;
}
