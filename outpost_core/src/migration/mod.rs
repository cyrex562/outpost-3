//! Population migration, immigration waves, and evacuation — §6A.
//!
//! Mirrors the commodity-flow model from `§8.1`: auto base-flow plus directed
//! player override.  Migration has friction (time/distance, transport capacity,
//! willingness) so geography stays meaningful.
//!
//! # Flow overview
//!
//! * **Auto pull-flow** — each strategic month the engine computes an
//!   [`ColonyAttractiveness`] score per colony and moves a fraction of the
//!   population from less-attractive toward more-attractive neighbours that are
//!   connected by infrastructure.  The flow is small and voluntary; stability
//!   is not penalised.
//!
//! * **Directed migration / evacuation** — the player can force a move via
//!   [`PendingMigration`].  Forced moves (`forced = true`) incur a stability
//!   penalty on the sending colony when they depart.
//!
//! * **Immigration waves** — external colonist ships arrive at gateway colonies
//!   after a transit delay modelled as a [`PendingMigration`] with
//!   `from_colony = None`.
//!
//! * **Evacuation pressure** — when refugees arrive the receiving colony may
//!   become overcrowded (population > housing capacity), which applies an
//!   additional stability penalty proportional to overcrowding.

use crate::colony::ColonyId;

// ─── Attractiveness ───────────────────────────────────────────────────────────

/// Attractiveness score for one colony, used to compute auto migration flows.
///
/// Higher scores attract more migrants; lower scores push people away.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColonyAttractiveness {
    /// Colony this score describes.
    pub colony_id: ColonyId,
    /// Raw attractiveness in the range `[0.0, 1.0]`.
    pub score: f32,
}

/// Compute attractiveness for a colony from its current metrics.
///
/// Formula: weighted blend of stability (×0.5), housing headroom (×0.3), and
/// labor demand fill (×0.2).  All inputs are clamped to `[0.0, 1.0]` before
/// weighting.
///
/// * `stability`         — current stability scalar.
/// * `housing_capacity`  — total housing units available.
/// * `population`        — current head-count (used to compute headroom).
/// * `labor_demand_fill` — fraction of labor demand that is currently met;
///   pass `1.0` if unknown (conservative default).
#[must_use]
pub fn compute_attractiveness(
    colony_id: ColonyId,
    stability: f32,
    housing_capacity: f32,
    population: f32,
    labor_demand_fill: f32,
) -> ColonyAttractiveness {
    let housing_headroom = if housing_capacity <= 0.0 {
        0.0_f32
    } else {
        ((housing_capacity - population) / housing_capacity).clamp(0.0, 1.0)
    };
    let score = (0.5 * stability.clamp(0.0, 1.0)
        + 0.3 * housing_headroom
        + 0.2 * labor_demand_fill.clamp(0.0, 1.0))
    .clamp(0.0, 1.0);
    ColonyAttractiveness { colony_id, score }
}

// ─── Pending migration ────────────────────────────────────────────────────────

/// A batch of colonists in transit between colonies (or arriving from off-map).
///
/// Created when a migration departs; resolved when `turns_remaining` reaches 0.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingMigration {
    /// Stable identifier for this migration leg.
    pub id: uuid::Uuid,
    /// Sending colony; `None` for off-map immigration waves.
    pub from_colony: Option<ColonyId>,
    /// Receiving colony.
    pub to_colony: ColonyId,
    /// Number of colonists in transit.
    pub count: f32,
    /// Turns until arrival (counts down each strategic month).
    pub turns_remaining: u32,
    /// Whether this was a forced evacuation (`true`) or voluntary flow (`false`).
    pub forced: bool,
    /// Transport capacity cost consumed by this migration (reserved units).
    pub transport_cost: f32,
}

impl PendingMigration {
    /// Create a new pending migration.
    #[must_use]
    pub fn new(
        from_colony: Option<ColonyId>,
        to_colony: ColonyId,
        count: f32,
        turns_remaining: u32,
        forced: bool,
        transport_cost: f32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            from_colony,
            to_colony,
            count,
            turns_remaining,
            forced,
            transport_cost,
        }
    }

    /// Decrement `turns_remaining` by one.  Returns `true` when the migration arrives.
    pub fn tick(&mut self) -> bool {
        if self.turns_remaining > 0 {
            self.turns_remaining -= 1;
        }
        self.turns_remaining == 0
    }
}

// ─── Auto flow ────────────────────────────────────────────────────────────────

/// Parameters governing one strategic-month auto migration computation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AutoMigrationParams {
    /// Maximum fraction of a colony's population that can voluntarily leave per month.
    pub max_outflow_fraction: f32,
    /// Minimum attractiveness difference to trigger auto flow.
    pub min_attractiveness_delta: f32,
    /// Transit time in strategic months (same for all routes; use 1 for intra-planet).
    pub transit_turns: u32,
    /// Transport capacity cost per colonist in transit.
    pub transport_cost_per_colonist: f32,
}

