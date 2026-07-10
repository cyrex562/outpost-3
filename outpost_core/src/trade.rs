//! Inter-colony trade — auto base-flow + manual priority override.
//!
//! Implements Phase 5 of the build sequence (DESIGN.md §8.1).
//!
//! # Design summary
//!
//! Goods flow automatically once a [`TradeRoute`] exists between two colonies.
//! Each strategic turn [`run_trade_flow`] is called with the full
//! [`TradeNetwork`] and all colony commodity pools.  It computes each colony's
//! *surplus* (pool amount above the configured target) for every commodity and
//! ships it toward colonies in *deficit* (below their target) over every
//! connected route, subject to the route's `throughput_cap` per commodity per
//! turn.
//!
//! A [`TradeOverride`] attached to a colony can suppress auto-flow for a
//! specific commodity (by setting `suppress_auto = true`) or clamp the flow
//! with an explicit `cap` — enabling the player to prioritise or block imports
//! and exports without breaking the general automation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::colony::ColonyId;

// ─── Site ─────────────────────────────────────────────────────────────────────

/// An opaque identifier for a surveyed planetary site (hex or named location).
///
/// Carried by [`FoundColonyAtSite`](crate::Command::FoundColonyAtSite) to record
/// where a colony was planted even when no hex map is loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SiteId(pub uuid::Uuid);

impl SiteId {
    /// Generate a new random [`SiteId`].
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for SiteId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SiteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ─── Trade route ─────────────────────────────────────────────────────────────

/// A directed or bidirectional infrastructure link between two colonies.
///
/// Throughput is symmetric: up to `throughput_cap` units of any single
/// commodity can travel in either direction per strategic turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRoute {
    /// Stable identifier for this route (for removal / UI).
    pub id: uuid::Uuid,
    /// One endpoint of the route.
    pub colony_a: ColonyId,
    /// Other endpoint of the route.
    pub colony_b: ColonyId,
    /// Maximum commodity units that may transit this route per strategic turn,
    /// per commodity.  Zero means the route is installed but carries nothing.
    pub throughput_cap: f64,
}

impl TradeRoute {
    /// Create a new route with the given endpoints and cap.
    #[must_use]
    pub fn new(colony_a: ColonyId, colony_b: ColonyId, throughput_cap: f64) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            colony_a,
            colony_b,
            throughput_cap,
        }
    }
}

// ─── Trade override ───────────────────────────────────────────────────────────

/// Manual priority override for a specific commodity at a specific colony.
///
/// The player can:
/// - **Suppress** auto-flow for a commodity entirely (`suppress_auto = true`).
/// - **Cap** the auto-flow amount below the route cap (`cap = Some(x)`).
///
/// Both `suppress_auto` and `cap` interact in a predictable way:
/// `suppress_auto` wins over `cap` if both are set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOverride {
    /// Colony this override applies to.
    pub colony_id: ColonyId,
    /// Commodity identifier (matches colony pool keys).
    pub commodity_id: String,
    /// When `true`, auto-flow is completely suppressed for this commodity at
    /// this colony (neither importing nor exporting).
    pub suppress_auto: bool,
    /// Optional per-turn quantity cap below the route throughput cap.
    ///
    /// `None` means "no additional cap — use route `throughput_cap`".
    pub cap: Option<f64>,
}

// ─── Trade network ────────────────────────────────────────────────────────────

/// The planetary trade network: routes + per-colony overrides.
///
/// Stored inside [`GameState`](crate::turn::GameState) and updated via
/// `Command` variants.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TradeNetwork {
    /// All active infrastructure routes.
    pub routes: Vec<TradeRoute>,
    /// Per-colony-per-commodity manual overrides keyed by `(colony_id, commodity_id)`.
    pub overrides: HashMap<(ColonyId, String), TradeOverride>,
}

impl TradeNetwork {
    /// Create an empty [`TradeNetwork`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a route to the network.
    pub fn add_route(&mut self, route: TradeRoute) {
        self.routes.push(route);
    }

    /// Remove a route by its id.  Returns `true` if a route was removed.
    pub fn remove_route(&mut self, id: uuid::Uuid) -> bool {
        let before = self.routes.len();
        self.routes.retain(|r| r.id != id);
        self.routes.len() < before
    }

    /// Set (or replace) a manual override for a colony+commodity pair.
    pub fn set_override(&mut self, ov: TradeOverride) {
        self.overrides
            .insert((ov.colony_id, ov.commodity_id.clone()), ov);
    }

    /// Remove a manual override.  Returns `true` if one existed.
    pub fn clear_override(&mut self, colony_id: ColonyId, commodity_id: &str) -> bool {
        self.overrides
            .remove(&(colony_id, commodity_id.to_owned()))
            .is_some()
    }
}

// ─── Trade flow result ────────────────────────────────────────────────────────

/// A single commodity transfer executed during one strategic-turn trade pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeTransfer {
    /// Route that carried the transfer.
    pub route_id: uuid::Uuid,
    /// Colony the goods were shipped *from*.
    pub from_colony: ColonyId,
    /// Colony the goods were shipped *to*.
    pub to_colony: ColonyId,
    /// Commodity identifier.
    pub commodity_id: String,
    /// Quantity transferred.
    pub amount: f64,
}

