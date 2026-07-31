//! Expeditions — textured exploration with events, encounters, and mid-mission decisions.
//!
//! Covers DESIGN.md §9: schematic system node maps, expedition types, survey outcomes,
//! anomalies, and mid-mission interrupts that reuse the interrupt + predicate system.
//!
//! # Architecture
//!
//! - [`Expedition`] — M8 field expedition with crew, supplies, sol-based transit.
//! - [`ExpeditionStatus`] — M8 lifecycle phases: `InTransit` → `OnSite` → `Returning` → `Completed` / `Lost`.
//! - [`ExpeditionType`] — the four mission profiles.
//! - [`ExpeditionState`] — live per-expedition state tracked in [`ExpeditionRegistry`].
//! - [`SurveyOutcome`] — full / partial / failed reveal produced at mission end.
//! - [`AnomalyDef`] — data-pack entry for an anomaly (loaded from YAML/JSON).
//! - [`MidMissionEvent`] — an encounter/decision injected into the expedition.
//! - [`ExpeditionRegistry`] — in-memory store of all active and completed expeditions.
//! - [`resolve_survey`] — pure function that rolls a survey outcome from a seed + modifiers.
//! - [`check_anomaly_trigger`] — pure function that tests whether an anomaly fires this turn.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::colony::ColonyId;
use crate::interrupt::Tier;
use crate::map::HexCoord;
use crate::system::BodyId;

// ─── M8: Field Expedition (issue #103) ───────────────────────────────────────

/// Stable identifier for a field expedition mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FieldExpeditionId(pub Uuid);

impl FieldExpeditionId {
    /// Create a new random [`FieldExpeditionId`].
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for FieldExpeditionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Lifecycle status for a field expedition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpeditionStatus {
    /// Crew is travelling to the target hex.
    InTransit,
    /// Crew has arrived and is actively exploring the target hex.
    OnSite,
    /// Survey complete; crew is returning to origin colony.
    Returning,
    /// Expedition has returned and resources deposited.
    Completed,
    /// Expedition was lost due to supply depletion or recall failure.
    Lost,
}

/// A field expedition launched from a colony to explore a hex tile.
///
/// Advances each colony-sol; discovers resources on arrival; deposits them on return.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expedition {
    /// Stable identifier for this mission instance.
    pub id: FieldExpeditionId,
    /// Colony that launched this expedition.
    pub origin_colony: ColonyId,
    /// Target hex tile being explored.
    pub target_hex: HexCoord,
    /// Number of crew assigned to the mission.
    pub crew_count: u32,
    /// Supplies consumed per sol while in transit or on-site.
    pub supply_consumed_per_sol: f32,
    /// Sol counter at launch time.
    pub sol_launched: u64,
    /// Sol counter when the expedition is expected to arrive at the target.
    pub eta_sol: u64,
    /// Current lifecycle status.
    pub status: ExpeditionStatus,
    /// Supplies remaining.
    pub supplies_remaining: f32,
    /// Sol at which the expedition arrived on-site (set on arrival).
    pub sol_arrived: Option<u64>,
    /// Resources discovered on-site, to be deposited on return.
    pub discovered_resources: Vec<(String, f64)>,
    /// Whether this expedition is flagged as deep-space (contributes to megaproject).
    pub is_deep_space: bool,
}

impl Expedition {
    /// Construct a new field expedition.
    ///
    /// `transit_sols` is the number of sols to travel from origin to target.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        origin_colony: ColonyId,
        target_hex: HexCoord,
        crew_count: u32,
        supplies: f32,
        supply_consumed_per_sol: f32,
        current_sol: u64,
        transit_sols: u64,
        is_deep_space: bool,
    ) -> Self {
        Self {
            id: FieldExpeditionId::new(),
            origin_colony,
            target_hex,
            crew_count,
            supply_consumed_per_sol,
            sol_launched: current_sol,
            eta_sol: current_sol + transit_sols,
            status: ExpeditionStatus::InTransit,
            supplies_remaining: supplies,
            sol_arrived: None,
            discovered_resources: Vec::new(),
            is_deep_space,
        }
    }

    /// Returns `true` if the expedition is still active (not completed or lost).
    #[must_use]
    pub fn is_active(&self) -> bool {
        !matches!(
            self.status,
            ExpeditionStatus::Completed | ExpeditionStatus::Lost
        )
    }
}

/// Outcome of advancing one field expedition by one sol.
#[derive(Debug)]
pub enum ExpeditionAdvanceOutcome {
    /// Expedition arrived at the target hex this sol.
    Arrived {
        /// Expedition identifier.
        id: FieldExpeditionId,
    },
    /// Expedition made a resource discovery on-site.
    Discovery {
        /// Expedition identifier.
        id: FieldExpeditionId,
        /// Discovered resource commodity id.
        resource_id: String,
        /// Amount discovered.
        amount: f64,
    },
    /// Expedition began its return journey.
    StartedReturn {
        /// Expedition identifier.
        id: FieldExpeditionId,
    },
    /// Expedition returned to origin colony and deposited resources.
    Returned {
        /// Expedition identifier.
        id: FieldExpeditionId,
        /// Resources deposited into the colony pool.
        deposits: Vec<(String, f64)>,
    },
    /// Expedition was lost due to supply depletion.
    Lost {
        /// Expedition identifier.
        id: FieldExpeditionId,
    },
    /// Nothing notable happened this sol.
    Nominal,
}

/// Default transit sols for a field expedition that does not supply a distance.
pub const DEFAULT_TRANSIT_SOLS: u64 = 10;
/// Number of on-site sols before a basic expedition begins its return.
pub const DEFAULT_ONSITE_SOLS: u64 = 5;
/// Minimum supply margin before the expedition is considered at risk.
pub const SUPPLY_LOSS_THRESHOLD: f32 = 0.0;

