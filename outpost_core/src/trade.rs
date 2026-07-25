//! Inter-colony trade — auto base-flow + manual priority override.
//!
//! Implements Phase 5 of the build sequence (DESIGN.md §8.1).
//!
//! # Design summary
//!
//! Goods flow automatically once a [`TradeRoute`] exists between two colonies.
//! Each **sol** [`run_trade_flow`] is called with the full [`TradeNetwork`], all
//! colony commodity pools, and a per-colony **need reserve**. It computes each
//! colony's *surplus* — the amount held **above** its reserve — for every
//! commodity and sends it toward colonies whose own surplus is lower, over every
//! connected route, subject to the route's `throughput_cap` per commodity per sol.
//!
//! # Trade takes time (issue #332)
//!
//! A dispatch is not a teleport. The pass **withdraws** the cargo from the
//! sender's pool immediately and hands back a [`TradeConvoy`]; the receiver is
//! credited only when the convoy's `sols_remaining` reaches zero, `transit_sols`
//! later. While in flight the goods exist in [`TradeNetwork::convoys`] and in
//! neither pool.
//!
//! Two consequences the flow pass has to account for, both handled by
//! [`TradeNetwork::inbound_in_flight`]:
//!
//! - The sender's surplus drops at dispatch, so it cannot re-offer committed stock.
//! - The receiver's surplus does *not* rise until arrival, so the target figure
//!   must include cargo already on its way — otherwise every sol of a multi-sol
//!   route re-ships against a reading that cannot move yet, overshooting by
//!   roughly `transit_sols`×.
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

/// Default convoy transit time for a route with no distance information.
///
/// One sol: goods dispatched this sol arrive on the next. Deliberately not
/// zero — an instantaneous route is the teleporting behaviour convoys replaced,
/// and a one-sol pipeline is what makes in-flight cargo observable at all.
pub const DEFAULT_TRANSIT_SOLS: u32 = 1;

fn default_transit_sols() -> u32 {
    DEFAULT_TRANSIT_SOLS
}

/// A directed or bidirectional infrastructure link between two colonies.
///
/// Throughput is symmetric: up to `throughput_cap` units of any single
/// commodity can be *dispatched* in either direction per sol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRoute {
    /// Stable identifier for this route (for removal / UI).
    pub id: uuid::Uuid,
    /// One endpoint of the route.
    pub colony_a: ColonyId,
    /// Other endpoint of the route.
    pub colony_b: ColonyId,
    /// Maximum commodity units that may be dispatched along this route per sol,
    /// per commodity.  Zero means the route is installed but carries nothing.
    pub throughput_cap: f64,
    /// Sols a convoy spends in transit between the two endpoints (issue #332).
    ///
    /// A balance scalar, not a physical constant: it is derived from the
    /// endpoints' body distance when both colonies have a known home body, and
    /// falls back to [`DEFAULT_TRANSIT_SOLS`] otherwise (same body, or a colony
    /// founded without map context). `#[serde(default)]` so pre-#332 saves load
    /// with the fallback rather than an instant route.
    #[serde(default = "default_transit_sols")]
    pub transit_sols: u32,
}

impl TradeRoute {
    /// Create a new route with the given endpoints and cap, using
    /// [`DEFAULT_TRANSIT_SOLS`] for transit time.
    #[must_use]
    pub fn new(colony_a: ColonyId, colony_b: ColonyId, throughput_cap: f64) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            colony_a,
            colony_b,
            throughput_cap,
            transit_sols: DEFAULT_TRANSIT_SOLS,
        }
    }

    /// Create a new route with an explicit convoy transit time.
    ///
    /// `transit_sols` is clamped to at least 1 — a zero-transit route would
    /// deposit cargo in the same sol it was withdrawn, which is the instant
    /// teleport convoys exist to replace.
    #[must_use]
    pub fn with_transit(
        colony_a: ColonyId,
        colony_b: ColonyId,
        throughput_cap: f64,
        transit_sols: u32,
    ) -> Self {
        Self {
            transit_sols: transit_sols.max(1),
            ..Self::new(colony_a, colony_b, throughput_cap)
        }
    }
}

// ─── Trade convoy ─────────────────────────────────────────────────────────────

