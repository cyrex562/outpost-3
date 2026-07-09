//! Modifier schema, resolution, and transient storage.
//!
//! The core modifier types (Modifier, ModifierCondition, ModifierStacking,
//! ModifierPredicate) live in `crate::ir`.  The pure resolution engine
//! (evaluate_condition, resolve_final_value, collect, resolve) lives in
//! `crate::components::modifiers`.  The async Python service layer
//! (ModifierService, ModifierResolver) is B9 (web host).
//!
//! This module adds the sync `TransientModifierRepository` which manages the
//! `entity_transient_modifiers` table.

pub mod transient;

pub use transient::{TransientModifierRecord, TransientModifierRepository};
