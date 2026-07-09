//! The entity-component vocabulary, brought online in Rust (R3).
//!
//! Evaluation only: these modules compute effective tags, resolve modifier
//! queries, and drive the status lifecycle (producing intents). Durable writes
//! stay with the host. Ported from `harsh_realm.{tags,modifiers,status_effects}`.

pub mod modifiers;
pub mod status;
pub mod tags;

pub use modifiers::{
    collect, evaluate_condition, resolve, resolve_final_value, ModifierContribution,
    ResolutionContext, ResolvedModifierResult,
};
pub use status::{expirations, expiry_tick, extended_expiry, is_due, ActiveStatus};
pub use tags::resolve_effective_tags;
