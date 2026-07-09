//! Runtime IR content: the bridge that makes compiled IR records drive the game.
//!
//! The IR vocabulary (`crate::ir`), the compiler (`crate::compiler`), the DSL
//! evaluator (`crate::dsl`), trigger→intent lowering (`crate::dispatch`), and the
//! status-effect/modifier services all exist independently. This module assembles
//! them into a live loop:
//!
//! - [`RuntimeContentStore`] projects compiled [`ComponentRecord`](crate::ir::ComponentRecord)s
//!   into typed, id-keyed lookups and serves status content to the status service.
//! - [`triggers_for_event`](index::triggers_for_event) sources an entity's active
//!   triggers (from its status effects, equipped items' granted traits, and
//!   intrinsic traits) for a given event type.
//! - [`EvalContextBuilder`] materializes an [`EvalContext`](crate::dsl::EvalContext)
//!   from an [`EntitySnapshot`](eval_context::EntitySnapshot) + status/trait/item state.
//! - [`IntentApplier`] applies the [`Intent`](crate::intent::Intent)s produced by
//!   [`dispatch`](crate::dispatch::dispatch) via the existing services.
//! - [`TriggerRuntime`] orchestrates: on an event, gather triggers → build context
//!   → dispatch → apply intents → cascade (bounded), all fail-safe.
//! - [`DemoCombatRunner`] exercises the whole chain in an isolated combat for the
//!   content demo harness.
//!
//! Everything here is fail-safe: parse/eval/apply errors are collected and
//! surfaced, never panicked or silently dropped (AGENTS.md "no silent failures").

pub mod creature_adapter;
pub mod demo_event_runner;
pub mod demo_runner;
pub mod eval_context;
pub mod index;
pub mod intent_applier;
pub mod item_adapter;
pub mod pack_compile;
pub mod store;
pub mod trigger_runtime;

pub use creature_adapter::ir_creature_to_data;
pub use demo_event_runner::{
    DemoEntity, DemoEntityState, DemoEventOutcome, DemoEventRunner, DemoEventStep,
};
pub use demo_runner::{DemoCombatRunner, DemoOutcome, DemoTurn};
pub use eval_context::{EntitySnapshot, EvalContextBuilder};
pub use index::triggers_for_event;
pub use intent_applier::{ApplyOutcome, IntentApplier};
pub use item_adapter::ir_item_to_data;
pub use pack_compile::compile_pack_ir;
pub use store::RuntimeContentStore;
pub use trigger_runtime::{EntityResolver, RuntimeOutcome, TriggerRuntime};
