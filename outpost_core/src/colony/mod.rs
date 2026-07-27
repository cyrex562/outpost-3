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
pub mod labour;
pub mod pool;
pub mod production;
pub mod resource_pool;
pub mod stores;

pub use building::{
    ConstructionProject, ConstructionQueue, ConstructionTick, PlacedBuilding, ProjectId,
    MAX_BUILDING_NAME_LEN,
};
pub use labour::{allocate_from, allocate_labour, LabourAllocation, LabourCandidate, LabourPlan};
pub use pool::{ColonyPool, RecipeOutcome, StockpileDelta};
pub use production::{
    building_io_summary, line_selection_key, lines_for_building, process_production,
    process_production_scaled, BuildingIoSummary, BuildingProductionResult, LineProductionResult,
    PowerGrid, ProductionInput, ProductionShortfall, ProductionStepOutcome, RecipeLine,
    ShortfallReason,
};
pub use resource_pool::ColonyResourcePool;
pub use stores::ColonyStores;

/// Unique identifier for a colony.
pub type ColonyId = uuid::Uuid;

/// Base number of build slots every new colony starts with.
///
/// Sized to fit the landing kit — one building for each basic resource — with a
/// little room to spare, so a new colony can produce everything it needs and
/// still make a choice or two before it has to buy more capacity with a
/// site-preparation project (issues #306, #317).
///
/// Raised from 5 when the kit became a guaranteed full loadout: 5 could not hold
/// it, and `DeployStarterKit` refused the batch outright. Leaving spare slots
/// rather than starting exactly full is deliberate — site preparation should read
/// as an early decision, not a mandatory first build before anything else is
/// possible.
pub const BASE_SLOT_CAPACITY: u32 = 10;