// ─── Expedition Type ──────────────────────────────────────────────────────────

/// The four mission profiles available for planetary exploration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExpeditionType {
    /// Fast automated flyby; low cost, low data quality.
    FastFlybyProbe,
    /// Robotic orbital survey; moderate cost, good mineral mapping.
    OrbitalSurvey,
    /// Surface lander; higher risk, reveals site candidates.
    Lander,
    /// Crewed expedition; highest cost and risk, reveals everything.
    MannedExpedition,
}

impl ExpeditionType {
    /// Base number of turns required for this mission type.
    #[must_use]
    pub fn base_duration_turns(&self) -> u32 {
        match self {
            Self::FastFlybyProbe => 2,
            Self::OrbitalSurvey => 5,
            Self::Lander => 8,
            Self::MannedExpedition => 12,
        }
    }

    /// Base probability (0–1) of a **full** reveal for this mission type.
    #[must_use]
    pub fn base_full_reveal_prob(&self) -> f32 {
        match self {
            Self::FastFlybyProbe => 0.10,
            Self::OrbitalSurvey => 0.35,
            Self::Lander => 0.60,
            Self::MannedExpedition => 0.85,
        }
    }

    /// Base probability (0–1) of a **partial** reveal (deposits only, no site).
    #[must_use]
    pub fn base_partial_reveal_prob(&self) -> f32 {
        match self {
            Self::FastFlybyProbe => 0.40,
            Self::OrbitalSurvey => 0.45,
            Self::Lander => 0.30,
            Self::MannedExpedition => 0.12,
        }
    }
    // Failure probability = 1 − full − partial (remainder).
}

// ─── Survey Outcome ───────────────────────────────────────────────────────────

/// The result of a completed survey mission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SurveyOutcome {
    /// Complete reveal: colony site candidate + resource deposit locations.
    FullReveal {
        /// Body that was surveyed.
        body_id: BodyId,
        /// Named candidate colony site.
        site_name: String,
        /// Resource deposits discovered (commodity id → estimated quantity).
        deposits: HashMap<String, f64>,
    },
    /// Partial reveal: deposit locations only, no viable colony site found.
    PartialReveal {
        /// Body that was surveyed.
        body_id: BodyId,
        /// Resource deposits discovered (commodity id → estimated quantity).
        deposits: HashMap<String, f64>,
    },
    /// Failed survey: hazard or equipment failure; no data returned.
    Failed {
        /// Body that was surveyed.
        body_id: BodyId,
        /// Human-readable reason for the failure.
        reason: String,
    },
}

/// Parameters that modulate survey outcome probabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SurveyModifiers {
    /// Additive bonus to full-reveal probability (from propulsion or sensor tech).
    pub full_reveal_bonus: f32,
    /// Additive penalty to full-reveal probability (from hazard events).
    pub full_reveal_penalty: f32,
    /// Additive bonus to partial-reveal probability.
    pub partial_reveal_bonus: f32,
}

/// Resolve a survey outcome given an expedition type, modifiers, and a deterministic seed.
///
/// Uses a simple linear-congruential roll so that tests can pass a fixed seed and assert
/// a deterministic outcome.  The seed is typically derived from the expedition id XOR
/// the current turn counter.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn resolve_survey(
    expedition_type: ExpeditionType,
    body_id: BodyId,
    modifiers: &SurveyModifiers,
    roll: f32, // caller supplies a [0,1) float from the RNG
    site_name: impl Into<String>,
    deposits: HashMap<String, f64>,
) -> SurveyOutcome {
    let full_p = (expedition_type.base_full_reveal_prob() + modifiers.full_reveal_bonus
        - modifiers.full_reveal_penalty)
        .clamp(0.0, 1.0);
    let partial_p = (expedition_type.base_partial_reveal_prob() + modifiers.partial_reveal_bonus)
        .clamp(0.0, 1.0 - full_p);

    if roll < full_p {
        SurveyOutcome::FullReveal {
            body_id,
            site_name: site_name.into(),
            deposits,
        }
    } else if roll < full_p + partial_p {
        SurveyOutcome::PartialReveal { body_id, deposits }
    } else {
        SurveyOutcome::Failed {
            body_id,
            reason: "Equipment failure or hazard encountered".to_string(),
        }
    }
}

// ─── Anomaly ─────────────────────────────────────────────────────────────────

/// A data-pack entry describing an anomaly that can be discovered during an expedition.
///
/// Loaded from YAML/JSON content packs; never hardcoded in the kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDef {
    /// Unique content-pack identifier.
    pub id: String,
    /// Human-readable name shown to the player.
    pub name: String,
    /// Probability per eligible turn that this anomaly triggers (0–1).
    pub trigger_probability: f32,
    /// Which expedition types can encounter this anomaly.
    pub eligible_expedition_types: Vec<ExpeditionType>,
    /// Description text shown to the player when the anomaly is encountered.
    pub description: String,
    /// Possible investigation outcomes (tech, resource, or narrative).
    pub outcomes: Vec<AnomalyOutcome>,
}

/// One possible result of investigating an anomaly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyOutcome {
    /// Identifier of this outcome variant.
    pub id: String,
    /// Probability weight relative to other outcomes (normalised at resolution).
    pub weight: f32,
    /// Player-facing description of what was discovered.
    pub description: String,
    /// Bonus research points awarded on this outcome.
    pub research_bonus: f32,
    /// Commodity rewards (commodity id → quantity).
    pub resource_reward: HashMap<String, f64>,
    /// Tech node unlocked by this outcome, if any.
    pub unlocks_tech: Option<String>,
}

