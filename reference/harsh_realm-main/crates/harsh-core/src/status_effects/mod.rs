//! Status effect content and runtime subsystem.
//!
//! Ported from the Python `harsh_realm.status_effects` package.

pub mod handlers;
pub mod models;
pub mod repository;
pub mod schema;
pub mod service;

pub use handlers::register_status_effect_handlers;
pub use models::ActiveStatusEffect;
pub use repository::StatusEffectRepository;
pub use schema::{Stacking, StatusEffect};
pub use service::{ContentLookup, StatusEffectService, WorldClock};
