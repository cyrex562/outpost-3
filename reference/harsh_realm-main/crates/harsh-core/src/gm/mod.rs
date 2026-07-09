//! GM controller and scene state machines.
//!
//! Ported incrementally from the Python `harsh_realm.gm` package.

pub mod controller;
pub mod event_handlers;
pub mod narrator;
pub mod scenes;

pub use controller::GMController;
pub use event_handlers::register_gm_event_handlers;
pub use narrator::Narrator;