/// Return `true` if the anomaly triggers this turn, given a `[0,1)` roll.
///
/// The expedition must be of an eligible type; otherwise the anomaly never fires.
#[must_use]
pub fn check_anomaly_trigger(
    anomaly: &AnomalyDef,
    expedition_type: ExpeditionType,
    roll: f32,
) -> bool {
    anomaly.eligible_expedition_types.contains(&expedition_type)
        && roll < anomaly.trigger_probability
}

/// Pick which [`AnomalyOutcome`] investigating `anomaly` resolves to, given a
/// `[0,1)` roll, weighted by each outcome's `weight` (normalised internally).
///
/// Returns `None` only if `anomaly.outcomes` is empty (a content-authoring
/// error the loader should reject before this is ever called).
#[must_use]
pub fn resolve_anomaly_outcome(anomaly: &AnomalyDef, roll: f32) -> Option<&AnomalyOutcome> {
    let total_weight: f32 = anomaly.outcomes.iter().map(|o| o.weight.max(0.0)).sum();
    if total_weight <= 0.0 {
        return anomaly.outcomes.first();
    }
    let target = roll.clamp(0.0, 0.999_999) * total_weight;
    let mut cumulative = 0.0;
    for outcome in &anomaly.outcomes {
        cumulative += outcome.weight.max(0.0);
        if target < cumulative {
            return Some(outcome);
        }
    }
    anomaly.outcomes.last()
}

// ─── Deterministic rolls ──────────────────────────────────────────────────────

/// Derive a deterministic pseudo-random roll in `[0, 1)` from an expedition
/// id and a salt (e.g. the current sol, or a purpose-specific constant).
///
/// Keeps survey/anomaly resolution reproducible from saved state without
/// threading an external RNG stream through the engine — matching the
/// existing field-expedition system's deterministic-arithmetic approach
/// (see `Command::AdvanceColonySol`'s Step 4e discovery roll).
#[must_use]
pub fn deterministic_roll(id: Uuid, salt: u64) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    let mixed = (id.as_u128() as u64) ^ salt;
    let hashed = mixed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(0x1234_5678);
    // Top 24 bits, well-distributed, scaled into [0, 1).
    #[allow(clippy::cast_precision_loss)]
    let value = (hashed >> 40) as f32 / 16_777_216.0_f32;
    value.clamp(0.0, 0.999_999)
}

/// FNV-1a hash of a string into a `u64`, used to derive a per-anomaly salt
/// for [`deterministic_roll`] so different anomalies checked on the same
/// sol don't share a roll.
#[must_use]
pub fn string_salt(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in s.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

// ─── Mid-Mission Event / Decision ────────────────────────────────────────────

/// A mid-mission event injected into an active expedition.
///
/// Some events are [`Tier::Blocking`] and halt fast-forward until resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidMissionEvent {
    /// Stable identifier for this event instance.
    pub id: Uuid,
    /// Human-readable title.
    pub title: String,
    /// Description presented to the player.
    pub description: String,
    /// Interrupt tier; [`Tier::Blocking`] halts fast-forward.
    pub tier: Tier,
    /// Available player choices.  Empty = informational only (auto-dismiss).
    pub choices: Vec<MissionChoice>,
}

/// A single choice available during a mid-mission event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionChoice {
    /// Identifier used when submitting [`Command::ResolveMissionDecision`].
    pub id: String,
    /// Label shown on the button/option.
    pub label: String,
    /// Effect applied to the expedition's modifiers when chosen.
    pub effect: ChoiceEffect,
}

/// Effect applied to the active expedition when a choice is made.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChoiceEffect {
    /// Boost the survey's full-reveal probability.
    BoostFullReveal(f32),
    /// Penalise the survey's full-reveal probability (e.g. abort instrument).
    PenaliseFullReveal(f32),
    /// Abort the expedition entirely (returns a `Failed` outcome immediately).
    AbortMission,
    /// No mechanical effect; purely narrative.
    Narrative,
    /// Grant the rewards of an already-resolved anomaly investigation
    /// outcome (issue #235). The outcome is pre-rolled when the mid-mission
    /// event is created (see [`resolve_anomaly_outcome`]); choosing this
    /// option applies its `research_bonus`/`resource_reward`/`unlocks_tech`
    /// to live game state. The engine layer (not [`ExpeditionRegistry`])
    /// applies the reward, since that requires access to `GameState`'s
    /// research pool, colony pools, and tech registry.
    GrantAnomalyOutcome(AnomalyOutcome),
}

// ─── Expedition State ─────────────────────────────────────────────────────────

/// Stable identifier for a single expedition mission.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExpeditionId(pub Uuid);

impl ExpeditionId {
    /// Create a new random [`ExpeditionId`].
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ExpeditionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Lifecycle phase of an expedition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpeditionPhase {
    /// Mission is in transit to the target body.
    InTransit,
    /// Actively surveying; events and anomalies may fire each turn.
    Surveying,
    /// Awaiting player decision on a mid-mission event.
    AwaitingDecision,
    /// Mission completed; outcome stored.
    Completed,
    /// Mission was aborted (equipment failure or player choice).
    Aborted,
}

/// Fixed transit-leg duration (turns) shared by every survey expedition,
/// regardless of mission profile (issue #235).
pub const SURVEY_TRANSIT_TURNS: u32 = 2;

/// Full live state of one expedition mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpeditionState {
    /// Stable identifier.
    pub id: ExpeditionId,
    /// Colony that launched and funds this expedition (issue #235) — the
    /// destination for any resource rewards and the context for tracking
    /// which colony's expedition this is.
    pub origin_colony: ColonyId,
    /// Mission profile.
    pub expedition_type: ExpeditionType,
    /// Target celestial body.
    pub target_body: BodyId,
    /// Current lifecycle phase.
    pub phase: ExpeditionPhase,
    /// Turns remaining in the transit leg (counts down first).
    pub transit_turns_remaining: u32,
    /// Turns remaining in the on-site survey leg (counts down once transit
    /// completes; a triggered anomaly pauses this countdown while
    /// `phase == AwaitingDecision`).
    pub survey_turns_remaining: u32,
    /// Accumulated modifier adjustments from mid-mission choices.
    pub modifiers: SurveyModifiers,
    /// Pending mid-mission event awaiting player resolution (if any).
    pub pending_event: Option<MidMissionEvent>,
    /// Final outcome (set when phase transitions to Completed or Aborted).
    pub outcome: Option<SurveyOutcome>,
    /// Anomalies already triggered (to avoid re-firing the same one).
    pub triggered_anomalies: Vec<String>,
}

