//! YAML scenario configuration format for multi-colony network balance simulations.

use serde::{Deserialize, Serialize};

/// Top-level scenario configuration loaded from a YAML file.
///
/// Example:
/// ```yaml
/// colonies:
///   - name: Alpha Base
///     starting_pop: 500
///     buildings: [water_extractor, hydroponics]
/// trade_routes:
///   - from: Alpha Base
///     to: Beta Station
///     commodity: water
///     max_per_month: 100
/// victory: population_milestone
/// victory_target: 10000
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioConfig {
    /// List of colonies to found at the start of the simulation.
    pub colonies: Vec<ColonySpec>,
    /// Trade routes to wire up between colonies after founding.
    #[serde(default)]
    pub trade_routes: Vec<TradeRouteSpec>,
    /// Victory condition kind (informational; used for reporting).
    #[serde(default)]
    pub victory: Option<String>,
    /// Numeric target for the victory condition (e.g. total population).
    #[serde(default)]
    pub victory_target: Option<f64>,
}

/// Specification for a single colony to found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColonySpec {
    /// Display name for the colony.
    pub name: String,
    /// Starting colonist head-count.
    pub starting_pop: u64,
    /// Building type IDs to pre-construct in the colony.
    #[serde(default)]
    pub buildings: Vec<String>,
}

/// Specification for a trade route between two colonies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRouteSpec {
    /// Source colony name.
    pub from: String,
    /// Destination colony name.
    pub to: String,
    /// Commodity identifier (reserved for future directed-flow; currently informational).
    #[serde(default)]
    pub commodity: Option<String>,
    /// Maximum units per strategic month that may transit this route.
    pub max_per_month: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_round_trips_yaml() {
        let yaml = r"
colonies:
  - name: Alpha Base
    starting_pop: 500
    buildings: [water_extractor, hydroponics]
  - name: Beta Station
    starting_pop: 200
trade_routes:
  - from: Alpha Base
    to: Beta Station
    commodity: water
    max_per_month: 100
victory: population_milestone
victory_target: 10000
";
        let cfg: ScenarioConfig = serde_yaml::from_str(yaml).expect("parse");
        assert_eq!(cfg.colonies.len(), 2);
        assert_eq!(cfg.colonies[0].name, "Alpha Base");
        assert_eq!(cfg.colonies[0].starting_pop, 500);
        assert_eq!(
            cfg.colonies[0].buildings,
            ["water_extractor", "hydroponics"]
        );
        assert_eq!(cfg.trade_routes.len(), 1);
        assert_eq!(cfg.trade_routes[0].from, "Alpha Base");
        assert_eq!(cfg.trade_routes[0].max_per_month, 100.0);
        assert_eq!(cfg.victory.as_deref(), Some("population_milestone"));
        assert_eq!(cfg.victory_target, Some(10000.0));
    }

    #[test]
    fn scenario_defaults_work_for_minimal_config() {
        let yaml = "colonies:\n  - name: Solo\n    starting_pop: 100\n";
        let cfg: ScenarioConfig = serde_yaml::from_str(yaml).expect("parse");
        assert_eq!(cfg.colonies.len(), 1);
        assert!(cfg.trade_routes.is_empty());
        assert!(cfg.victory.is_none());
        assert!(cfg.victory_target.is_none());
    }
}
