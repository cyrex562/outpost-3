//! Inter-colony trade — auto base-flow + manual priority override.
//!
//! Implements Phase 5 of the build sequence (DESIGN.md §8.1).
//!
//! # Design summary
//!
//! Goods flow automatically once a [`TradeRoute`] exists between two colonies.
//! Each strategic turn [`run_trade_flow`] is called with the full
//! [`TradeNetwork`], all colony commodity pools, and a per-colony **need
//! reserve**. It computes each colony's *surplus* — the amount held **above**
//! its reserve — for every commodity and ships it toward colonies whose own
//! surplus is lower, over every connected route, subject to the route's
//! `throughput_cap` per commodity per turn.
//!
//! # Only surpluses are tradeable
//!
//! Several commodities are also survival consumables: `water`, `oxygen`,
//! `food_ration`. They are genuine cargo — shipping water to a dry colony is
//! the logistics problem the network exists for — but a colony must never
//! export the stock its own population is about to consume. The reserve is what
//! enforces that: everything at or below it is invisible to trade, so a colony
//! can be a net exporter of water and still never starve itself.
//!
//! This is distinct from colony *resources* (`power`, `housing`, `research`),
//! which are not tradeable in any quantity and don't live in a `ColonyPool` at
//! all — see `colony::resource_pool` (issue #304).
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
    ///
    /// Serialized as a flat list of [`TradeOverride`] values (each already
    /// carries its `colony_id` + `commodity_id`), not as a JSON object:
    /// `serde_json` — the snapshot/save format — cannot serialize a map with a
    /// `(ColonyId, String)` tuple key, which used to fail every save with
    /// `"key must be a string"` once any override existed.
    #[serde(with = "overrides_serde")]
    pub overrides: HashMap<(ColonyId, String), TradeOverride>,
}

/// (De)serialize [`TradeNetwork::overrides`] as a flat `Vec<TradeOverride>`,
/// rebuilding the `(colony_id, commodity_id)` keys from each value on load —
/// the tuple key can't be a JSON object key. See the field's doc comment.
mod overrides_serde {
    use super::{ColonyId, HashMap, TradeOverride};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S: Serializer>(
        overrides: &HashMap<(ColonyId, String), TradeOverride>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let values: Vec<&TradeOverride> = overrides.values().collect();
        values.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<(ColonyId, String), TradeOverride>, D::Error> {
        let values = Vec::<TradeOverride>::deserialize(deserializer)?;
        Ok(values
            .into_iter()
            .map(|ov| ((ov.colony_id, ov.commodity_id.clone()), ov))
            .collect())
    }
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
pub fn run_trade_flow<P: TradePool, S: std::hash::BuildHasher>(
    network: &TradeNetwork,
    colony_ids: &[ColonyId],
    pools: &mut [P],
    commodities: &[String],
    reserves: &[HashMap<String, f64, S>],
) -> TradeFlowResult {
    let mut result = TradeFlowResult::default();

    // Helper: find index of a colony in the slice.
    let find = |id: ColonyId| colony_ids.iter().position(|&c| c == id);

    // Stock this colony must keep for its own consumption; everything above it
    // is the tradeable surplus. A colony with no entry reserves nothing, which
    // is correct for commodities no need consumes (ores, components).
    let reserve = |idx: usize, commodity: &str| -> f64 {
        reserves
            .get(idx)
            .and_then(|r| r.get(commodity))
            .copied()
            .unwrap_or(0.0)
    };
    let surplus_of = |pools: &[P], idx: usize, commodity: &str| -> f64 {
        (pools[idx].amount(commodity) - reserve(idx, commodity)).max(0.0)
    };

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

            // Compare *surpluses*, not raw stock. A colony sitting on exactly
            // its own need reserve has nothing to offer even if a neighbour has
            // none at all, and a colony below its reserve looks maximally
            // needy — which is what should pull imports in.
            let surplus_a = surplus_of(pools, idx_a, commodity);
            let surplus_b = surplus_of(pools, idx_b, commodity);

            // Only flow if there is a meaningful imbalance.
            if (surplus_a - surplus_b).abs() < f64::EPSILON {
                continue;
            }

            // Determine direction: flow from whichever side has more to spare.
            let (from_idx, to_idx, from_id, to_id) = if surplus_a > surplus_b {
                (idx_a, idx_b, route.colony_a, route.colony_b)
            } else {
                (idx_b, idx_a, route.colony_b, route.colony_a)
            };

            let surplus = surplus_of(pools, from_idx, commodity);
            let to_surplus = surplus_of(pools, to_idx, commodity);
            let deficit_gap = (to_surplus - surplus).abs();

            // How much can we actually move?
            // Capped by: route throughput, exportable surplus (never the
            // reserve), and any override cap.
            let mut cap = route
                .throughput_cap
                .min(surplus)
                .min(deficit_gap / 2.0 + surplus / 2.0);

            // Equalise the two surpluses, not the two stock levels.
            let transfer_ideal = (surplus - to_surplus) / 2.0;
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

    /// Run the flow pass with no need reserve — every colony is free to export
    /// everything it holds.
    ///
    /// Shadows [`run_trade_flow`] so the pre-reserve tests below keep reading as
    /// they did: they cover route capacity, direction, and override handling,
    /// none of which the reserve changes. Reserve behaviour has its own tests at
    /// the end of this module.
    fn run_trade_flow<P: TradePool>(
        network: &TradeNetwork,
        colony_ids: &[ColonyId],
        pools: &mut [P],
        commodities: &[String],
    ) -> TradeFlowResult {
        // Empty slice needs a concrete hasher for `S` to be inferable.
        let no_reserves: [HashMap<String, f64>; 0] = [];
        super::run_trade_flow(network, colony_ids, pools, commodities, &no_reserves)
    }

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
    // ── Need reserve: only surpluses are tradeable ────────────────────────────

    fn reserve(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
    }

    /// A colony must not export the water its own colonists are about to drink.
    ///
    /// Before the reserve existed, the flow pass equalised raw stock: a colony
    /// holding exactly enough water for its population would ship half of it to
    /// a neighbour with none, and starve itself.
    #[test]
    fn stock_at_or_below_the_reserve_is_never_exported() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::new(a, b, 100.0));

        let mut pa = StubPool::default();
        pa.deposit("water", 150.0);
        let pb = StubPool::default();
        let mut pools = vec![pa, pb];

        // Colony A needs all 150 for itself; B has no population, no reserve.
        let reserves = vec![reserve(&[("water", 150.0)]), HashMap::new()];

        let result =
            super::run_trade_flow(&net, &[a, b], &mut pools, &["water".to_string()], &reserves);

        assert!(
            result.transfers.is_empty(),
            "nothing should move: A's entire stock is its own reserve, got {:?}",
            result.transfers
        );
        assert!((pools[0].amount("water") - 150.0).abs() < f64::EPSILON);
        assert_eq!(pools[1].amount("water"), 0.0);
    }

