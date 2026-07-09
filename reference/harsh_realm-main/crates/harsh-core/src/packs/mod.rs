//! Pack subsystem: manifests, loading, discovery, registry, and content reads.
//!
//! Ported from `src/harsh_realm/packs/`. The `code_loader` (Python importlib)
//! and `migrations` modules are ported separately.

pub mod content_service;
pub mod discovery;
pub mod exceptions;
pub mod loader;
pub mod manifest;
pub mod migrations;
pub mod paths;
pub mod registry;
pub mod version;

pub use content_service::ContentService;
pub use discovery::discover_packs;
pub use loader::{load_manifest, load_pack, Pack, PackRecords};
pub use manifest::{PackDependency, PackManifest};
pub use migrations::{discover_migrations, get_pending_migrations, read_schema_migration, Migration};
pub use registry::{IdConflict, PackRegistry};
