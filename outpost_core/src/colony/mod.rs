//! Colony simulation — pooled commodities, building slots, production chains.
//!
//! A colony is the primary player-managed entity in Outpost 3
//! (see `docs/DESIGN.md §5, §6, §7`). This module owns:
//!
//! - The commodity store (pooled resources per colony) — [`ColonyPool`].
//! - Building slots and their labour assignments.
//! - Production-chain resolution (inputs → outputs per turn).
//!
//! All types here are pure data structures with no I/O; they are driven by
//! [`crate::GameEngine::apply`] through the turn processor.

pub mod building;
pub mod pool;
pub mod production;

pub use building::{ConstructionProject, ConstructionQueue, PlacedBuilding, ProjectId};
pub use pool::{ColonyPool, RecipeOutcome, StockpileDelta};
pub use production::{
    process_production, BuildingProductionResult, PowerGrid, ProductionShortfall,
    ProductionStepOutcome, ShortfallReason,
};

/// Unique identifier for a colony.
pub type ColonyId = uuid::Uuid;

/// Base number of build slots every new colony starts with.
pub const BASE_SLOT_CAPACITY: u32 = 5;

/// A colony: the primary player-managed simulation entity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Colony {
    /// Stable identifier for this colony.
    pub id: ColonyId,
    /// Human-readable colony name.
    pub name: String,
    /// Pooled commodity stockpile for this colony.
    pub pool: ColonyPool,
    /// Completed buildings that are operational.
    pub buildings: Vec<PlacedBuilding>,
    /// In-progress construction queue.
    pub build_queue: ConstructionQueue,
    /// Total build-slot capacity (base + tech bonuses).
    pub slot_capacity: u32,
    /// Optional terrain/biome slug used for hazard probability modifiers.
    ///
    /// Set when the colony is founded at a planetary site; `None` for
    /// colonies created without map context.
    #[serde(default)]
    pub terrain_id: Option<String>,
}

impl Colony {
    /// Create a new colony with the given name, a random ID, and an empty pool.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            pool: ColonyPool::new(),
            buildings: Vec::new(),
            build_queue: ConstructionQueue::new(),
            slot_capacity: BASE_SLOT_CAPACITY,
            terrain_id: None,
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

    /// Return the subset of `all_buildings` constructable in this colony.
    ///
    /// A building is available if it has no `tech_prerequisite`, or its
    /// prerequisite ID is present in `unlocked_buildings`.
    #[must_use]
    pub fn available_buildings<'a>(
        all_buildings: impl Iterator<Item = &'a crate::content::types::BuildingDef>,
        unlocked_buildings: &std::collections::HashSet<String>,
    ) -> Vec<&'a crate::content::types::BuildingDef> {
        crate::tech::unlocked_buildings(all_buildings, unlocked_buildings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colony_new_assigns_unique_ids() {
        let a = Colony::new("Alpha");
        let b = Colony::new("Beta");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn colony_stores_name() {
        let c = Colony::new("Gamma Station");
        assert_eq!(c.name, "Gamma Station");
    }

    #[test]
    fn colony_starts_with_empty_pool() {
        let c = Colony::new("Delta");
        assert_eq!(c.pool.amount("water"), 0.0);
    }

    #[test]
    fn colony_starts_with_base_slot_capacity() {
        let c = Colony::new("Epsilon");
        assert_eq!(c.slot_capacity, BASE_SLOT_CAPACITY);
        assert_eq!(c.slots_used(), 0);
        assert_eq!(c.slots_available(), BASE_SLOT_CAPACITY);
    }

    #[test]
    fn placed_building_consumes_slots() {
        let mut c = Colony::new("Zeta");
        c.buildings.push(PlacedBuilding::new("greenhouse", 2));
        assert_eq!(c.slots_used(), 2);
        assert_eq!(c.slots_available(), BASE_SLOT_CAPACITY - 2);
    }

    #[test]
    fn queued_project_reserves_slots() {
        let mut c = Colony::new("Eta");
        let proj = ConstructionProject::new("mine", 1, 5, vec![], 3);
        c.build_queue.enqueue(proj);
        assert_eq!(c.slots_used(), 1);
    }
}
