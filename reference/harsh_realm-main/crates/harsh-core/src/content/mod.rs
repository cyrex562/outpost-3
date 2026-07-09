//! Content authoring: YAML loading, IR record store, and legacy migration.
//!
//! `compile.py` and `reference.py` were thin Python shims around PyO3-exported
//! Rust functions (`compile_content`, `ir_schema_json`). Those functions already
//! live in `crates/harsh-core/src/compiler/` and `crates/harsh-core/src/public_api.rs`
//! respectively. The Python wrappers are deleted without replacement here.

pub mod ir_store;
pub mod legacy;
pub mod loader;

pub use ir_store::IRRecordRepository;
pub use legacy::{migrate_records, MigrationResult, MIGRATABLE_CATEGORIES};
pub use loader::{load_documents, load_yaml};
