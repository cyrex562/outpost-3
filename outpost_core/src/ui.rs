//! UI data-transfer types for the Phase 6 frontend screens.
//!
//! This module exposes **read-only view models** consumed by the Vue frontend
//! via the Axum web host.  All types are plain data structs with `serde`
//! derives — no business logic lives here.
//!
//! Screen breakdown (see `docs/DESIGN.md §5, §8.1, §12A`):
//! - [`ColonyScreenData`]   — colony management screen (buildings, stockpile, population).
//! - [`PlanetMapData`]      — planet hex map (hexes, colony nodes, infrastructure).
//! - [`InterruptDigestData`]— return-from-fast-forward triage panel.
//! - [`TimeControlState`]   — current time-control settings (threshold, max turns).

use serde::{Deserialize, Serialize};

use crate::colony::ColonyId;
use crate::interrupt::{Interrupt, Tier};

// ─── Colony management screen ─────────────────────────────────────────────────

/// Complete data bundle for the colony management screen (§5).
///
/// Aggregates all panels shown on the colony screen: buildings, stockpile,
/// population, labour, construction queue, and directives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColonyScreenData {
    /// Stable colony identifier.
    pub colony_id: ColonyId,
    /// Human-readable colony name.
    pub name: String,
    /// Current colonist head-count (fractional for growth modelling).
    pub population: f32,
    /// Stability scalar in `[0.0, 1.0]`.
    pub stability: f32,
    /// Build slots currently in use.
    pub slots_used: u32,
    /// Total build slot capacity.
    pub slot_capacity: u32,
    /// Labour units able to work this turn (population scaled by stability).
    pub labour_available: f32,
    /// Total workforce the population could field at full stability — the
    /// ceiling [`Self::labour_available`] is reduced from by unrest.
    pub labour_total: f32,
    /// Worker slots the colony's operational buildings are asking for — the
    /// number of jobs on offer (issue #305).
    pub labour_demanded: f32,
    /// Workforce actually taken up by those jobs:
    /// `min(labour_demanded, labour_available)`.
    pub labour_employed: f32,
    /// Workforce with no job to go to: `labour_available - labour_employed`.
    ///
    /// Note this is "no post to fill", not a social-unemployment model — there
    /// is no gameplay consequence attached yet (see issue #305's open
    /// questions).
    pub labour_unemployed: f32,
    /// All operational buildings with their current labour assignments.
    pub buildings: Vec<BuildingRow>,
    /// Stockpile rows — one per commodity that has ever had a non-zero amount.
    pub stockpile: Vec<StockpileRow>,
    /// Construction projects currently in the queue.
    pub construction_queue: Vec<ConstructionQueueRow>,
    /// Whether manual override is active (suppresses directive automation).
    pub manual_override: bool,
}

/// A single building row for the colony management screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingRow {
    /// Content-pack key for the building type.
    pub building_type: String,
    /// Labour units currently assigned to this building.
    ///
    /// **Always `0` today**: per-building labour assignment has no backing
    /// state — `PlacedBuilding` stores no labour, and `Command::AssignLabour`
    /// validates and emits an event without persisting anything. Production is
    /// gated by a *colony-wide* labour ratio instead
    /// (`colony::production`), so this must not be used to infer whether a
    /// building is working — read [`Self::scale`] for that. Real per-building
    /// assignment arrives with automatic labour assignment (issue #307).
    pub labour_assigned: u32,
    /// Number of build slots consumed by this building.
    pub slot_cost: u32,
    /// Whether the building ran at full capacity last turn.
    pub full_capacity: bool,
    /// Scale the building actually produced at last turn, in `[0.0, 1.0]`.
    ///
    /// `0.0` means it genuinely produced nothing; `1.0` means full output.
    /// Sourced from `Colony::last_production`, so this is the authoritative
    /// "is it working?" signal (issue #303 — the colony screen used to infer
    /// status from the always-zero [`Self::labour_assigned`] and therefore
    /// reported *every* building as idle).
    pub scale: f64,
    /// Why the building fell short of full output last turn, if it did — a
    /// short human-readable reason (e.g. `"input short: water"`).
    pub shortfall_reason: Option<String>,
    /// Whether this building only has always-on ([`concurrent`]) recipes and
    /// therefore has no recipe for the player to choose.
    ///
    /// Lets the UI say so explicitly rather than showing an empty picker —
    /// `colony_hq` is the motivating case (issue #303).
    ///
    /// [`concurrent`]: crate::content::types::RecipeDef::concurrent
    pub always_on: bool,
}

