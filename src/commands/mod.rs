pub mod colony_commands;

pub use colony_commands::*;

use crate::events::EventType;

pub trait Command {
    type Error;

    fn validate(&self) -> Result<(), Self::Error>;
    fn execute(&self) -> Result<Vec<EventType>, Self::Error>;
}