/// A colony: the primary player-managed simulation entity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Colony {
    /// Stable identifier for this colony.
    pub id: ColonyId,
    /// Human-readable colony name.
    pub name: String,
    /// Pooled **tradeable** commodity stockpile for this colony.
    ///
    /// Colony-local resources (power, housing, research) live in
    /// [`Self::resources`] instead — see issue #304. Trade, haulers, and supply
    /// packages only ever see this pool, which is what makes resources
    /// structurally unshippable.
    pub pool: ColonyPool,
    /// Colony-local resources produced and consumed in place this sol.
    ///
    /// Reset at the end of every colony sol, so it always reports current
    /// throughput rather than an accumulated total. `#[serde(default)]` so
    /// pre-#304 saves load: the field starts empty and is repopulated by the
    /// next production pass.
    #[serde(default)]
    pub resources: ColonyResourcePool,
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
    /// Optional back-pointer to the star-system body this colony sits on.
    ///
    /// Set by [`crate::Command::AssignColonyHomeBody`] so downstream systems
    /// (production modifier, habitability displays, future colonisation
    /// gating) can look up the body's environmental attributes.
    #[serde(default)]
    pub home_body_id: Option<crate::system::BodyId>,
    /// Multiplicative scalar applied to colony production outputs.
    ///
    /// Derived from the home body's habitability rating when
    /// [`crate::Command::AssignColonyHomeBody`] runs. Defaults to `1.0`
    /// (neutral) for colonies founded without a body reference.
    #[serde(default = "default_habitability_modifier")]
    pub habitability_modifier: f32,
    /// Per-category production modifiers, cached from the home body
    /// (issue #184).
    ///
    /// Populated alongside [`Self::habitability_modifier`] whenever
    /// [`crate::Command::AssignColonyHomeBody`] (or the auto-link in
    /// `FoundColonyAtSite`) runs. Stacks *multiplicatively* with
    /// `habitability_modifier` rather than replacing it — see
    /// [`crate::system::Body::modifiers`]. Empty for colonies founded
    /// without a body reference (every category neutral at `1.0`).
    #[serde(default)]
    pub category_modifiers: Vec<crate::system::BodyModifier>,
    /// Each operational building's most recent production outcome, keyed by
    /// **placed-instance id** (issues #182, #307).
    ///
    /// Overwritten every sol by the turn processor's production step;
    /// buildings with no matching recipe (pure storage/habitat) are absent.
    ///
    /// Keyed by instance rather than by `building_type` since #307: per-building
    /// labour means two mines can run at different scales, and a type key forced
    /// them to share one entry — so a starved building hid behind a healthy
    /// sibling, and anything summing this map counted one instance per type.
    ///
    /// The pre-#307 field was `last_production`, keyed by `building_type`. It is
    /// deliberately **not** migrated: a `String`-keyed map cannot deserialize
    /// into a `Uuid`-keyed one, and this is per-sol derived data that the next
    /// advance regenerates in full. Old saves load with this empty.
    #[serde(default)]
    pub last_production_by_building:
        std::collections::HashMap<uuid::Uuid, BuildingProductionResult>,
    /// How the workforce was distributed on the most recent sol (issue #307).
    ///
    /// Stored rather than recomputed so between-sol readouts (employed vs
    /// unemployed labour, #305) report what production *actually did*. Demand is
    /// gated on whether each building could run at all, which a registry-only
    /// estimate cannot know — so recomputing would over-report jobs offered
    /// whenever a building sat idle for want of inputs or power.
    ///
    /// `None` until the colony's first sol has been processed.
    #[serde(default)]
    pub last_labour: Option<labour::LabourPlan>,
    /// Player-selected active recipe per `building_type`, for buildings with
    /// more than one authored recipe (issue #166).
    ///
    /// Applies to every instance of that building type in this colony —
    /// recipe selection is colony-wide per type, not per placed instance.
    /// Absent entries fall back to the first authored recipe for the type
    /// (the pre-#166 deterministic default), so single-recipe buildings need
    /// no entry here at all.
    #[serde(default)]
    pub active_recipes: std::collections::HashMap<String, String>,
    /// Player-set floors that industry may not draw the stockpile below,
    /// keyed by commodity id (issue #308).
    ///
    /// The motivating case is a commodity two chains compete for — keep biomass
    /// for food rather than letting the fuel plant burn it. Reserved stock stays
    /// in [`Self::pool`] and shows in every readout; it is simply not offered to
    /// recipe inputs or building maintenance.
    ///
    /// **Colonist needs are exempt.** Needs resolution is step 2 of the sol and
    /// production is step 3, so colonists eat from the untouched pool *before* a
    /// reserve throttles anything. A reserve therefore cannot starve the
    /// population however large it is — which is the whole point of reserving
    /// food.
    ///
    /// **Construction and trade export respect it too** (issue #355). The build
    /// queue's per-sol instalments are drawn above this floor, and a project
    /// blocked by it reports
    /// [`Event::ConstructionStalledByReserve`](crate::Event::ConstructionStalledByReserve)
    /// rather than a materials shortage — the stock is there, so telling the
    /// player they are short would send them hunting for what they already have.
    /// Export combines this floor with the automatic need reserve by *maximum*;
    /// see `TurnProcessor::compute_trade_reserves`.
    ///
    /// So of the consumers that can draw this stock, only colonist needs ignore
    /// the floor. Everything discretionary honours it.
    ///
    /// **Maintenance is not exempt.** Reserving the commodity your upkeep runs on
    /// can stall your own buildings, reported as
    /// [`ShortfallReason::MaintenanceShort`]. Splitting inputs from maintenance
    /// would need two different "available" figures inside one affordability
    /// ratio, so the reserve applies to the whole production pass.
    ///
    /// [`ConstructionQueue::tick_active_charging`]: ConstructionQueue::tick_active_charging
    /// [`ColonyStores`]: ColonyStores
    /// [`ShortfallReason::MaintenanceShort`]: production::ShortfallReason::MaintenanceShort
    ///
    /// Absent or `0.0` means unreserved. `#[serde(default)]` so pre-#308 saves
    /// load with nothing withheld, matching their behaviour exactly.
    #[serde(default)]
    pub commodity_reserves: std::collections::HashMap<String, f64>,
    /// Whether [`crate::Command::DeployStarterKit`] has already been used on
    /// this colony (issue: playtest feedback round 2 — starter buildings
    /// should land instantly, "like a lander", rather than sit in the
    /// multi-turn `build_queue`).
    ///
    /// One-shot: prevents `DeployStarterKit` from being used as a
    /// repeatable free-and-instant alternative to `QueueConstruction` later
    /// in the game — it's meant for the founding moment only, not a
    /// standing bypass of the normal construction-turn cost.
    #[serde(default)]
    pub starter_kit_deployed: bool,

    /// Whether the buildings on this colony are the engine's default landing
    /// kit rather than a loadout the player chose (issue #317).
    ///
    /// Founding auto-places the kit so that *every* host's founding path yields
    /// a colony that can produce the basics — the browser-mode wizard used to
    /// place nothing at all. But the Tauri wizard does let the player pick a
    /// loadout, and that choice has to win over the default. So while this flag
    /// is set, a single [`crate::Command::DeployStarterKit`] may still supersede
    /// the auto-placed kit, replacing it wholesale.
    ///
    /// Cleared on the first sol advance, which closes the window to the founding
    /// moment: once the colony has actually run, `DeployStarterKit` is barred
    /// again and can't be used as a standing free-construction bypass.
    #[serde(default)]
    pub auto_landing_kit: bool,
}

fn default_habitability_modifier() -> f32 {
    1.0
}

impl Colony {
    /// Create a new colony with the given name, a random ID, and an empty pool.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            pool: ColonyPool::new(),
            resources: ColonyResourcePool::new(),
            buildings: Vec::new(),
            build_queue: ConstructionQueue::new(),
            slot_capacity: BASE_SLOT_CAPACITY,
            terrain_id: None,
            home_body_id: None,
            habitability_modifier: default_habitability_modifier(),
            category_modifiers: Vec::new(),
            last_production_by_building: std::collections::HashMap::new(),
            last_labour: None,
            active_recipes: std::collections::HashMap::new(),
            commodity_reserves: std::collections::HashMap::new(),
            starter_kit_deployed: false,
            auto_landing_kit: false,
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