/// A single commodity row in the stockpile table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockpileRow {
    /// Commodity identifier (content-pack key).
    pub commodity_id: String,
    /// Current amount in the pool.
    pub amount: f64,
    /// Maximum storable amount (`None` = unlimited).
    pub capacity: Option<f64>,
    /// Net change last turn (positive = production surplus, negative = deficit).
    pub net_per_turn: f64,
}

/// A single in-progress construction project row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructionQueueRow {
    /// Stable project identifier (for cancellation commands).
    pub project_id: uuid::Uuid,
    /// Content-pack key of the building being built.
    pub building_type: String,
    /// Turns completed so far.
    pub turns_completed: u32,
    /// Total turns required.
    pub turns_total: u32,
    /// Build slots reserved during construction.
    pub slot_cost: u32,
}

// ─── Building details HUD ──────────────────────────────────────────────────────

/// Full detail bundle for one building type within a colony (issue #182).
///
/// Combines authored content-pack data (category, recipe flows, maintenance)
/// with the building's most recent production outcome, if it has run at
/// least once. `None` for both `recipe`/`last_run` means the building has no
/// matching recipe (e.g. pure storage/habitat structures).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingDetailData {
    /// Content-pack key for the building type.
    pub building_type: String,
    /// Human-readable name.
    pub name: String,
    /// Short description from the content pack.
    pub description: String,
    /// Logical category (`Production`, `Storage`, `Housing`, `Power`, `Research`, `Other`).
    pub category: String,
    /// Number of build slots this building occupies.
    pub slot_cost: u32,
    /// Power delta per sol (negative = produced).
    pub power_delta: f64,
    /// Per-sol maintenance upkeep, if any.
    pub maintenance: Vec<IngredientRow>,
    /// The recipe this building actually runs right now — `active_recipes`'
    /// selection if set, else the deterministic default (issue #166).
    pub recipe: Option<RecipeRow>,
    /// Every recipe authored for this building type, in the order a player
    /// could select between them (issue #166). Empty for buildings with at
    /// most one recipe — `recipe` alone covers that case; a selector only
    /// makes sense once there's a real choice.
    pub available_recipes: Vec<RecipeRow>,
    /// Every [`crate::content::types::RecipeDef::concurrent`] recipe authored
    /// for this building type — these always run alongside `recipe` (if any),
    /// every turn, with no player selection needed. A building with only
    /// concurrent recipes (e.g. `colony_hq`) has `recipe: None` here but a
    /// non-empty `concurrent_recipes`.
    pub concurrent_recipes: Vec<RecipeRow>,
    /// Outcome of the building's most recent production attempt, if it has
    /// run at least once since the colony was founded.
    pub last_run: Option<BuildingRunRow>,
}

/// A commodity id + quantity pair (construction cost, maintenance draw, or
/// recipe flow line).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngredientRow {
    /// Commodity identifier.
    pub commodity_id: String,
    /// Quantity consumed or produced per cycle.
    pub quantity: f64,
}

/// A production recipe's input/output flows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeRow {
    /// Recipe identifier.
    pub recipe_id: String,
    /// Human-readable recipe name.
    pub name: String,
    /// Commodities consumed per cycle.
    pub inputs: Vec<IngredientRow>,
    /// Commodities produced per cycle.
    pub outputs: Vec<IngredientRow>,
    /// Duration in colony-sols per production cycle.
    pub cycle_sols: u32,
}

/// The outcome of a building's most recent production attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingRunRow {
    /// Scale factor applied to inputs/outputs last turn, in `[0.0, 1.0]`.
    pub scale: f64,
    /// `true` if the building ran at full capacity with no shortfalls.
    pub is_full_production: bool,
    /// Shortfalls that reduced the scale below 1.0, if any.
    pub shortfalls: Vec<ShortfallRow>,
}

/// A single shortfall reason + severity, shaped for direct UI display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortfallRow {
    /// Shortfall category: `input_short`, `power_brownout`, `labor_short`, or `maintenance_short`.
    pub kind: String,
    /// The commodity id that was the tightest constraint, if applicable.
    pub commodity_id: Option<String>,
    /// The scale factor that was actually applied (`< 1.0` when short).
    pub effective_scale: f64,
}

// ─── Planet hex map ───────────────────────────────────────────────────────────

/// Complete data bundle for the planet hex map view (§8.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanetMapData {
    /// Planet display name.
    pub planet_name: String,
    /// All hex cells on the planet map.
    pub hexes: Vec<HexCell>,
    /// Colony nodes placed on the map (subset of hexes that have colonies).
    pub colony_nodes: Vec<ColonyNode>,
    /// Infrastructure edges connecting colony nodes.
    pub infrastructure: Vec<InfraEdge>,
}

