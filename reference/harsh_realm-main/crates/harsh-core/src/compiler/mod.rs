//! The content grammar → IR compiler (R5-C).
//!
//! Deterministic, no LLM. Compiles the form-2 YAML authoring surface to validated
//! IR records, collecting every diagnostic and failing atomically (grammar spec
//! `docs/design/2026-06-18-content-grammar-compiler-spec.md`).
//!
//! Increment 1: the diagnostics model and the form-2 effect verb registry.
//! Later increments add YAML parsing, record dispatch, inline-record lifting, and
//! whole-document compilation with JSON Schema validation.

pub mod compile;
pub mod diagnostics;
pub mod merge;
pub mod references;
pub mod validate;
pub mod verbs;

pub use compile::{compile, CompileReport, RECORD_TYPES};
pub use diagnostics::{Diagnostic, DiagnosticClass};
pub use merge::{deep_merge, resolve_action_base};
pub use references::{resolve_references, Catalogs};
pub use validate::validate_semantics;
pub use verbs::VerbRegistry;
