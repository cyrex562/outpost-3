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
    /// Labour units available this turn.
    pub labour_available: f32,
    /// Total labour derived from population.
    pub labour_total: f32,
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
    pub labour_assigned: u32,
    /// Number of build slots consumed by this building.
    pub slot_cost: u32,
    /// Whether the building ran at full capacity last turn.
    pub full_capacity: bool,
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
        self.digest_items.iter().filter(|i| filter.matches(i)).collect()
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
        Interrupt::new(tier, InterruptSource::ConstructionComplete, colony_id, message)
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
            buildings: vec![BuildingRow {
                building_type: "greenhouse".to_string(),
                labour_assigned: 10,
                slot_cost: 1,
                full_capacity: true,
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