/// A single hex cell on the planet map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexCell {
    /// Axial column coordinate.
    pub q: i32,
    /// Axial row coordinate.
    pub r: i32,
    /// Biome or terrain category (e.g. `"tundra"`, `"desert"`, `"ocean"`).
    pub biome: String,
    /// Resource deposit indicators visible on this hex.
    pub deposits: Vec<String>,
}

/// A colony node placed on the hex map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColonyNode {
    /// Colony stable identifier.
    pub colony_id: ColonyId,
    /// Display name.
    pub name: String,
    /// Axial column of the hex this colony occupies.
    pub q: i32,
    /// Axial row of the hex this colony occupies.
    pub r: i32,
    /// Population count displayed as node label.
    pub population: f32,
}

/// An infrastructure edge between two colony nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfraEdge {
    /// Source colony node.
    pub from_colony_id: ColonyId,
    /// Destination colony node.
    pub to_colony_id: ColonyId,
    /// Infrastructure category (e.g. `"road"`, `"pipeline"`, `"power_line"`).
    pub kind: String,
    /// Normalised throughput in `[0.0, 1.0]` (0 = blocked, 1 = full capacity).
    pub throughput: f32,
}

// ─── Interrupt digest ─────────────────────────────────────────────────────────

/// The return-from-fast-forward triage panel (§12A).
///
/// Shown when `advance_until_interrupted` halts or completes; summarises
/// what happened and what needs the player's attention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptDigestData {
    /// Sol turn at which the fast-forward stopped.
    pub stopped_at_turn: u64,
    /// Number of turns actually advanced in this run.
    pub turns_advanced: u32,
    /// The halt-level interrupt that stopped fast-forward, if any.
    pub halting_interrupt: Option<Interrupt>,
    /// Accumulated `Notable` and `Ambient` items collected during the run.
    pub digest_items: Vec<DigestItem>,
    /// Active filter applied to `digest_items` (empty = show all).
    pub active_filter: DigestFilter,
}

/// A single entry in the interrupt digest panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestItem {
    /// Underlying interrupt.
    pub interrupt: Interrupt,
    /// Whether this item has been acknowledged (dismissed) by the player.
    pub acknowledged: bool,
}

/// Filter state for the interrupt digest panel.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DigestFilter {
    /// Only show items at or above this tier (`None` = show all tiers).
    pub min_tier: Option<Tier>,
    /// Only show items belonging to this colony (`None` = all colonies).
    pub colony_id: Option<ColonyId>,
    /// Free-text substring filter applied to interrupt messages.
    pub search_text: String,
}

impl DigestFilter {
    /// Create a new empty filter (shows everything).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return `true` if `item` passes this filter.
    #[must_use]
    pub fn matches(&self, item: &DigestItem) -> bool {
        if let Some(min_tier) = self.min_tier {
            if item.interrupt.tier < min_tier {
                return false;
            }
        }
        if let Some(cid) = self.colony_id {
            if item.interrupt.colony_id != Some(cid) {
                return false;
            }
        }
        if !self.search_text.is_empty() {
            let needle = self.search_text.to_lowercase();
            if !item.interrupt.message.to_lowercase().contains(&needle) {
                return false;
            }
        }
        true
    }
}

impl InterruptDigestData {
    /// Apply `filter` and return only matching digest items.
    #[must_use]
    pub fn filtered_items(&self, filter: &DigestFilter) -> Vec<&DigestItem> {
        self.digest_items
            .iter()
            .filter(|i| filter.matches(i))
            .collect()
    }
}

// ─── Time-control state ───────────────────────────────────────────────────────