impl ExpeditionState {
    /// Create a new expedition in the [`ExpeditionPhase::InTransit`] phase.
    #[must_use]
    pub fn new(
        expedition_type: ExpeditionType,
        target_body: BodyId,
        origin_colony: ColonyId,
    ) -> Self {
        Self {
            id: ExpeditionId::new(),
            origin_colony,
            expedition_type,
            target_body,
            phase: ExpeditionPhase::InTransit,
            transit_turns_remaining: SURVEY_TRANSIT_TURNS,
            survey_turns_remaining: expedition_type.base_duration_turns(),
            modifiers: SurveyModifiers::default(),
            pending_event: None,
            outcome: None,
            triggered_anomalies: Vec::new(),
        }
    }
}

// ─── Registry ─────────────────────────────────────────────────────────────────

/// In-memory store for all active and recently completed expeditions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExpeditionRegistry {
    /// All known expeditions keyed by their stable id.
    pub expeditions: HashMap<ExpeditionId, ExpeditionState>,
}

impl ExpeditionRegistry {
    /// Register a new expedition, returning its id.
    pub fn launch(&mut self, state: ExpeditionState) -> ExpeditionId {
        let id = state.id.clone();
        self.expeditions.insert(id.clone(), state);
        id
    }

    /// Look up an expedition by id.
    #[must_use]
    pub fn get(&self, id: &ExpeditionId) -> Option<&ExpeditionState> {
        self.expeditions.get(id)
    }

    /// Mutably look up an expedition by id.
    pub fn get_mut(&mut self, id: &ExpeditionId) -> Option<&mut ExpeditionState> {
        self.expeditions.get_mut(id)
    }

    /// Iterate over all expeditions.
    pub fn iter(&self) -> impl Iterator<Item = (&ExpeditionId, &ExpeditionState)> {
        self.expeditions.iter()
    }

    /// Apply a player decision to the pending mid-mission event.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the expedition is not currently in
    /// [`ExpeditionPhase::AwaitingDecision`] or the choice id is unknown.
    pub fn resolve_decision(
        &mut self,
        expedition_id: &ExpeditionId,
        choice_id: &str,
    ) -> Result<(), ExpeditionError> {
        let state = self
            .expeditions
            .get_mut(expedition_id)
            .ok_or(ExpeditionError::NotFound)?;

        if state.phase != ExpeditionPhase::AwaitingDecision {
            return Err(ExpeditionError::NotAwaitingDecision);
        }

        let event = state
            .pending_event
            .take()
            .ok_or(ExpeditionError::NotAwaitingDecision)?;

        let choice = event
            .choices
            .iter()
            .find(|c| c.id == choice_id)
            .ok_or_else(|| ExpeditionError::UnknownChoice(choice_id.to_string()))?;

        match &choice.effect {
            ChoiceEffect::BoostFullReveal(delta) => {
                state.modifiers.full_reveal_bonus += delta;
            }
            ChoiceEffect::PenaliseFullReveal(delta) => {
                state.modifiers.full_reveal_penalty += delta;
            }
            ChoiceEffect::AbortMission => {
                state.phase = ExpeditionPhase::Aborted;
                state.outcome = Some(SurveyOutcome::Failed {
                    body_id: state.target_body.clone(),
                    reason: "Mission aborted by player decision".to_string(),
                });
                return Ok(());
            }
            ChoiceEffect::Narrative | ChoiceEffect::GrantAnomalyOutcome(_) => {}
        }

        // Resume surveying after decision.
        state.phase = ExpeditionPhase::Surveying;
        Ok(())
    }
}

// ─── Errors ───────────────────────────────────────────────────────────────────

/// Errors returned by expedition operations.
#[derive(Debug, thiserror::Error)]
pub enum ExpeditionError {
    /// The referenced expedition does not exist.
    #[error("expedition not found")]
    NotFound,
    /// The expedition is not waiting for a player decision.
    #[error("expedition is not awaiting a decision")]
    NotAwaitingDecision,
    /// The submitted choice id is not one of the available choices.
    #[error("unknown choice: {0}")]
    UnknownChoice(String),
}

// ─── Surface expeditions — reaching off-colony deposits (issue #340) ────────
//
// See DESIGN.md §9B. A colony *builds* a surface expedition targeting a hex
// within its reach; once deployed it yields resources every sol from that
// hex's deposit, continuously, until recalled — not a one-off haul like
// `Expedition` or a system-scale survey like `ExpeditionState`. Cost is
// entirely up front; recall returns the expedition, not the outlay.

/// Stable identifier for a surface expedition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SurfaceExpeditionId(pub Uuid);