/// Cargo in flight between two colonies along a [`TradeRoute`] (issue #332).
///
/// Convoys are what make trade take time. `run_trade_flow` withdraws the cargo
/// from the sender's pool at **dispatch**, and it only lands in the receiver's
/// pool once `sols_remaining` reaches zero — so the goods are absent from both
/// pools while travelling, and the sender cannot re-offer stock it has already
/// committed.
///
/// This is deliberately *not* [`crate::system::CargoShipment`]. That type models
/// interplanetary bulk freight: it consumes a [`crate::system::Hauler`] from a
/// finite fleet, is keyed by body rather than colony, and is dispatched
/// explicitly by the player. Auto-trade is a standing arrangement between two
/// colonies that must keep working without a free hauler, and must work between
/// two colonies on the *same* body — where a body-to-body travel time is
/// undefined.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeConvoy {
    /// Stable identifier.
    pub id: uuid::Uuid,
    /// Route this convoy is travelling along.
    pub route_id: uuid::Uuid,
    /// Colony that dispatched the cargo (already debited).
    pub from_colony: ColonyId,
    /// Colony that will receive the cargo on arrival.
    pub to_colony: ColonyId,
    /// Commodity being carried.
    pub commodity_id: String,
    /// Quantity in the hold.
    pub amount: f64,
    /// Sols until arrival. Decremented once per sol; zero means "arriving now".
    pub sols_remaining: u32,
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
    /// Cargo currently in flight between colonies (issue #332).
    ///
    /// Already debited from the sender's pool and not yet credited to the
    /// receiver's, so this is the only place those goods exist. `#[serde(default)]`
    /// so pre-#332 saves load with an empty manifest.
    #[serde(default)]
    pub convoys: Vec<TradeConvoy>,
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

    /// Total quantity of `commodity_id` currently in flight *toward* `colony_id`.
    ///
    /// The flow pass adds this to the destination's surplus when deciding what to
    /// dispatch. Without it a sender re-ships every sol against a receiver figure
    /// that will not move until the first convoy lands, overshooting the target
    /// by roughly `transit_sols` times the intended amount.
    #[must_use]
    pub fn inbound_in_flight(&self, colony_id: ColonyId, commodity_id: &str) -> f64 {
        self.convoys
            .iter()
            .filter(|c| c.to_colony == colony_id && c.commodity_id == commodity_id)
            .map(|c| c.amount)
            .sum()
    }

    /// Advance every convoy by one sol and return those that have arrived.
    ///
    /// Arrived convoys are removed from the manifest; the caller is responsible
    /// for crediting their cargo to the destination pool. A convoy whose
    /// destination no longer exists is still returned, so the caller decides
    /// what to do with orphaned cargo rather than having it silently vanish here.
    pub fn advance_convoys(&mut self) -> Vec<TradeConvoy> {
        for convoy in &mut self.convoys {
            convoy.sols_remaining = convoy.sols_remaining.saturating_sub(1);
        }
        let arrived: Vec<TradeConvoy> = self
            .convoys
            .iter()
            .filter(|c| c.sols_remaining == 0)
            .cloned()
            .collect();
        self.convoys.retain(|c| c.sols_remaining > 0);
        arrived
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
    /// Every dispatch decision made this sol.
    ///
    /// Since #332 a transfer records goods **debited and sent**, not goods
    /// received — the matching credit happens `transit_sols` later when the
    /// convoy lands.
    pub transfers: Vec<TradeTransfer>,
    /// Convoys created by this pass, for the caller to add to
    /// [`TradeNetwork::convoys`].
    ///
    /// Returned rather than pushed directly because the pass borrows the network
    /// immutably; the cargo is already withdrawn from the sender's pool, so
    /// **dropping this instead of storing it destroys goods**.
    pub dispatched: Vec<TradeConvoy>,
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

/// Execute one per-sol trade dispatch pass over the given network and pools.
///
/// For each route and each commodity with a non-zero surplus imbalance between
/// the two endpoints, **withdraws** goods from the surplus side and returns a
/// [`TradeConvoy`] carrying them, subject to the route's `throughput_cap` and any
/// manual overrides. Nothing is deposited here: the destination is credited when
/// the convoy arrives (see [`TradeNetwork::advance_convoys`]).
///
/// The destination's effective surplus includes cargo already in flight toward it
/// ([`TradeNetwork::inbound_in_flight`]), so a multi-sol route does not get
/// re-dispatched against a stale reading every sol.
///
/// The `colony_ids` slice must be ordered so that its index matches `pools`.
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
    // What this colony could actually put on a convoy right now.
    let surplus_of = |pools: &[P], idx: usize, commodity: &str| -> f64 {
        (pools[idx].amount(commodity) - reserve(idx, commodity)).max(0.0)
    };
    // What this colony will be holding once everything already sent to it lands.
    // Direction and volume are decided on this figure so a route longer than one
    // sol isn't re-dispatched against a receiver reading that cannot move yet.
    //
    // `pending` carries dispatches made earlier in *this* pass, which are not yet
    // in `network.convoys`. Without it, two routes feeding the same colony would
    // each ship a full share against the same stale reading.
    let effective_surplus = |pools: &[P],
                             idx: usize,
                             commodity: &str,
                             pending: &HashMap<(ColonyId, String), f64>|
     -> f64 {
        surplus_of(pools, idx, commodity)
            + network.inbound_in_flight(colony_ids[idx], commodity)
            + pending
                .get(&(colony_ids[idx], commodity.to_owned()))
                .copied()
                .unwrap_or(0.0)
    };

    // (destination colony, commodity) → quantity dispatched so far this pass.
    let mut pending_inbound: HashMap<(ColonyId, String), f64> = HashMap::new();

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
            let surplus_a = effective_surplus(pools, idx_a, commodity, &pending_inbound);
            let surplus_b = effective_surplus(pools, idx_b, commodity, &pending_inbound);

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

            // The sender can only load what it physically holds above its
            // reserve; the *target* is set by the effective figures, which
            // account for cargo already on its way.
            let loadable = surplus_of(pools, from_idx, commodity);
            let from_effective = effective_surplus(pools, from_idx, commodity, &pending_inbound);
            let to_effective = effective_surplus(pools, to_idx, commodity, &pending_inbound);

            // How much can we actually move?
            // Capped by: route throughput, loadable surplus (never the
            // reserve), and any override cap.
            let mut cap = route.throughput_cap.min(loadable);

            // Equalise the two surpluses, not the two stock levels.
            let transfer_ideal = (from_effective - to_effective) / 2.0;
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

            // Debit the sender now, credit the receiver on arrival. The cargo
            // lives only in `result.dispatched` until then, so the caller must
            // store it (see `TradeFlowResult::dispatched`).
            pools[from_idx].withdraw(commodity, cap);
            *pending_inbound
                .entry((to_id, commodity.clone()))
                .or_default() += cap;
            result.dispatched.push(TradeConvoy {
                id: uuid::Uuid::new_v4(),
                route_id: route.id,
                from_colony: from_id,
                to_colony: to_id,
                commodity_id: commodity.clone(),
                amount: cap,
                sols_remaining: route.transit_sols.max(1),
            });

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

    /// Run the flow pass with no need reserve and settle every convoy at once,
    /// so a single call moves goods end-to-end.
    ///
    /// Shadows [`run_trade_flow`] so the tests below keep reading as they did:
    /// they cover route capacity, direction, override handling, and reserve
    /// arithmetic, none of which convoy *latency* changes. Latency, in-flight
    /// accounting, and arrival have their own tests at the end of this module,
    /// which call `super::run_trade_flow` directly.
    fn run_trade_flow<P: TradePool>(
        network: &TradeNetwork,
        colony_ids: &[ColonyId],
        pools: &mut [P],
        commodities: &[String],
    ) -> TradeFlowResult {
        // Empty slice needs a concrete hasher for `S` to be inferable.
        let no_reserves: [HashMap<String, f64>; 0] = [];
        settle(
            super::run_trade_flow(network, colony_ids, pools, commodities, &no_reserves),
            colony_ids,
            pools,
        )
    }

    /// Credit every convoy in `result` to its destination pool immediately.
    ///
    /// Stands in for the turn processor's arrival step so a test can assert on
    /// end-to-end movement in one call.
    fn settle<P: TradePool>(
        result: TradeFlowResult,
        colony_ids: &[ColonyId],
        pools: &mut [P],
    ) -> TradeFlowResult {
        for convoy in &result.dispatched {
            if let Some(idx) = colony_ids.iter().position(|&c| c == convoy.to_colony) {
                pools[idx].deposit(&convoy.commodity_id, convoy.amount);
            }
        }
        result
    }

    /// Run one dispatch pass with explicit reserves, leaving convoys in flight.
    fn dispatch_only<P: TradePool>(
        network: &TradeNetwork,
        colony_ids: &[ColonyId],
        pools: &mut [P],
        commodities: &[String],
    ) -> TradeFlowResult {
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

        let flow =
            super::run_trade_flow(&net, &[a, b], &mut pools, &["water".to_string()], &reserves);
        settle(flow, &[a, b], &mut pools);

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

        let flow = super::run_trade_flow(
            &net,
            &[a, b],
            &mut pools,
            &["food_ration".to_string()],
            &reserves,
        );
        settle(flow, &[a, b], &mut pools);

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

        let flow = super::run_trade_flow(
            &net,
            &[a, b],
            &mut pools,
            &["structural_ore".to_string()],
            &reserves,
        );
        settle(flow, &[a, b], &mut pools);

        assert!(
            pools[1].amount("structural_ore") > 0.0,
            "ore has no need reserve and should flow"
        );
    }

    // ── Convoys: trade takes time (issue #332) ────────────────────────────

    /// The core of the convoy model: the sender is debited now, the receiver is
    /// credited later. Nothing teleports.
    #[test]
    fn dispatched_cargo_leaves_the_sender_before_it_reaches_the_receiver() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::with_transit(a, b, 100.0, 3));

        let mut pa = StubPool::default();
        pa.deposit("water", 100.0);
        let mut pools = vec![pa, StubPool::default()];

        let flow = dispatch_only(&net, &[a, b], &mut pools, &["water".to_string()]);
        net.convoys.extend(flow.dispatched);

        let sent: f64 = net.convoys.iter().map(|c| c.amount).sum();
        assert!(sent > 0.0, "something should have been dispatched");
        assert!(
            (pools[0].amount("water") - (100.0 - sent)).abs() < 1e-9,
            "the sender must be debited at dispatch: holds {}, sent {sent}",
            pools[0].amount("water")
        );
        assert_eq!(
            pools[1].amount("water"),
            0.0,
            "the receiver must not be credited until the convoy lands"
        );

        // Two sols of travel leave it in flight; the third lands it.
        for sol in 1..3 {
            let arrived = net.advance_convoys();
            assert!(
                arrived.is_empty(),
                "should still be in transit at sol {sol}"
            );
        }
        let arrived = net.advance_convoys();
        assert_eq!(arrived.len(), 1, "the convoy should land on its third sol");
        assert!((arrived[0].amount - sent).abs() < 1e-9);
        assert!(
            net.convoys.is_empty(),
            "manifest should be clear after arrival"
        );
    }

    /// A one-sol route — the default — lands on the very next sol.
    #[test]
    fn a_default_route_lands_its_convoy_on_the_next_sol() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::new(a, b, 100.0));
        assert_eq!(net.routes[0].transit_sols, DEFAULT_TRANSIT_SOLS);

        let mut pa = StubPool::default();
        pa.deposit("water", 100.0);
        let mut pools = vec![pa, StubPool::default()];

        let flow = dispatch_only(&net, &[a, b], &mut pools, &["water".to_string()]);
        net.convoys.extend(flow.dispatched);

        let arrived = net.advance_convoys();
        assert_eq!(arrived.len(), 1, "a one-sol route lands immediately after");
    }

    /// A zero transit time is clamped away: it would deposit cargo in the same
    /// sol it was withdrawn, which is the teleport convoys replaced.
    #[test]
    fn a_zero_transit_route_is_clamped_to_one_sol() {
        let a = colony_id();
        let b = colony_id();
        assert_eq!(TradeRoute::with_transit(a, b, 10.0, 0).transit_sols, 1);
    }

    /// The regression this model is most likely to introduce: on a multi-sol
    /// route the receiver's stock cannot move until the first convoy lands, so a
    /// pass that ignores in-flight cargo re-ships a full share every sol and
    /// overshoots by roughly `transit_sols`×.
    #[test]
    fn a_multi_sol_route_does_not_re_ship_against_a_stale_receiver_reading() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::with_transit(a, b, 1000.0, 5));

        let mut pa = StubPool::default();
        pa.deposit("water", 100.0);
        let mut pools = vec![pa, StubPool::default()];

        // First pass: half the surplus goes out (equalisation).
        let flow = dispatch_only(&net, &[a, b], &mut pools, &["water".to_string()]);
        net.convoys.extend(flow.dispatched);
        let first = net.convoys.iter().map(|c| c.amount).sum::<f64>();
        assert!(
            (first - 50.0).abs() < 1e-9,
            "expected half of 100, got {first}"
        );

        // Four more passes while that convoy is still in flight. With in-flight
        // accounting the two sides already look equal, so nothing more is sent.
        for _ in 0..4 {
            let flow = dispatch_only(&net, &[a, b], &mut pools, &["water".to_string()]);
            net.convoys.extend(flow.dispatched);
        }

        let total_sent: f64 = net.convoys.iter().map(|c| c.amount).sum();
        assert!(
            (total_sent - 50.0).abs() < 1e-9,
            "only the first pass should have shipped; total in flight {total_sent}"
        );
        assert!(
            (pools[0].amount("water") - 50.0).abs() < 1e-9,
            "the sender should still hold its half, has {}",
            pools[0].amount("water")
        );
    }

    /// Two routes feeding one colony in the same pass must not each ship a full
    /// share — the second needs to see the first, which is not yet in
    /// `network.convoys`.
    #[test]
    fn two_routes_into_one_colony_do_not_each_ship_a_full_share() {
        let a = colony_id();
        let b = colony_id();
        let sink = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::with_transit(a, sink, 1000.0, 4));
        net.add_route(TradeRoute::with_transit(b, sink, 1000.0, 4));

        let mut pa = StubPool::default();
        pa.deposit("water", 100.0);
        let mut pb = StubPool::default();
        pb.deposit("water", 100.0);
        let mut pools = vec![pa, pb, StubPool::default()];

        let flow = dispatch_only(&net, &[a, b, sink], &mut pools, &["water".to_string()]);

        let inbound: f64 = flow
            .dispatched
            .iter()
            .filter(|c| c.to_colony == sink)
            .map(|c| c.amount)
            .sum();
        // A ships 50 (half its surplus). B then sees the sink already expecting
        // 50, so its own equalisation target is 25, not another 50.
        assert!(
            inbound < 100.0,
            "the second route must account for the first: inbound {inbound}"
        );
        assert!(
            inbound > 50.0,
            "the second route should still send something: {inbound}"
        );
    }

    /// Nothing is created or destroyed by a dispatch-then-arrive cycle.
    #[test]
    fn a_dispatch_and_arrival_cycle_conserves_the_commodity() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        net.add_route(TradeRoute::with_transit(a, b, 1000.0, 2));

        let mut pa = StubPool::default();
        pa.deposit("water", 90.0);
        let mut pools = vec![pa, StubPool::default()];

        let flow = dispatch_only(&net, &[a, b], &mut pools, &["water".to_string()]);
        net.convoys.extend(flow.dispatched);

        // Mid-flight the goods are in neither pool — the manifest is the only
        // place they exist, which is why the caller must store `dispatched`.
        net.advance_convoys();
        let in_pools = pools[0].amount("water") + pools[1].amount("water");
        let in_flight: f64 = net.convoys.iter().map(|c| c.amount).sum();
        assert!(in_flight > 0.0, "expected cargo still in transit");
        assert!(
            (in_pools + in_flight - 90.0).abs() < 1e-9,
            "pools {in_pools} + in flight {in_flight} should still be 90"
        );

        for convoy in net.advance_convoys() {
            let idx = if convoy.to_colony == a { 0 } else { 1 };
            pools[idx].deposit(&convoy.commodity_id, convoy.amount);
        }
        let total = pools[0].amount("water") + pools[1].amount("water");
        assert!(
            (total - 90.0).abs() < 1e-9,
            "the full 90 should be back in the pools, found {total}"
        );
    }

    /// `inbound_in_flight` is scoped to one destination and one commodity.
    #[test]
    fn inbound_in_flight_counts_only_the_matching_colony_and_commodity() {
        let a = colony_id();
        let b = colony_id();
        let mut net = TradeNetwork::new();
        let route_id = uuid::Uuid::new_v4();
        let convoy = |to: ColonyId, commodity: &str, amount: f64| TradeConvoy {
            id: uuid::Uuid::new_v4(),
            route_id,
            from_colony: a,
            to_colony: to,
            commodity_id: commodity.to_owned(),
            amount,
            sols_remaining: 2,
        };
        net.convoys.push(convoy(b, "water", 10.0));
        net.convoys.push(convoy(b, "water", 5.0));
        net.convoys.push(convoy(b, "food_ration", 100.0));
        net.convoys.push(convoy(a, "water", 999.0));

        assert!((net.inbound_in_flight(b, "water") - 15.0).abs() < f64::EPSILON);
        assert!((net.inbound_in_flight(b, "food_ration") - 100.0).abs() < f64::EPSILON);
        assert_eq!(net.inbound_in_flight(b, "structural_ore"), 0.0);
    }
}
