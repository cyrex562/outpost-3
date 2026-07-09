//! XWN combat engine (split into concern-focused submodules).
//!
//! Ported from the Python `harsh_realm.engine.combat` package. The shared combat
//! data models live in [`crate::combat_runtime`]; the roll math lives in
//! [`crate::resolvers`]. These modules add the engine logic on top.

pub mod action_resolver;
pub mod awareness;
pub mod creation;
pub mod flee;
pub mod narration;
pub mod positioning;
pub mod resolvers;

pub use action_resolver::{resolve_action, ActionOutcome};
pub use awareness::awareness_check;
pub use creation::create_combat;
pub use flee::resolve_flee;
pub use positioning::{assign_positions, positions_notice};
pub use resolvers::{avoidance_defense, resolve_attack, resolve_damage, resolve_shock, AttackParams};