impl SurfaceExpeditionId {
    /// Create a new random [`SurfaceExpeditionId`].
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SurfaceExpeditionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Base range in hexes a surface expedition can reach over flat (`Plains`,
/// difficulty `1.0`) terrain, before any tech-driven bonus. A balance dial —
/// DESIGN.md §17 flags surface-expedition range/yield numbers as TBD via the
/// harness, so this is a reasonable starting placeholder, not a tuned value.
pub const BASE_SURFACE_EXPEDITION_RANGE_HEXES: u32 = 6;

/// Compute how many hexes a surface expedition can reach from its origin.
///
/// `path_difficulty` is the terrain difficulty ([`crate::map::Terrain::difficulty`])
/// along the route — rougher terrain (a value above `1.0`) shrinks the usable
/// range; an impassable route (`f32::INFINITY`, e.g. crossing `Ocean`) collapses
/// range to `0`. `tech_range_bonus_hexes` is an additive bonus (from researched
/// tech) applied after the terrain scaling, per DESIGN.md §9B ("terrain
/// difficulty on the path plus the colony's unlocked tech together set how
/// far it can reach").
#[must_use]
pub fn surface_expedition_range_hexes(path_difficulty: f32, tech_range_bonus_hexes: f32) -> u32 {
    if !path_difficulty.is_finite() || path_difficulty <= 0.0 {
        return 0;
    }
    let terrain_scalar = 1.0 / path_difficulty.max(1.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let base = (BASE_SURFACE_EXPEDITION_RANGE_HEXES as f32 * terrain_scalar).floor() as u32;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bonus = tech_range_bonus_hexes.max(0.0).floor() as u32;
    base.saturating_add(bonus)
}

/// Base continuous yield (units/sol) a surface expedition extracts at
/// deposit `richness == 1.0`. A balance dial, see
/// [`BASE_SURFACE_EXPEDITION_RANGE_HEXES`]'s doc comment.
pub const SURFACE_EXPEDITION_BASE_YIELD_PER_SOL: f64 = 5.0;

/// Compute a surface expedition's continuous per-sol yield from its target
/// deposit's richness.
///
/// Mirrors the `0.5 + richness * 0.5` deposit-richness ratio
/// `colony::production::compute_deposit_ratio` already uses for a colony's
/// own extraction, so the arithmetic is identical whether a commodity comes
/// from a colony's own hex or a deployed expedition (DESIGN.md §9B: "this
/// keeps the per-sol extraction arithmetic identical to a colony's").
#[must_use]
pub fn surface_expedition_yield_per_sol(richness: f32) -> f64 {
    let ratio = 0.5 + f64::from(richness.clamp(0.0, 1.0)) * 0.5;
    SURFACE_EXPEDITION_BASE_YIELD_PER_SOL * ratio
}

/// Up-front cost (in `structural_ore`) to deploy a surface expedition at
/// zero distance, before the per-hex distance surcharge. Entirely up front —
/// DESIGN.md §9B: "there is no per-sol crew upkeep". Balance dial, see
/// [`BASE_SURFACE_EXPEDITION_RANGE_HEXES`]'s doc comment.
pub const SURFACE_EXPEDITION_BASE_COST_ORE: f64 = 50.0;

/// Additional up-front cost (in `structural_ore`) per hex of distance from
/// the launching colony. Balance dial.
pub const SURFACE_EXPEDITION_COST_PER_HEX_ORE: f64 = 10.0;

/// Compute the up-front `structural_ore` cost to deploy a surface expedition
/// `distance_hexes` from its launching colony.
#[must_use]
pub fn surface_expedition_cost_ore(distance_hexes: u32) -> f64 {
    SURFACE_EXPEDITION_BASE_COST_ORE
        + f64::from(distance_hexes) * SURFACE_EXPEDITION_COST_PER_HEX_ORE
}

/// A constructed surface expedition deployed to a hex within a colony's
/// reach, continuously extracting one commodity until recalled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceExpedition {
    /// Stable identifier for this deployment.
    pub id: SurfaceExpeditionId,
    /// Colony that launched, funds, and receives the yield of this expedition.
    pub colony_id: ColonyId,
    /// Target hex on the colony's planet map.
    pub target_hex: HexCoord,
    /// Commodity this expedition extracts. Contention is per resource, not
    /// per hex (DESIGN.md §9B) — a second expedition may target the same hex
    /// provided it extracts a different commodity.
    pub commodity_id: String,
    /// Deposit richness recorded at deploy time; terrain's effect on yield is
    /// already baked into this figure (DESIGN.md §9B), so extraction never
    /// re-applies a terrain multiplier.
    pub richness: f32,
    /// Sol at which this expedition was deployed.
    pub sol_deployed: u64,
}

impl SurfaceExpedition {
    /// Construct a newly deployed surface expedition.
    #[must_use]
    pub fn new(
        colony_id: ColonyId,
        target_hex: HexCoord,
        commodity_id: impl Into<String>,
        richness: f32,
        sol_deployed: u64,
    ) -> Self {
        Self {
            id: SurfaceExpeditionId::new(),
            colony_id,
            target_hex,
            commodity_id: commodity_id.into(),
            richness,
            sol_deployed,
        }
    }

    /// This expedition's continuous per-sol yield.
    #[must_use]
    pub fn yield_per_sol(&self) -> f64 {
        surface_expedition_yield_per_sol(self.richness)
    }
}

/// In-memory store for all currently deployed surface expeditions.
///
/// Unlike [`ExpeditionRegistry`], entries are removed outright on recall —
/// there is no terminal `Completed`/`Aborted` phase to retain, since a
/// surface expedition's only lifecycle events are deploy and recall.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SurfaceExpeditionRegistry {
    /// All currently deployed expeditions, keyed by their stable id.
    pub expeditions: HashMap<SurfaceExpeditionId, SurfaceExpedition>,
}

impl SurfaceExpeditionRegistry {
    /// Register a newly deployed expedition, returning its id.
    pub fn deploy(&mut self, expedition: SurfaceExpedition) -> SurfaceExpeditionId {
        let id = expedition.id;
        self.expeditions.insert(id, expedition);
        id
    }

    /// Look up an expedition by id.
    #[must_use]
    pub fn get(&self, id: &SurfaceExpeditionId) -> Option<&SurfaceExpedition> {
        self.expeditions.get(id)
    }