impl Default for AutoMigrationParams {
    fn default() -> Self {
        Self {
            max_outflow_fraction: 0.05, // up to 5 % can leave per month
            min_attractiveness_delta: 0.10,
            transit_turns: 1,
            transport_cost_per_colonist: 0.1,
        }
    }
}

/// Compute the voluntary auto-flow migrations for one strategic month.
///
/// Returns a list of [`PendingMigration`]s to enqueue.  The caller is
/// responsible for deducting the departed colonists from the source colony's
/// population immediately (they are "in transit") and enqueueing the pending
/// migration for later arrival.
///
/// # Arguments
///
/// * `attractiveness` — attractiveness scores for all colonies (any order).
/// * `populations`    — current population per colony, parallel to `colony_ids`.
/// * `colony_ids`     — colony identifiers, parallel to `populations`.
/// * `params`         — tunable flow parameters.
#[must_use]
pub fn compute_auto_flows(
    attractiveness: &[ColonyAttractiveness],
    populations: &[f32],
    colony_ids: &[ColonyId],
    params: &AutoMigrationParams,
) -> Vec<PendingMigration> {
    let mut flows = Vec::new();
    let n = colony_ids.len();
    if n < 2 {
        return flows;
    }

    for i in 0..n {
        let src_id = colony_ids[i];
        let src_pop = populations[i];
        if src_pop <= 0.0 {
            continue;
        }

        let src_score = attractiveness
            .iter()
            .find(|a| a.colony_id == src_id)
            .map_or(0.5, |a| a.score);

        // Find the most attractive colony that is better than this one.
        let best = attractiveness
            .iter()
            .enumerate()
            .filter(|(j, a)| {
                *j < n
                    && colony_ids[*j] != src_id
                    && a.score - src_score >= params.min_attractiveness_delta
            })
            .max_by(|(_, a), (_, b)| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some((_, dst_attr)) = best {
            // Flow proportional to attractiveness gap, capped at max_outflow_fraction.
            let delta = (dst_attr.score - src_score).clamp(0.0, 1.0);
            let fraction =
                (delta * params.max_outflow_fraction).clamp(0.0, params.max_outflow_fraction);
            let movers = (src_pop * fraction).floor();
            if movers < 1.0 {
                continue;
            }
            flows.push(PendingMigration::new(
                Some(src_id),
                dst_attr.colony_id,
                movers,
                params.transit_turns,
                false,
                movers * params.transport_cost_per_colonist,
            ));
        }
    }
    flows
}

// ─── Arrival resolution ───────────────────────────────────────────────────────

/// Result of resolving one arriving migration batch.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArrivalOutcome {
    /// The migration that arrived.
    pub migration_id: uuid::Uuid,
    /// Number of colonists that actually arrived (may be < `count` if capped).
    pub arrived: f32,
    /// Stability penalty applied to the receiving colony due to overcrowding.
    ///
    /// Negative value (or zero).  Added to the colony's stability this turn.
    pub overcrowding_stability_penalty: f32,
    /// Stability penalty applied to the sending colony for forced departure.
    ///
    /// Only non-zero for `forced = true` migrations.  Negative value (or zero).
    pub forced_departure_stability_penalty: f32,
}

/// Stability penalty per overcrowding unit (receiver colony).
pub const OVERCROWDING_PENALTY_PER_UNIT: f32 = 0.01;
/// Stability cost applied to the sending colony for a forced evacuation.
pub const FORCED_MOVE_STABILITY_COST: f32 = 0.05;

/// Resolve an arriving migration batch, computing stability effects.
///
/// * `housing_capacity` — total housing units at the receiving colony.
/// * `current_population` — population at the receiver *before* arrival.
///
/// Returns an [`ArrivalOutcome`] describing how many arrived and any penalties.
#[must_use]
pub fn resolve_arrival(
    migration: &PendingMigration,
    housing_capacity: f32,
    current_population: f32,
) -> ArrivalOutcome {
    let arrived = migration.count;
    let new_population = current_population + arrived;

    // Overcrowding: population exceeds housing capacity.
    let overcrowding_penalty = if housing_capacity > 0.0 && new_population > housing_capacity {
        let excess_fraction = (new_population - housing_capacity) / housing_capacity;
        -(excess_fraction.clamp(0.0, 1.0) * OVERCROWDING_PENALTY_PER_UNIT * arrived)
    } else {
        0.0
    };

    // Forced-move stability cost on the sending colony.
    let forced_penalty = if migration.forced {
        -FORCED_MOVE_STABILITY_COST
    } else {
        0.0
    };

    ArrivalOutcome {
        migration_id: migration.id,
        arrived,
        overcrowding_stability_penalty: overcrowding_penalty,
        forced_departure_stability_penalty: forced_penalty,
    }
}

