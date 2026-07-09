//! Content-pack loading for authored game data.
//!
//! All authored records (buildings, commodities, recipes, tech nodes, events)
//! are defined in YAML/JSON content packs under `content/` and loaded at
//! runtime — never hardcoded in the kernel (see `docs/DESIGN.md §14`).
//!
//! This module will own:
//! - The pack manifest format.
//! - Pack discovery and loading logic.
//! - The registry that makes loaded content available to the turn processor.
//!
//! No I/O is performed from within this module itself; callers inject the
//! raw YAML bytes so that `outpost_core` stays free of `std::fs` dependencies.

/// Identifies a content pack by its string slug (e.g. `"base-colony"`).
pub type PackId = String;

/// Stub content registry.
///
/// Will be expanded with typed registries for buildings, commodities, etc.
/// once content-loading mechanics are implemented (issue #9+).
#[derive(Debug, Default)]
pub struct ContentRegistry {
    /// Slugs of all packs that have been loaded into this registry.
    loaded_packs: Vec<PackId>,
}

impl ContentRegistry {
    /// Create an empty content registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register that a pack with the given ID has been loaded.
    ///
    /// In the full implementation this will parse and index pack manifests.
    pub fn register_pack(&mut self, id: impl Into<PackId>) {
        self.loaded_packs.push(id.into());
    }

    /// Return the slugs of all currently registered packs.
    #[must_use]
    pub fn loaded_packs(&self) -> &[PackId] {
        &self.loaded_packs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_starts_empty() {
        let r = ContentRegistry::new();
        assert!(r.loaded_packs().is_empty());
    }

    #[test]
    fn register_pack_records_id() {
        let mut r = ContentRegistry::new();
        r.register_pack("base-colony");
        assert_eq!(r.loaded_packs(), &["base-colony"]);
    }
}
