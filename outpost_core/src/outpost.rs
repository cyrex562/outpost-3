//! Outpost — a lightweight, colony-anchored presence on a system body for
//! single-purpose resource extraction, research, or megaproject support
//! (issue #233).
//!
//! Unlike a [`crate::colony::Colony`], an outpost has no population, no
//! housing/needs resolution, and no stability tracking — it exists purely to
//! run buildings/recipes against a resource pool with a fixed skeleton-crew
//! labor allocation. It reuses the colony production pipeline
//! ([`crate::colony::process_production_scaled`]) and construction queue
//! ([`crate::colony::ConstructionQueue`]) unchanged, since neither is
//! actually coupled to `Colony`/population — `process_production_scaled`
//! takes a raw `labor: f32` and a resource pool, not a `Colony` reference.
//!
//! An outpost always belongs to a parent colony (`parent_colony_id`) and is
//! anchored to a specific system body (`body_id`) — it extends that colony's
//! reach rather than existing independently. Establishing an outpost beyond
//! [`max_outpost_range_au`] of the parent colony's home body is rejected
//! (issue #241); tech/bonus gating of *what can be built* at an outpost is
//! enforced separately at `Command::QueueOutpostConstruction`. Promotion to a
//! full colony is issue #242.

use serde::{Deserialize, Serialize};

use crate::colony::{
    BuildingProductionResult, ColonyId, ColonyPool, ConstructionQueue, PlacedBuilding,
};
use crate::system::{BodyId, BodyModifier};

/// Unique identifier for an outpost.
pub type OutpostId = uuid::Uuid;

/// Base build-slot capacity for a newly established outpost.
///
/// Deliberately smaller than [`crate::colony::BASE_SLOT_CAPACITY`] — an
/// outpost is meant to be single-purpose, not a second colony.
pub const OUTPOST_BASE_SLOT_CAPACITY: u32 = 2;

/// Fixed skeleton-crew labor allocation used by outpost production.
///
/// Outposts have no population to derive labor from (`Population::available_labor`
/// is a colony-only concept) — this is a flat stand-in representing a small
/// permanent or automated crew. A future automation tech could scale this
/// per issue #241's "bonuses" framing; fixed for this first pass.
pub const OUTPOST_BASE_LABOR: f32 = 2.0;

/// Base max range (AU) an outpost may be established from its parent
/// colony's home body, per point of `propulsion_level` (issue #241).
///
/// A colony with no `home_body_id` (never placed on the planet map — e.g.
/// one founded via the bare `Command::FoundColony` used throughout the test
/// suite) has no meaningful notion of "distance from home", so the range
/// check is skipped entirely for it rather than defaulting to zero — see
/// [`max_outpost_range_au`].
pub const BASE_OUTPOST_RANGE_AU: f32 = 3.0;

/// Compute the max distance (AU) an outpost may be established from its
/// parent colony's home body, given the system's current propulsion level
/// and any tech-driven range bonus (`TechEffect::ExtendOutpostRange`).
///
/// `max_range = BASE_OUTPOST_RANGE_AU * propulsion_level + range_bonus_au`.
#[must_use]
pub fn max_outpost_range_au(propulsion_level: u32, range_bonus_au: f32) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let level = propulsion_level.max(1) as f32;
    BASE_OUTPOST_RANGE_AU * level + range_bonus_au.max(0.0)
}

/// A lightweight, single-purpose off-world presence anchored to a system
/// body, established and owned by a parent colony.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outpost {
    /// Stable identifier.
    pub id: OutpostId,
    /// Human-readable name.
    pub name: String,
    /// The colony that established (and owns) this outpost.
    pub parent_colony_id: ColonyId,
    /// The system body this outpost is anchored to.
    pub body_id: BodyId,
    /// Pooled **tradeable** commodity stockpile.
    pub pool: ColonyPool,
    /// Colony-local resources produced and consumed in place this sol
    /// (issue #304). Reset every sol, like a colony's — an outpost runs the
    /// same production pipeline, so it needs the same resource store for its
    /// buildings' `power`/`research` outputs to land in.
    #[serde(default)]
    pub resources: crate::colony::ColonyResourcePool,
    /// Completed buildings that are operational.
    pub buildings: Vec<PlacedBuilding>,
    /// In-progress construction queue (reuses the colony queue type
    /// unchanged — see module doc comment).
    pub build_queue: ConstructionQueue,
    /// Total build-slot capacity.
    pub slot_capacity: u32,
    /// Per-category production modifiers, cached from `body_id` at
    /// establishment time (mirrors [`crate::colony::Colony::category_modifiers`]).
    pub category_modifiers: Vec<BodyModifier>,
    /// Player-selected active recipe per `building_type` (mirrors
    /// [`crate::colony::Colony::active_recipes`]).
    pub active_recipes: std::collections::HashMap<String, String>,
    /// Each operational building's most recent production outcome.
    pub last_production: std::collections::HashMap<String, BuildingProductionResult>,
}

impl Outpost {
    /// Construct a new, empty outpost.
    #[must_use]
    pub fn new(name: impl Into<String>, parent_colony_id: ColonyId, body_id: BodyId) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            parent_colony_id,
            body_id,
            pool: ColonyPool::new(),
            resources: crate::colony::ColonyResourcePool::new(),
            buildings: Vec::new(),
            build_queue: ConstructionQueue::new(),
            slot_capacity: OUTPOST_BASE_SLOT_CAPACITY,
            category_modifiers: Vec::new(),
            active_recipes: std::collections::HashMap::new(),
            last_production: std::collections::HashMap::new(),
        }
    }

    /// Return the number of build slots currently in use (completed + queued).
    #[must_use]
    pub fn slots_used(&self) -> u32 {
        let from_buildings: u32 = self.buildings.iter().map(|b| b.slot_cost).sum();
        let from_queue = self.build_queue.slots_reserved();
        from_buildings + from_queue
    }

    /// Return the number of build slots still available.
    #[must_use]
    pub fn slots_available(&self) -> u32 {
        self.slot_capacity.saturating_sub(self.slots_used())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outpost_new_assigns_unique_ids_and_links_parent_and_body() {
        let parent = uuid::Uuid::new_v4();
        let body = BodyId::new();
        let o = Outpost::new("Mining Camp Alpha", parent, body.clone());
        assert_eq!(o.parent_colony_id, parent);
        assert_eq!(o.body_id, body);
        assert_eq!(o.slot_capacity, OUTPOST_BASE_SLOT_CAPACITY);
        assert_eq!(o.slots_used(), 0);
        assert_eq!(o.slots_available(), OUTPOST_BASE_SLOT_CAPACITY);
    }

    #[test]
    fn outpost_starts_with_empty_pool_and_no_buildings() {
        let o = Outpost::new("Camp", uuid::Uuid::new_v4(), BodyId::new());
        assert_eq!(o.pool.amount("structural_ore"), 0.0);
        assert!(o.buildings.is_empty());
    }

    #[test]
    fn placed_building_consumes_outpost_slots() {
        let mut o = Outpost::new("Zeta", uuid::Uuid::new_v4(), BodyId::new());
        o.buildings.push(PlacedBuilding::new("mining_outpost", 2));
        assert_eq!(o.slots_used(), 2);
        assert_eq!(
            o.slots_available(),
            OUTPOST_BASE_SLOT_CAPACITY.saturating_sub(2)
        );
    }
}
