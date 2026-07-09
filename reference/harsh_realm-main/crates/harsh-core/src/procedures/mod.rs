//! Declarative procedure runtime.
//!
//! Ported from the Python `harsh_realm.procedures` package. The runner and
//! app-state wiring are ported separately once their content-service and
//! pack-registry dependencies land in Rust.

pub mod compute_registry;
pub mod runner;
pub mod schema;

pub use compute_registry::{ComputeCallable, ComputeRegistry};
pub use runner::{ProcedureError, ProcedureRunner};
pub use schema::{
    Procedure, ProcedureInput, ProcedureInputType, ProcedureOutput, ProcedureStep,
    ProcedureStepKind, ProcedureValue,
};
