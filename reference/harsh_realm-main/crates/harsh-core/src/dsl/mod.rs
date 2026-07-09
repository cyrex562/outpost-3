//! The condition DSL — parser and evaluator, ported from `harsh_realm.dsl`.
//!
//! The AST itself lives in [`crate::ir::condition`] (shared with the schema).
//! This module adds the string parser and a pure evaluator that runs against a
//! caller-supplied data context (the host materializes entity state; the engine
//! does no I/O).

pub mod eval;
pub mod parser;

pub use eval::{evaluate, evaluate_to_bool, EntityView, EvalContext, EvalError};
pub use parser::{parse_condition, ParseError};
