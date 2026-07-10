//! Expeditions — textured exploration with events, encounters, and mid-mission decisions.
//!
//! Covers DESIGN.md §9: schematic system node maps, expedition types, survey outcomes,
//! anomalies, and mid-mission interrupts that reuse the interrupt + predicate system.
//!
//! # Architecture
//!
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

use crate::interrupt::Tier;
use crate::system::BodyId;

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
pub fn resolve_survey(
    expedition_type: ExpeditionType,
    body_id: BodyId,
    modifiers: &SurveyModifiers,
    roll: f32, // caller supplies a [0,1) float from the RNG
    site_name: impl Into<String>,
    deposits: HashMap<String, f64>,
) -> SurveyOutcome {
    let full_p = (expedition_type.base_full_reveal_prob()
        + modifiers.full_reveal_bonus
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
pub fn check_anomaly_trigger(anomaly: &AnomalyDef, expedition_type: ExpeditionType, roll: f32) -> bool {
    anomaly.eligible_expedition_types.contains(&expedition_type) && roll < anomaly.trigger_probability
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

/// Full live state of one expedition mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpeditionState {
    /// Stable identifier.
    pub id: ExpeditionId,
    /// Mission profile.
    pub expedition_type: ExpeditionType,
    /// Target celestial body.
    pub target_body: BodyId,
    /// Current lifecycle phase.
    pub phase: ExpeditionPhase,
    /// Turns remaining until transit / survey completes.
    pub turns_remaining: u32,
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
    pub fn new(expedition_type: ExpeditionType, target_body: BodyId) -> Self {
        let transit = 2u32; // fixed transit leg
        let survey = expedition_type.base_duration_turns();
        Self {
            id: ExpeditionId::new(),
            expedition_type,
            target_body,
            phase: ExpeditionPhase::InTransit,
            turns_remaining: transit + survey,
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
            ChoiceEffect::Narrative => {}
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

// ─── Travel Time ──────────────────────────────────────────────────────────────

/// Compute transit turns from one body to another given a propulsion multiplier.
///
/// `distance_au` is an abstract distance in Astronomical Units (schematic, not real).
/// `propulsion_factor` ≥ 1.0; higher values reduce travel time.
#[must_use]
pub fn travel_time_turns(distance_au: f32, propulsion_factor: f32) -> u32 {
    let factor = propulsion_factor.max(1.0);
    // Base: 1 AU ≈ 4 turns; factor reduces linearly.
    ((distance_au * 4.0) / factor).ceil() as u32
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
        let mut state = ExpeditionState::new(ExpeditionType::Lander, body);
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
        let mut state = ExpeditionState::new(ExpeditionType::Lander, body);
        state.phase = ExpeditionPhase::AwaitingDecision;
        state.pending_event = Some(make_event_with_boost());
        let id = registry.launch(state);

        registry.resolve_decision(&id, "abort").unwrap();

        let updated = registry.get(&id).unwrap();
        assert_eq!(updated.phase, ExpeditionPhase::Aborted);
        assert!(matches!(updated.outcome, Some(SurveyOutcome::Failed { .. })));
    }

    #[test]
    fn decision_unknown_choice_returns_error() {
        let body = sample_body();
        let mut registry = ExpeditionRegistry::default();
        let mut state = ExpeditionState::new(ExpeditionType::Lander, body);
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
        let state = ExpeditionState::new(ExpeditionType::Lander, body);
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

    // ── Registry serde round-trip ─────────────────────────────────────────────

    #[test]
    fn expedition_registry_serde_round_trip() {
        let body = sample_body();
        let mut registry = ExpeditionRegistry::default();
        registry.launch(ExpeditionState::new(
            ExpeditionType::OrbitalSurvey,
            body,
        ));
        let json = serde_json::to_string(&registry).unwrap();
        let back: ExpeditionRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.expeditions.len(), 1);
    }
}