    /// Iterate over all deployed expeditions.
    pub fn iter(&self) -> impl Iterator<Item = (&SurfaceExpeditionId, &SurfaceExpedition)> {
        self.expeditions.iter()
    }

    /// Recall a deployed expedition, removing it and returning its final state.
    ///
    /// # Errors
    ///
    /// Returns [`ExpeditionError::NotFound`] if `id` is not currently deployed.
    pub fn recall(
        &mut self,
        id: &SurfaceExpeditionId,
    ) -> Result<SurfaceExpedition, ExpeditionError> {
        self.expeditions.remove(id).ok_or(ExpeditionError::NotFound)
    }

    /// Returns `true` if a currently-deployed expedition already targets
    /// `target_hex` extracting `commodity_id`.
    ///
    /// Contention is per resource, not per hex (DESIGN.md §9B): two
    /// expeditions may share a hex provided they extract different
    /// commodities, so this only blocks an exact hex+commodity match.
    #[must_use]
    pub fn contends(&self, target_hex: HexCoord, commodity_id: &str) -> bool {
        self.expeditions
            .values()
            .any(|e| e.target_hex == target_hex && e.commodity_id == commodity_id)
    }
}

// ─── Surface expedition failure table (issue #340) ──────────────────────────

/// Content-pack entry describing the failure-effect table rolled when a
/// surface expedition launch fails (issue #340).
///
/// Reuses the weighted-outcome mechanism [`AnomalyOutcome`]/
/// [`resolve_anomaly_outcome`] already established for survey expeditions —
/// DESIGN.md §9B: "failure resolves from a content-authored effect table ...
/// reusing the `AnomalyDef`/`AnomalyOutcome` mechanism ... rather than
/// hardcoding failure effects in the kernel."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceExpeditionFailureDef {
    /// Unique content-pack identifier.
    pub id: String,
    /// Probability (0-1) that launching a surface expedition triggers this
    /// failure table at all.
    pub trigger_probability: f32,
    /// Possible failure outcomes, weighted.
    pub outcomes: Vec<SurfaceExpeditionFailureOutcome>,
}

/// One possible effect applied when a surface expedition's launch fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceExpeditionFailureOutcome {
    /// Identifier of this outcome variant.
    pub id: String,
    /// Probability weight relative to other outcomes (normalised at resolution).
    pub weight: f32,
    /// Player-facing description of what happened.
    pub description: String,
    /// Colonists lost from the launching colony's population.
    #[serde(default)]
    pub colonists_lost: u32,
    /// Resources lost from the launching colony's pool (commodity id → quantity).
    #[serde(default)]
    pub resources_lost: HashMap<String, f64>,
}

/// Pick which [`SurfaceExpeditionFailureOutcome`] a failed launch resolves
/// to, given a `[0,1)` roll weighted by each outcome's `weight` — identical
/// selection logic to [`resolve_anomaly_outcome`].
#[must_use]
pub fn resolve_surface_expedition_failure(
    def: &SurfaceExpeditionFailureDef,
    roll: f32,
) -> Option<&SurfaceExpeditionFailureOutcome> {
    let total_weight: f32 = def.outcomes.iter().map(|o| o.weight.max(0.0)).sum();
    if total_weight <= 0.0 {
        return def.outcomes.first();
    }
    let target = roll.clamp(0.0, 0.999_999) * total_weight;
    let mut cumulative = 0.0;
    for outcome in &def.outcomes {
        cumulative += outcome.weight.max(0.0);
        if target < cumulative {
            return Some(outcome);
        }
    }
    def.outcomes.last()
}

// ─── Travel Time ──────────────────────────────────────────────────────────────