// ─── Population trajectory warning ───────────────────────────────────────────

/// Track the last N population samples to support predictive warnings.
///
/// Analogous to [`crate::interrupt::StabilityTracker`] but for population count.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PopulationTracker {
    /// Recent population samples (head-count), oldest first.
    pub samples: std::collections::VecDeque<f32>,
}

impl PopulationTracker {
    /// Number of samples retained.
    pub const WINDOW: usize = 5;

    /// Record a new population sample.
    pub fn push(&mut self, count: f32) {
        self.samples.push_back(count);
        if self.samples.len() > Self::WINDOW {
            self.samples.pop_front();
        }
    }

    /// Estimate turns until population falls below `floor`, using linear extrapolation.
    ///
    /// Returns `None` when population is not declining or when fewer than two
    /// samples are available.
    #[must_use]
    pub fn eta_to_floor(&self, floor: f32) -> Option<u32> {
        if self.samples.len() < 2 {
            return None;
        }
        let current = *self.samples.back()?;
        if current <= floor {
            return None;
        }
        let first = f64::from(*self.samples.front()?);
        let current_f64 = f64::from(current);
        let floor_f64 = f64::from(floor);
        let n = f64::from(u32::try_from(self.samples.len() - 1).unwrap_or(1));
        let avg_delta = (current_f64 - first) / n;
        if avg_delta >= 0.0 {
            return None;
        }
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let turns = ((current_f64 - floor_f64) / (-avg_delta) - 1e-6).ceil() as u32;
        Some(turns)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> ColonyId {
        uuid::Uuid::new_v4()
    }

    // ── Attractiveness ─────────────────────────────────────────────────────

    #[test]
    fn full_stability_full_housing_is_attractive() {
        let cid = id();
        let a = compute_attractiveness(cid, 1.0, 200.0, 100.0, 1.0);
        assert!(
            a.score > 0.8,
            "full stability + full housing headroom should score > 0.8, got {}",
            a.score
        );
    }

    #[test]
    fn zero_stability_no_housing_is_unattractive() {
        let cid = id();
        let a = compute_attractiveness(cid, 0.0, 0.0, 100.0, 0.0);
        assert!(
            a.score < 0.1,
            "zero stability + no housing should score < 0.1, got {}",
            a.score
        );
    }

    #[test]
    fn overcrowded_colony_has_lower_score_than_spacious_one() {
        let id1 = id();
        let id2 = id();
        let crowded = compute_attractiveness(id1, 0.8, 100.0, 200.0, 1.0); // over capacity
        let spacious = compute_attractiveness(id2, 0.8, 300.0, 100.0, 1.0); // lots of room
        assert!(
            spacious.score > crowded.score,
            "spacious colony should be more attractive; spacious={} crowded={}",
            spacious.score,
            crowded.score
        );
    }

    // ── Auto flow ──────────────────────────────────────────────────────────

    #[test]
    fn auto_flow_moves_people_from_unattractive_to_attractive() {
        let id_src = id();
        let id_dst = id();
        let attractiveness = vec![
            ColonyAttractiveness {
                colony_id: id_src,
                score: 0.2,
            },
            ColonyAttractiveness {
                colony_id: id_dst,
                score: 0.8,
            },
        ];
        let populations = vec![500.0_f32, 300.0_f32];
        let colony_ids = vec![id_src, id_dst];
        let params = AutoMigrationParams::default();

        let flows = compute_auto_flows(&attractiveness, &populations, &colony_ids, &params);

        assert!(
            !flows.is_empty(),
            "should generate at least one migration flow"
        );
        let flow = &flows[0];
        assert_eq!(flow.from_colony, Some(id_src));
        assert_eq!(flow.to_colony, id_dst);
        assert!(flow.count >= 1.0, "should move at least 1 colonist");
    }

    #[test]
    fn auto_flow_does_not_trigger_below_min_delta() {
        let id_a = id();
        let id_b = id();
        // Only 0.05 difference, below default min delta of 0.10
        let attractiveness = vec![
            ColonyAttractiveness {
                colony_id: id_a,
                score: 0.50,
            },
            ColonyAttractiveness {
                colony_id: id_b,
                score: 0.55,
            },
        ];
        let params = AutoMigrationParams::default();
        let flows = compute_auto_flows(&attractiveness, &[200.0, 200.0], &[id_a, id_b], &params);
        assert!(
            flows.is_empty(),
            "delta below min_attractiveness_delta should not trigger flow"
        );
    }

    #[test]
    fn auto_flow_respects_max_outflow_fraction() {
        let id_src = id();
        let id_dst = id();
        let attractiveness = vec![
            ColonyAttractiveness {
                colony_id: id_src,
                score: 0.0,
            },
            ColonyAttractiveness {
                colony_id: id_dst,
                score: 1.0,
            },
        ];
        let pop = 1000.0_f32;
        let params = AutoMigrationParams {
            max_outflow_fraction: 0.05,
            ..Default::default()
        };
        let flows = compute_auto_flows(&attractiveness, &[pop, 100.0], &[id_src, id_dst], &params);
        assert!(!flows.is_empty());
        let moved = flows.iter().map(|f| f.count).sum::<f32>();
        assert!(
            moved <= pop * 0.05 + 1.0, // +1 for floor() rounding
            "outflow must not exceed max_outflow_fraction * pop: moved={moved}"
        );
    }

    // ── Arrival resolution ────────────────────────────────────────────────

    #[test]
    fn arrival_with_room_has_no_overcrowding_penalty() {
        let dst = id();
        let mig = PendingMigration::new(None, dst, 50.0, 0, false, 0.0);
        let outcome = resolve_arrival(&mig, 500.0, 100.0); // 500 capacity, 100 + 50 = 150
        assert!(
            outcome.overcrowding_stability_penalty >= 0.0
                || outcome.overcrowding_stability_penalty.abs() < 1e-6,
            "no overcrowding when pop < capacity; penalty={}",
            outcome.overcrowding_stability_penalty
        );
    }

    #[test]
    fn arrival_exceeding_housing_triggers_overcrowding_penalty() {
        let dst = id();
        let mig = PendingMigration::new(None, dst, 100.0, 0, false, 0.0);
        let outcome = resolve_arrival(&mig, 100.0, 80.0); // capacity=100, new_pop=180
        assert!(
            outcome.overcrowding_stability_penalty < 0.0,
            "overcrowding should apply negative stability penalty; got {}",
            outcome.overcrowding_stability_penalty
        );
    }

    #[test]
    fn forced_migration_incurs_departure_stability_cost() {
        let dst = id();
        let mig = PendingMigration::new(Some(id()), dst, 50.0, 0, true, 0.0);
        let outcome = resolve_arrival(&mig, 1000.0, 100.0);
        assert!(
            outcome.forced_departure_stability_penalty < 0.0,
            "forced migration should carry a departure stability cost"
        );
    }

    #[test]
    fn voluntary_migration_has_no_forced_departure_cost() {
        let dst = id();
        let mig = PendingMigration::new(Some(id()), dst, 50.0, 0, false, 0.0);
        let outcome = resolve_arrival(&mig, 1000.0, 100.0);
        assert!(
            (outcome.forced_departure_stability_penalty).abs() < 1e-6,
            "voluntary migration should not incur departure stability penalty"
        );
    }

    // ── Pending migration tick ─────────────────────────────────────────────

    #[test]
    fn pending_migration_ticks_down_to_zero() {
        let mut mig = PendingMigration::new(None, id(), 100.0, 3, false, 0.0);
        assert!(!mig.tick());
        assert!(!mig.tick());
        assert!(mig.tick(), "should arrive on third tick");
        assert_eq!(mig.turns_remaining, 0);
    }

    // ── Population tracker ─────────────────────────────────────────────────

    #[test]
    fn population_tracker_eta_declining() {
        let mut t = PopulationTracker::default();
        t.push(1000.0);
        t.push(500.0);
        let eta = t.eta_to_floor(0.0);
        assert!(eta.is_some());
        assert_eq!(
            eta.unwrap(),
            1,
            "eta should be 1 turn from 500 to 0 at -500/turn"
        );
    }

    #[test]
    fn population_tracker_stable_returns_none() {
        let mut t = PopulationTracker::default();
        for v in [100.0f32, 101.0, 102.0] {
            t.push(v);
        }
        assert!(t.eta_to_floor(50.0).is_none());
    }

    #[test]
    fn population_tracker_insufficient_data_returns_none() {
        let mut t = PopulationTracker::default();
        t.push(500.0);
        assert!(t.eta_to_floor(0.0).is_none());
    }

    // ── Immigration wave (off-map) ─────────────────────────────────────────

    #[test]
    fn immigration_wave_arrives_as_pending_migration_from_none() {
        let gateway = id();
        // An immigration wave is a PendingMigration with from_colony=None
        let wave = PendingMigration::new(None, gateway, 200.0, 2, false, 0.0);
        assert!(
            wave.from_colony.is_none(),
            "immigration wave has no source colony"
        );
        assert_eq!(wave.to_colony, gateway);
        assert_eq!(wave.count, 200.0);
        assert_eq!(wave.turns_remaining, 2);
    }
}