/// Result produced by one [`run_trade_flow`] call.
#[derive(Debug, Clone, Default)]
pub struct TradeFlowResult {
    /// All transfers that actually moved goods this turn.
    pub transfers: Vec<TradeTransfer>,
}

// ─── Colony pool accessor ─────────────────────────────────────────────────────

/// Minimal interface required by [`run_trade_flow`] to read and mutate pool amounts.
///
/// This trait is satisfied by [`crate::colony::CommodityPool`] and can be
/// implemented by test stubs without pulling in the full colony module.
pub trait TradePool {
    /// Current amount of `commodity_id` in the pool (0.0 if absent).
    fn amount(&self, commodity_id: &str) -> f64;
    /// Add `qty` of `commodity_id` to the pool.
    fn deposit(&mut self, commodity_id: &str, qty: f64);
    /// Remove up to `qty` of `commodity_id` (removes min(qty, available)).
    fn withdraw(&mut self, commodity_id: &str, qty: f64);
}

// ─── Auto trade flow ──────────────────────────────────────────────────────────

/// Execute one strategic-turn trade flow pass over the given network and pools.
///
/// For each route and each commodity that has a non-zero balance between the two
/// endpoints, transfers goods from the surplus side to the deficit side subject
/// to the route's `throughput_cap` and any manual overrides.  Pools are mutated
/// in-place.
///
/// The `colonies` slice must be ordered so that `colony_index(id)` returns the
/// correct index into `pools`.
pub fn run_trade_flow<P: TradePool>(
    network: &TradeNetwork,
    colony_ids: &[ColonyId],
    pools: &mut [P],
    commodities: &[String],
) -> TradeFlowResult {
    let mut result = TradeFlowResult::default();

    // Helper: find index of a colony in the slice.
    let find = |id: ColonyId| colony_ids.iter().position(|&c| c == id);

    for route in &network.routes {
        if route.throughput_cap <= 0.0 {
            continue;
        }
        let Some(idx_a) = find(route.colony_a) else {
            continue;
        };
        let Some(idx_b) = find(route.colony_b) else {
            continue;
        };

        for commodity in commodities {
            // Per-colony override checks.
            let ov_a = network.overrides.get(&(route.colony_a, commodity.clone()));
            let ov_b = network.overrides.get(&(route.colony_b, commodity.clone()));

            // If *either* endpoint suppresses auto-flow for this commodity,
            // skip the automatic transfer.
            if ov_a.is_some_and(|o| o.suppress_auto) || ov_b.is_some_and(|o| o.suppress_auto) {
                continue;
            }

            let amount_a = pools[idx_a].amount(commodity);
            let amount_b = pools[idx_b].amount(commodity);

            // Only flow if there is a meaningful imbalance.
            if (amount_a - amount_b).abs() < f64::EPSILON {
                continue;
            }

            // Determine direction: flow from whichever side has more.
            let (from_idx, to_idx, from_id, to_id) = if amount_a > amount_b {
                (idx_a, idx_b, route.colony_a, route.colony_b)
            } else {
                (idx_b, idx_a, route.colony_b, route.colony_a)
            };

            let surplus = pools[from_idx].amount(commodity);
            let deficit_gap = (pools[to_idx].amount(commodity) - surplus).abs();

            // How much can we actually move?
            // Capped by: route throughput, available surplus, and any override cap.
            let mut cap = route
                .throughput_cap
                .min(surplus)
                .min(deficit_gap / 2.0 + surplus / 2.0);

            // If both values are positive, transfer half the difference to equalise.
            let transfer_ideal = (surplus - pools[to_idx].amount(commodity)) / 2.0;
            cap = cap.min(transfer_ideal);

            // Apply the most restrictive per-colony cap from either side.
            let override_cap = [ov_a, ov_b]
                .iter()
                .filter_map(|o| o.and_then(|o| o.cap))
                .fold(f64::INFINITY, f64::min);
            cap = cap.min(override_cap);

            // Cap to route throughput regardless.
            cap = cap.min(route.throughput_cap);

            if cap <= 0.0 {
                continue;
            }

            pools[from_idx].withdraw(commodity, cap);
            pools[to_idx].deposit(commodity, cap);

            result.transfers.push(TradeTransfer {
                route_id: route.id,
                from_colony: from_id,
                to_colony: to_id,
                commodity_id: commodity.clone(),
                amount: cap,
            });
        }
    }

    result
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal stub pool for tests.
    #[derive(Default, Clone)]
    struct StubPool(HashMap<String, f64>);

    impl TradePool for StubPool {
        fn amount(&self, id: &str) -> f64 {
            self.0.get(id).copied().unwrap_or(0.0)
        }
        fn deposit(&mut self, id: &str, qty: f64) {
            *self.0.entry(id.to_owned()).or_default() += qty;
        }
        fn withdraw(&mut self, id: &str, qty: f64) {
            let v = self.0.entry(id.to_owned()).or_default();
            *v = (*v - qty).max(0.0);
        }
    }

    fn colony_id() -> ColonyId {
        crate::colony::Colony::new("X").id
    }

    #[test]
    fn surplus_flows_to_deficit() {
        let a = colony_id();
        let b = colony_id();
        let route = TradeRoute::new(a, b, 100.0);
        let mut net = TradeNetwork::new();
        net.add_route(route);

        let mut pa = StubPool::default();
        pa.deposit("food", 80.0);
        let mut pb = StubPool::default();
        pb.deposit("food", 20.0);

        let commodities = vec!["food".to_owned()];
        let ids = vec![a, b];
        let mut pools = vec![pa, pb];

        let result = run_trade_flow(&net, &ids, &mut pools, &commodities);

        // Some food moved from a → b.
        assert!(
            !result.transfers.is_empty(),
            "expected at least one transfer"
        );
        // Net flow: a lost, b gained.
        assert!(
            pools[0].amount("food") < 80.0,
            "colony A should have exported some food"
        );
        assert!(
            pools[1].amount("food") > 20.0,
            "colony B should have imported some food"
        );
        // Conservation: total food unchanged.
        let total = pools[0].amount("food") + pools[1].amount("food");
        assert!(
            (total - 100.0).abs() < 1e-9,
            "total food must be conserved (got {total})"
        );
    }

    #[test]
    fn throughput_cap_is_respected() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::new(a, b, 10.0)); // cap = 10

        let mut pa = StubPool::default();
        pa.deposit("ore", 200.0);
        let mut pb = StubPool::default(); // no ore at all

        let commodities = vec!["ore".to_owned()];
        let ids = vec![a, b];
        let mut pools = vec![pa, pb];

        run_trade_flow(&net, &ids, &mut pools, &commodities);

        // Colony B should have received at most 10 ore.
        assert!(
            pools[1].amount("ore") <= 10.0 + f64::EPSILON,
            "throughput cap must not be exceeded (got {})",
            pools[1].amount("ore")
        );
    }

    #[test]
    fn manual_override_suppresses_auto_flow() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::new(a, b, 100.0));

        // Suppress auto-flow for "food" at colony A.
        net.set_override(TradeOverride {
            colony_id: a,
            commodity_id: "food".to_owned(),
            suppress_auto: true,
            cap: None,
        });

        let mut pa = StubPool::default();
        pa.deposit("food", 100.0);
        let mut pb = StubPool::default();

        let commodities = vec!["food".to_owned()];
        let ids = vec![a, b];
        let mut pools = vec![pa, pb];

        let result = run_trade_flow(&net, &ids, &mut pools, &commodities);

        assert!(
            result.transfers.is_empty(),
            "override suppress_auto must prevent all transfers"
        );
        assert!(
            (pools[0].amount("food") - 100.0).abs() < f64::EPSILON,
            "colony A food must be unchanged"
        );
        assert!(
            pools[1].amount("food") < f64::EPSILON,
            "colony B food must remain zero"
        );
    }

    #[test]
    fn override_cap_further_limits_flow() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::new(a, b, 100.0)); // route cap = 100

        // Player caps exports at 5 for colony A.
        net.set_override(TradeOverride {
            colony_id: a,
            commodity_id: "fuel".to_owned(),
            suppress_auto: false,
            cap: Some(5.0),
        });

        let mut pa = StubPool::default();
        pa.deposit("fuel", 80.0);
        let mut pb = StubPool::default(); // empty

        let commodities = vec!["fuel".to_owned()];
        let ids = vec![a, b];
        let mut pools = vec![pa, pb];

        run_trade_flow(&net, &ids, &mut pools, &commodities);

        assert!(
            pools[1].amount("fuel") <= 5.0 + f64::EPSILON,
            "override cap of 5 must be respected (got {})",
            pools[1].amount("fuel")
        );
    }

    #[test]
    fn no_flow_when_balanced() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::new(a, b, 100.0));

        let mut pa = StubPool::default();
        pa.deposit("water", 50.0);
        let mut pb = StubPool::default();
        pb.deposit("water", 50.0);

        let commodities = vec!["water".to_owned()];
        let ids = vec![a, b];
        let mut pools = vec![pa, pb];

        let result = run_trade_flow(&net, &ids, &mut pools, &commodities);
        assert!(
            result.transfers.is_empty(),
            "no transfer expected when colonies are balanced"
        );
    }

    #[test]
    fn route_removal_stops_flow() {
        let a = colony_id();
        let b = colony_id();
        let route = TradeRoute::new(a, b, 100.0);
        let route_id = route.id;
        let mut net = TradeNetwork::new();
        net.add_route(route);

        let removed = net.remove_route(route_id);
        assert!(removed, "route should have been removed");

        let mut pa = StubPool::default();
        pa.deposit("food", 100.0);
        let mut pb = StubPool::default();

        let commodities = vec!["food".to_owned()];
        let ids = vec![a, b];
        let mut pools = vec![pa, pb];

        let result = run_trade_flow(&net, &ids, &mut pools, &commodities);
        assert!(
            result.transfers.is_empty(),
            "no transfer after route removal"
        );
    }
}