/// Compute transit turns from one body to another given a propulsion multiplier.
///
/// `distance_au` is an abstract distance in Astronomical Units (schematic, not real).
/// `propulsion_factor` ≥ 1.0; higher values reduce travel time.
#[must_use]
pub fn travel_time_turns(distance_au: f32, propulsion_factor: f32) -> u32 {
    let factor = propulsion_factor.max(1.0);
    // Base: 1 AU ≈ 4 turns; factor reduces linearly.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let turns = ((distance_au * 4.0) / factor).ceil() as u32;
    turns
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_deposits() -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("iron".to_string(), 500.0);
        m
    }

    fn sample_body() -> BodyId {
        BodyId::new()
    }

    fn sample_colony_id() -> ColonyId {
        Uuid::new_v4()
    }

    // ── Survey outcome probabilities ──────────────────────────────────────────

    #[test]
    fn manned_expedition_low_roll_gives_full_reveal() {
        let body = sample_body();
        let outcome = resolve_survey(
            ExpeditionType::MannedExpedition,
            body.clone(),
            &SurveyModifiers::default(),
            0.01, // well below 0.85
            "Alpha Site",
            sample_deposits(),
        );
        assert!(
            matches!(outcome, SurveyOutcome::FullReveal { .. }),
            "expected FullReveal but got {outcome:?}"
        );
    }

    #[test]
    fn fast_flyby_high_roll_gives_failure() {
        let body = sample_body();
        // full_p=0.10, partial_p=0.40, so roll=0.55 → above both → Failed
        let outcome = resolve_survey(
            ExpeditionType::FastFlybyProbe,
            body.clone(),
            &SurveyModifiers::default(),
            0.55,
            "Beta Site",
            sample_deposits(),
        );
        assert!(
            matches!(outcome, SurveyOutcome::Failed { .. }),
            "expected Failed but got {outcome:?}"
        );
    }

    #[test]
    fn fast_flyby_mid_roll_gives_partial_reveal() {
        let body = sample_body();
        // full_p=0.10, partial_p=0.40, roll=0.30 → partial
        let outcome = resolve_survey(
            ExpeditionType::FastFlybyProbe,
            body.clone(),
            &SurveyModifiers::default(),
            0.30,
            "Gamma Site",
            sample_deposits(),
        );
        assert!(
            matches!(outcome, SurveyOutcome::PartialReveal { .. }),
            "expected PartialReveal but got {outcome:?}"
        );
    }

    #[test]
    fn full_reveal_bonus_shifts_boundary() {
        let body = sample_body();
        // Without bonus, roll=0.80 on OrbitalSurvey (base full=0.35) → partial.
        let without_bonus = resolve_survey(
            ExpeditionType::OrbitalSurvey,
            body.clone(),
            &SurveyModifiers::default(),
            0.80,
            "S",
            HashMap::new(),
        );
        assert!(matches!(without_bonus, SurveyOutcome::Failed { .. }));

        // With full_reveal_bonus=0.50, full_p = 0.35+0.50 = 0.85 → roll=0.80 → full.
        let boosted = SurveyModifiers {
            full_reveal_bonus: 0.50,
            ..Default::default()
        };
        let with_bonus = resolve_survey(
            ExpeditionType::OrbitalSurvey,
            body,
            &boosted,
            0.80,
            "S",
            HashMap::new(),
        );
        assert!(matches!(with_bonus, SurveyOutcome::FullReveal { .. }));
    }

    // ── Anomaly trigger ───────────────────────────────────────────────────────

    fn sample_anomaly() -> AnomalyDef {
        AnomalyDef {
            id: "ancient_ruins".to_string(),
            name: "Ancient Ruins".to_string(),
            trigger_probability: 0.25,
            eligible_expedition_types: vec![
                ExpeditionType::Lander,
                ExpeditionType::MannedExpedition,
            ],
            description: "Unusual geometric formations detected.".to_string(),
            outcomes: vec![AnomalyOutcome {
                id: "tech_find".to_string(),
                weight: 1.0,
                description: "Ancient data cores recovered.".to_string(),
                research_bonus: 50.0,
                resource_reward: HashMap::new(),
                unlocks_tech: Some("xenotech_tier1".to_string()),
            }],
        }
    }

    #[test]
    fn anomaly_triggers_when_roll_below_probability() {
        let anomaly = sample_anomaly();
        assert!(check_anomaly_trigger(
            &anomaly,
            ExpeditionType::Lander,
            0.10 // 0.10 < 0.25 → triggers
        ));
    }

    #[test]
    fn anomaly_does_not_trigger_when_roll_above_probability() {
        let anomaly = sample_anomaly();
        assert!(!check_anomaly_trigger(
            &anomaly,
            ExpeditionType::Lander,
            0.30 // 0.30 >= 0.25 → no trigger
        ));
    }

    #[test]
    fn anomaly_does_not_trigger_for_ineligible_expedition_type() {
        let anomaly = sample_anomaly();
        // FastFlybyProbe not in eligible list
        assert!(!check_anomaly_trigger(
            &anomaly,
            ExpeditionType::FastFlybyProbe,
            0.01 // would trigger if eligible
        ));
    }

    // ── Decision resolution ───────────────────────────────────────────────────

    fn make_event_with_boost() -> MidMissionEvent {
        MidMissionEvent {
            id: Uuid::new_v4(),
            title: "Sensor Array Overload".to_string(),
            description: "Recalibrate sensors for better data?".to_string(),
            tier: Tier::Blocking,
            choices: vec![
                MissionChoice {
                    id: "recalibrate".to_string(),
                    label: "Recalibrate (+20 % full reveal)".to_string(),
                    effect: ChoiceEffect::BoostFullReveal(0.20),
                },
                MissionChoice {
                    id: "abort".to_string(),
                    label: "Abort mission".to_string(),
                    effect: ChoiceEffect::AbortMission,
                },
            ],
        }
    }

    #[test]
    fn decision_boost_applies_modifier() {
        let body = sample_body();
        let mut registry = ExpeditionRegistry::default();
        let mut state = ExpeditionState::new(ExpeditionType::Lander, body, sample_colony_id());
        state.phase = ExpeditionPhase::AwaitingDecision;
        state.pending_event = Some(make_event_with_boost());
        let id = registry.launch(state);

        registry.resolve_decision(&id, "recalibrate").unwrap();

        let updated = registry.get(&id).unwrap();
        assert_eq!(updated.phase, ExpeditionPhase::Surveying);
        assert!((updated.modifiers.full_reveal_bonus - 0.20).abs() < 1e-6);
    }

    #[test]
    fn decision_abort_sets_failed_outcome() {
        let body = sample_body();
        let mut registry = ExpeditionRegistry::default();
        let mut state = ExpeditionState::new(ExpeditionType::Lander, body, sample_colony_id());
        state.phase = ExpeditionPhase::AwaitingDecision;
        state.pending_event = Some(make_event_with_boost());
        let id = registry.launch(state);

        registry.resolve_decision(&id, "abort").unwrap();

        let updated = registry.get(&id).unwrap();
        assert_eq!(updated.phase, ExpeditionPhase::Aborted);
        assert!(matches!(
            updated.outcome,
            Some(SurveyOutcome::Failed { .. })
        ));
    }

    #[test]
    fn decision_unknown_choice_returns_error() {
        let body = sample_body();
        let mut registry = ExpeditionRegistry::default();
        let mut state = ExpeditionState::new(ExpeditionType::Lander, body, sample_colony_id());
        state.phase = ExpeditionPhase::AwaitingDecision;
        state.pending_event = Some(make_event_with_boost());
        let id = registry.launch(state);

        let err = registry.resolve_decision(&id, "nonexistent");
        assert!(matches!(err, Err(ExpeditionError::UnknownChoice(_))));
    }

    #[test]
    fn decision_on_non_awaiting_expedition_returns_error() {
        let body = sample_body();
        let mut registry = ExpeditionRegistry::default();
        let state = ExpeditionState::new(ExpeditionType::Lander, body, sample_colony_id());
        let id = registry.launch(state); // phase = InTransit

        let err = registry.resolve_decision(&id, "recalibrate");
        assert!(matches!(err, Err(ExpeditionError::NotAwaitingDecision)));
    }

    // ── Travel time ───────────────────────────────────────────────────────────

    #[test]
    fn travel_time_scales_with_distance() {
        let slow = travel_time_turns(1.0, 1.0); // 4 turns
        let far = travel_time_turns(2.0, 1.0); // 8 turns
        assert_eq!(slow, 4);
        assert_eq!(far, 8);
    }

    #[test]
    fn travel_time_reduced_by_propulsion() {
        let slow = travel_time_turns(4.0, 1.0); // 16 turns
        let fast = travel_time_turns(4.0, 4.0); // 4 turns
        assert_eq!(slow, 16);
        assert_eq!(fast, 4);
    }

    // ── Surface expedition range/yield ────────────────────────────────────────

    #[test]
    fn surface_expedition_range_shrinks_with_terrain_difficulty() {
        let flat = surface_expedition_range_hexes(1.0, 0.0);
        let hills = surface_expedition_range_hexes(1.8, 0.0);
        assert_eq!(flat, BASE_SURFACE_EXPEDITION_RANGE_HEXES);
        assert!(hills < flat, "rougher terrain should shrink range");
    }

    #[test]
    fn surface_expedition_range_is_zero_over_impassable_terrain() {
        assert_eq!(surface_expedition_range_hexes(f32::INFINITY, 0.0), 0);
    }

    #[test]
    fn surface_expedition_range_tech_bonus_is_additive() {
        let base = surface_expedition_range_hexes(1.0, 0.0);
        let boosted = surface_expedition_range_hexes(1.0, 3.0);
        assert_eq!(boosted, base + 3);
    }

    #[test]
    fn surface_expedition_yield_scales_with_richness() {
        let low = surface_expedition_yield_per_sol(0.1);
        let high = surface_expedition_yield_per_sol(1.0);
        assert!(low > 0.0);
        assert!(high > low);
        assert!((high - SURFACE_EXPEDITION_BASE_YIELD_PER_SOL).abs() < 1e-9);
    }

    // ── Surface expedition registry ───────────────────────────────────────────

    #[test]
    fn surface_expedition_deploy_and_recall_round_trip() {
        let mut registry = SurfaceExpeditionRegistry::default();
        let colony = sample_colony_id();
        let hex = HexCoord::new(2, 3);
        let expedition = SurfaceExpedition::new(colony, hex, "structural_ore", 0.8, 10);
        let id = registry.deploy(expedition);

        assert!(registry.get(&id).is_some());
        let recalled = registry.recall(&id).unwrap();
        assert_eq!(recalled.colony_id, colony);
        assert!(registry.get(&id).is_none());
    }

    #[test]
    fn surface_expedition_recall_unknown_id_errors() {
        let mut registry = SurfaceExpeditionRegistry::default();
        let err = registry.recall(&SurfaceExpeditionId::new());
        assert!(matches!(err, Err(ExpeditionError::NotFound)));
    }

    #[test]
    fn surface_expedition_contention_is_per_resource_not_per_hex() {
        let mut registry = SurfaceExpeditionRegistry::default();
        let hex = HexCoord::new(5, 5);
        registry.deploy(SurfaceExpedition::new(
            sample_colony_id(),
            hex,
            "structural_ore",
            0.5,
            0,
        ));

        assert!(registry.contends(hex, "structural_ore"));
        assert!(
            !registry.contends(hex, "conductive_ore"),
            "same hex, different resource, must not contend"
        );
        assert!(!registry.contends(HexCoord::new(6, 6), "structural_ore"));
    }

    // ── Surface expedition failure table ──────────────────────────────────────

    fn sample_failure_def() -> SurfaceExpeditionFailureDef {
        SurfaceExpeditionFailureDef {
            id: "surface_expedition_mishap".to_string(),
            trigger_probability: 0.2,
            outcomes: vec![
                SurfaceExpeditionFailureOutcome {
                    id: "crew_lost".to_string(),
                    weight: 1.0,
                    description: "The crew was lost.".to_string(),
                    colonists_lost: 2,
                    resources_lost: HashMap::new(),
                },
                SurfaceExpeditionFailureOutcome {
                    id: "cargo_lost".to_string(),
                    weight: 1.0,
                    description: "Equipment was destroyed.".to_string(),
                    colonists_lost: 0,
                    resources_lost: {
                        let mut m = HashMap::new();
                        m.insert("structural_ore".to_string(), 25.0);
                        m
                    },
                },
            ],
        }
    }

    #[test]
    fn surface_expedition_failure_resolves_by_weight() {
        let def = sample_failure_def();
        let low_roll = resolve_surface_expedition_failure(&def, 0.1).unwrap();
        assert_eq!(low_roll.id, "crew_lost");

        let high_roll = resolve_surface_expedition_failure(&def, 0.9).unwrap();
        assert_eq!(high_roll.id, "cargo_lost");
    }

    // ── Registry serde round-trip ─────────────────────────────────────────────

    #[test]
    fn expedition_registry_serde_round_trip() {
        let body = sample_body();
        let mut registry = ExpeditionRegistry::default();
        registry.launch(ExpeditionState::new(
            ExpeditionType::OrbitalSurvey,
            body,
            sample_colony_id(),
        ));
        let json = serde_json::to_string(&registry).unwrap();
        let back: ExpeditionRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.expeditions.len(), 1);
    }
}