    /// The surplus above the reserve *is* tradeable — this is cargo, after all.
    #[test]
    fn only_the_amount_above_the_reserve_is_offered_to_trade() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::new(a, b, 100.0));

        let mut pa = StubPool::default();
        pa.deposit("water", 250.0);
        let pb = StubPool::default();
        let mut pools = vec![pa, pb];

        // A reserves 150, so 100 is exportable; B reserves nothing.
        let reserves = vec![reserve(&[("water", 150.0)]), HashMap::new()];

        super::run_trade_flow(&net, &[a, b], &mut pools, &["water".to_string()], &reserves);

        let moved = pools[1].amount("water");
        assert!(moved > 0.0, "the 100-unit surplus should be tradeable");
        assert!(
            moved <= 100.0 + f64::EPSILON,
            "never more than the surplus: moved {moved}, surplus was 100"
        );
        assert!(
            pools[0].amount("water") >= 150.0 - f64::EPSILON,
            "the exporter must still hold its full reserve, has {}",
            pools[0].amount("water")
        );
    }

    /// A colony *below* its reserve is the neediest party and should pull
    /// imports, even from a colony that also has a reserve of its own.
    #[test]
    fn a_colony_below_its_reserve_receives_from_a_colony_in_surplus() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::new(a, b, 100.0));

        let mut pa = StubPool::default();
        pa.deposit("food_ration", 300.0);
        let mut pb = StubPool::default();
        pb.deposit("food_ration", 10.0);
        let mut pools = vec![pa, pb];

        // Both need 100; A has 200 spare, B is 90 short.
        let reserves = vec![
            reserve(&[("food_ration", 100.0)]),
            reserve(&[("food_ration", 100.0)]),
        ];

        super::run_trade_flow(
            &net,
            &[a, b],
            &mut pools,
            &["food_ration".to_string()],
            &reserves,
        );

        assert!(
            pools[1].amount("food_ration") > 10.0,
            "the short colony should have received food, has {}",
            pools[1].amount("food_ration")
        );
        assert!(
            pools[0].amount("food_ration") >= 100.0 - f64::EPSILON,
            "the exporter keeps its own reserve, has {}",
            pools[0].amount("food_ration")
        );
    }

    /// Commodities no need consumes have no reserve entry, so they trade freely.
    #[test]
    fn a_commodity_with_no_reserve_entry_trades_in_full() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::new(a, b, 100.0));

        let mut pa = StubPool::default();
        pa.deposit("structural_ore", 80.0);
        let pb = StubPool::default();
        let mut pools = vec![pa, pb];

        // Reserve map covers water only — ore isn't a survival need.
        let reserves = vec![reserve(&[("water", 500.0)]), HashMap::new()];

        super::run_trade_flow(
            &net,
            &[a, b],
            &mut pools,
            &["structural_ore".to_string()],
            &reserves,
        );

        assert!(
            pools[1].amount("structural_ore") > 0.0,
            "ore has no need reserve and should flow"
        );
    }
}