/// Current time-control settings surfaced to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeControlState {
    /// Current colony-sol counter.
    pub current_sol: u64,
    /// Current strategic-month counter.
    pub current_month: u64,
    /// The interrupt tier threshold: fast-forward halts at this tier or above.
    pub threshold: Tier,
    /// Maximum turns the next fast-forward advance will run.
    pub max_advance_turns: u32,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interrupt::{Interrupt, InterruptSource, Tier};

    fn make_interrupt(tier: Tier, message: &str, colony_id: Option<ColonyId>) -> Interrupt {
        Interrupt::new(
            tier,
            InterruptSource::ConstructionComplete,
            colony_id,
            message,
        )
    }

    fn make_item(tier: Tier, msg: &str, colony_id: Option<ColonyId>) -> DigestItem {
        DigestItem {
            interrupt: make_interrupt(tier, msg, colony_id),
            acknowledged: false,
        }
    }

    #[test]
    fn digest_filter_default_matches_all() {
        let filter = DigestFilter::new();
        let item = make_item(Tier::Ambient, "hello", None);
        assert!(filter.matches(&item));
    }

    #[test]
    fn digest_filter_min_tier_excludes_lower() {
        let filter = DigestFilter {
            min_tier: Some(Tier::Urgent),
            colony_id: None,
            search_text: String::new(),
        };
        let notable = make_item(Tier::Notable, "info", None);
        let urgent = make_item(Tier::Urgent, "urgent", None);
        assert!(!filter.matches(&notable));
        assert!(filter.matches(&urgent));
    }

    #[test]
    fn digest_filter_colony_excludes_other_colony() {
        let target = uuid::Uuid::new_v4();
        let other = uuid::Uuid::new_v4();
        let filter = DigestFilter {
            min_tier: None,
            colony_id: Some(target),
            search_text: String::new(),
        };
        assert!(filter.matches(&make_item(Tier::Notable, "msg", Some(target))));
        assert!(!filter.matches(&make_item(Tier::Notable, "msg", Some(other))));
        assert!(!filter.matches(&make_item(Tier::Notable, "msg", None)));
    }

    #[test]
    fn digest_filter_search_text_case_insensitive() {
        let filter = DigestFilter {
            min_tier: None,
            colony_id: None,
            search_text: "WATER".to_string(),
        };
        assert!(filter.matches(&make_item(Tier::Notable, "water shortage", None)));
        assert!(!filter.matches(&make_item(Tier::Notable, "power failure", None)));
    }

    #[test]
    fn digest_filtered_items_applies_filter() {
        let colony_a = uuid::Uuid::new_v4();
        let digest = InterruptDigestData {
            stopped_at_turn: 10,
            turns_advanced: 5,
            halting_interrupt: None,
            digest_items: vec![
                make_item(Tier::Notable, "water", Some(colony_a)),
                make_item(Tier::Ambient, "noise", None),
            ],
            active_filter: DigestFilter::new(),
        };
        let filter = DigestFilter {
            min_tier: Some(Tier::Notable),
            colony_id: None,
            search_text: String::new(),
        };
        let items = digest.filtered_items(&filter);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].interrupt.message, "water");
    }

    #[test]
    fn colony_screen_data_serde_round_trip() {
        let data = ColonyScreenData {
            colony_id: uuid::Uuid::new_v4(),
            name: "Alpha Base".to_string(),
            population: 100.0,
            stability: 0.8,
            slots_used: 2,
            slot_capacity: 5,
            labour_available: 40.0,
            labour_total: 50.0,
            labour_demanded: 12.0,
            labour_employed: 12.0,
            labour_unemployed: 28.0,
            buildings: vec![BuildingRow {
                building_type: "greenhouse".to_string(),
                labour_assigned: 10,
                slot_cost: 1,
                full_capacity: true,
                scale: 1.0,
                shortfall_reason: None,
                always_on: false,
            }],
            stockpile: vec![StockpileRow {
                commodity_id: "food".to_string(),
                amount: 200.0,
                capacity: Some(500.0),
                net_per_turn: 10.0,
            }],
            construction_queue: vec![],
            manual_override: false,
        };
        let json = serde_json::to_string(&data).unwrap();
        let back: ColonyScreenData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Alpha Base");
        assert_eq!(back.buildings.len(), 1);
        assert_eq!(back.stockpile[0].commodity_id, "food");
    }

    #[test]
    fn planet_map_data_serde_round_trip() {
        let map = PlanetMapData {
            planet_name: "Kepler-b".to_string(),
            hexes: vec![HexCell {
                q: 0,
                r: 0,
                biome: "tundra".to_string(),
                deposits: vec!["iron".to_string()],
            }],
            colony_nodes: vec![],
            infrastructure: vec![],
        };
        let json = serde_json::to_string(&map).unwrap();
        let back: PlanetMapData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.planet_name, "Kepler-b");
        assert_eq!(back.hexes[0].biome, "tundra");
    }

    #[test]
    fn time_control_state_serde_round_trip() {
        let state = TimeControlState {
            current_sol: 42,
            current_month: 3,
            threshold: Tier::Notable,
            max_advance_turns: 10,
        };
        let json = serde_json::to_string(&state).unwrap();
        let back: TimeControlState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.current_sol, 42);
        assert!(matches!(back.threshold, Tier::Notable));
    }

    #[test]
    fn infra_edge_stores_kind_and_throughput() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        let edge = InfraEdge {
            from_colony_id: a,
            to_colony_id: b,
            kind: "road".to_string(),
            throughput: 0.75,
        };
        assert_eq!(edge.kind, "road");
        assert!((edge.throughput - 0.75).abs() < f32::EPSILON);
    }
}
