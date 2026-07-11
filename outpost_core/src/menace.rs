//! Existential clock (menace) — optional campaign pressure layer (§10A).
//!
//! **Architecture discipline:** The base game must stand alone without a menace.
//! The menace is a *modifier layer* — nothing in the core simulation loop depends
//! on its presence. It is toggled on (`campaign` mode) or off (`sandbox` mode).
//!
//! A menace is authored data following the schema:
//! ```text
//! { id, phases: [{ trigger_time, telegraph, effects[] }], final_semantics }
//! ```
//! Phases emit [`ModifierDescriptor`]s and optional hazard injections. Telegraph
//! strings wire to the interrupt system to warn the player. The failure mode is
//! *degrading collapse* — survival becomes untenable; there is no instant lose screen.

use serde::{Deserialize, Serialize};

use crate::modifier::ModifierDescriptor;

// ─── MenaceKind ──────────────────────────────────────────────────────────────

/// Categorises the nature of the existential threat driving the menace clock.
///
/// Used by [`Event::MenaceCritical`] and [`Event::GameOver`] so callers can
/// distinguish which menace line fired without inspecting the full definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenaceKind {
    /// Environmental systems are failing (atmosphere, ecology, climate).
    EnvironmentalCollapse,
    /// A rival faction is overwhelming the colony network.
    RivalFaction,
    /// Critical resource stockpiles are approaching zero.
    ResourceDepletion,
}

// ─── MenacePhase ─────────────────────────────────────────────────────────────

/// One escalation phase of a menace.
///
/// Phases are evaluated in order; a phase activates when `trigger_time` is
/// reached (measured in strategic months since the menace was activated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenacePhase {
    /// Strategic months from menace activation when this phase triggers.
    pub trigger_time: u64,
    /// Human-readable telegraph message wired to the interrupt system.
    ///
    /// Displayed one phase early as a predictive warning.
    pub telegraph: String,
    /// Modifier descriptors emitted when this phase becomes active.
    ///
    /// These are injected into the modifier pipeline and degrade the
    /// player's resource situation without causing an instant game-over.
    pub effects: Vec<ModifierDescriptor>,
    /// Optional hazard tag injected into the event queue when this phase activates.
    ///
    /// A plain string key referencing a hazard record in the content pack;
    /// the turn processor resolves it against the event system.
    pub hazard_injection: Option<String>,
}

// ─── FinalSemantics ──────────────────────────────────────────────────────────

/// What happens when the menace reaches its final phase.
///
/// The collapse is emergent — the final phase makes survival untenable,
/// but a scripted game-over screen is never shown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinalSemantics {
    /// All production rates approach zero; population slowly declines.
    ProductionCollapse,
    /// Stability decay accelerates to unrecoverable levels.
    StabilitySpiral,
    /// Population exodus becomes unstoppable.
    PopulationExodus,
    /// A custom authored semantic with a content-pack id.
    Custom(String),
}

// ─── MenaceDefinition ────────────────────────────────────────────────────────

/// A complete authored menace definition loaded from a content pack.
///
/// One clock mechanism reads this; multiple menace types may be authored using
/// the same schema (economic squeeze vs physical hazard, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenaceDefinition {
    /// Unique content-pack identifier for this menace.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Ordered escalation phases (sorted by `trigger_time` ascending).
    pub phases: Vec<MenacePhase>,
    /// What the final phase does when reached.
    pub final_semantics: FinalSemantics,
}

impl MenaceDefinition {
    /// Return the phase index active at `elapsed_months`, or `None` if no phase
    /// has triggered yet.
    #[must_use]
    pub fn active_phase_index(&self, elapsed_months: u64) -> Option<usize> {
        // The active phase is the last one whose trigger_time ≤ elapsed_months.
        self.phases
            .iter()
            .enumerate()
            .filter(|(_, p)| p.trigger_time <= elapsed_months)
            .map(|(i, _)| i)
            .next_back()
    }

    /// Return the *next* phase index that has not yet triggered (for telegraph display).
    #[must_use]
    pub fn upcoming_phase_index(&self, elapsed_months: u64) -> Option<usize> {
        self.phases
            .iter()
            .enumerate()
            .find(|(_, p)| p.trigger_time > elapsed_months)
            .map(|(i, _)| i)
    }

    /// Whether the menace has reached its final phase.
    #[must_use]
    pub fn is_final_phase_active(&self, elapsed_months: u64) -> bool {
        if self.phases.is_empty() {
            return false;
        }
        let last_trigger = self.phases.last().map_or(u64::MAX, |p| p.trigger_time);
        elapsed_months >= last_trigger
    }
}

// ─── MenaceState ─────────────────────────────────────────────────────────────

/// Runtime state of the active menace clock.
///
/// Stored in [`crate::turn::GameState`]; `None` when sandbox mode is active.
/// Combines the authored phase-based definition with a continuous pressure level
/// that escalates each strategic month and fires critical/game-over signals when
/// thresholds are crossed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenaceState {
    /// The authored menace driving this clock.
    pub definition: MenaceDefinition,
    /// Strategic months elapsed since the menace was activated.
    pub elapsed_months: u64,
    /// Index of the phase that was active at the previous tick (for transition detection).
    pub last_phase_index: Option<usize>,
    /// Whether the menace is currently active (can be paused for sandbox revert).
    pub active: bool,

    // ── Continuous-level pressure layer ──────────────────────────────────────
    /// Broad category of this menace (for event emission).
    pub kind: MenaceKind,
    /// Current continuous pressure level; starts at `0.0` and climbs each tick.
    pub level: f32,
    /// Amount added to `level` per strategic month.
    pub escalation_rate: f32,
    /// When `level` reaches this value a `MenaceCritical` event fires and the
    /// countdown begins.
    pub critical_threshold: f32,
    /// Remaining strategic months before game-over once the clock goes critical.
    ///
    /// `None` until `level >= critical_threshold`; counts down to zero, at which
    /// point `GameOver` fires.  Player mitigation resets or pauses this.
    pub countdown: Option<u32>,
}

impl MenaceState {
    /// Construct a new menace state from a definition (starts at elapsed = 0).
    #[must_use]
    pub fn new(definition: MenaceDefinition) -> Self {
        Self {
            definition,
            elapsed_months: 0,
            last_phase_index: None,
            active: true,
            kind: MenaceKind::EnvironmentalCollapse,
            level: 0.0,
            escalation_rate: 1.0,
            critical_threshold: 10.0,
            countdown: None,
        }
    }

    /// Construct a new menace state with explicit continuous-level parameters.
    #[must_use]
    pub fn new_with_level(
        definition: MenaceDefinition,
        kind: MenaceKind,
        escalation_rate: f32,
        critical_threshold: f32,
    ) -> Self {
        Self {
            definition,
            elapsed_months: 0,
            last_phase_index: None,
            active: true,
            kind,
            level: 0.0,
            escalation_rate,
            critical_threshold,
            countdown: None,
        }
    }

    /// Reduce the pressure level by `amount`.
    ///
    /// Clamps to `[0.0, ∞)`.  If level drops below `critical_threshold` the
    /// countdown is cancelled (player bought themselves more time).
    pub fn mitigate(&mut self, amount: f32) {
        self.level = (self.level - amount).max(0.0);
        if self.level < self.critical_threshold {
            self.countdown = None;
        }
    }

    /// Default countdown length (strategic months) once critical threshold is crossed.
    pub const DEFAULT_COUNTDOWN: u32 = 12;

    /// Advance the menace by one strategic month.
    ///
    /// Returns the [`MenaceTickOutcome`] describing any phase transitions,
    /// telegraphs, and continuous-level signals.
    pub fn tick(&mut self) -> MenaceTickOutcome {
        if !self.active {
            return MenaceTickOutcome::default();
        }

        self.elapsed_months += 1;

        // ── Phase-based escalation ────────────────────────────────────────────
        let new_phase = self.definition.active_phase_index(self.elapsed_months);

        let phase_entered = match (self.last_phase_index, new_phase) {
            (None, Some(i)) => Some(i),
            (Some(old), Some(new)) if new > old => Some(new),
            _ => None,
        };

        let telegraph = self
            .definition
            .upcoming_phase_index(self.elapsed_months)
            .map(|i| self.definition.phases[i].telegraph.clone());

        let effects: Vec<ModifierDescriptor> = new_phase
            .map(|i| self.definition.phases[i].effects.clone())
            .unwrap_or_default();

        let hazard_injection: Option<String> =
            phase_entered.and_then(|i| self.definition.phases[i].hazard_injection.clone());

        let final_phase_reached = self.definition.is_final_phase_active(self.elapsed_months)
            && phase_entered.is_some_and(|i| i + 1 == self.definition.phases.len());

        self.last_phase_index = new_phase;

        // ── Continuous-level escalation ───────────────────────────────────────
        let was_below_critical = self.level < self.critical_threshold;
        self.level += self.escalation_rate;

        let just_went_critical =
            was_below_critical && self.level >= self.critical_threshold;

        if just_went_critical && self.countdown.is_none() {
            self.countdown = Some(Self::DEFAULT_COUNTDOWN);
        }

        let game_over = if let Some(ref mut cd) = self.countdown {
            if *cd == 0 {
                true
            } else {
                *cd -= 1;
                false
            }
        } else {
            false
        };

        if game_over {
            self.countdown = Some(0);
        }

        MenaceTickOutcome {
            phase_entered,
            active_effects: effects,
            telegraph,
            hazard_injection,
            final_phase_reached,
            just_went_critical,
            game_over,
            kind: self.kind,
        }
    }
}

/// Outcome of advancing the menace clock by one strategic month.
#[derive(Debug, Clone)]
pub struct MenaceTickOutcome {
    /// Index of the phase just entered, if a transition occurred this tick.
    pub phase_entered: Option<usize>,
    /// All modifier effects from the currently active phase.
    pub active_effects: Vec<ModifierDescriptor>,
    /// Telegraph message for the *next* upcoming phase (for the interrupt digest).
    pub telegraph: Option<String>,
    /// Hazard content-pack key injected on phase entry, if any.
    pub hazard_injection: Option<String>,
    /// Whether the final phase was just reached this tick.
    pub final_phase_reached: bool,
    /// Whether the continuous level just crossed `critical_threshold` this tick.
    pub just_went_critical: bool,
    /// Whether the countdown reached zero this tick, triggering game-over.
    pub game_over: bool,
    /// Kind of menace (copied for convenience so callers do not need the full state).
    pub kind: MenaceKind,
}

impl Default for MenaceTickOutcome {
    fn default() -> Self {
        Self {
            phase_entered: None,
            active_effects: Vec::new(),
            telegraph: None,
            hazard_injection: None,
            final_phase_reached: false,
            just_went_critical: false,
            game_over: false,
            kind: MenaceKind::EnvironmentalCollapse,
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modifier::{ModifiableQuantity, ModifierDescriptor};

    fn sample_menace() -> MenaceDefinition {
        MenaceDefinition {
            id: "destabilising_star".into(),
            name: "Destabilising Star".into(),
            phases: vec![
                MenacePhase {
                    trigger_time: 5,
                    telegraph: "Solar flux increasing — prepare cooling systems.".into(),
                    effects: vec![ModifierDescriptor::new(
                        ModifiableQuantity::ProductionRate("*".into()),
                        "menace",
                        -0.10,
                    )],
                    hazard_injection: None,
                },
                MenacePhase {
                    trigger_time: 15,
                    telegraph: "Radiation storm imminent — evacuate exposed habitats.".into(),
                    effects: vec![
                        ModifierDescriptor::new(
                            ModifiableQuantity::ProductionRate("*".into()),
                            "menace",
                            -0.25,
                        ),
                        ModifierDescriptor::new(
                            ModifiableQuantity::PopulationGrowth,
                            "menace",
                            -0.30,
                        ),
                    ],
                    hazard_injection: Some("radiation_storm".into()),
                },
                MenacePhase {
                    trigger_time: 30,
                    telegraph: "Star entering terminal phase — launch the expedition NOW.".into(),
                    effects: vec![
                        ModifierDescriptor::new(
                            ModifiableQuantity::ProductionRate("*".into()),
                            "menace",
                            -0.50,
                        ),
                        ModifierDescriptor::new(ModifiableQuantity::StabilityRate, "menace", 2.00),
                    ],
                    hazard_injection: Some("stellar_collapse_wave".into()),
                },
            ],
            final_semantics: FinalSemantics::StabilitySpiral,
        }
    }

    #[test]
    fn no_phase_before_first_trigger() {
        let menace = sample_menace();
        assert_eq!(menace.active_phase_index(0), None);
        assert_eq!(menace.active_phase_index(4), None);
    }

    #[test]
    fn phase_zero_activates_at_trigger_time() {
        let menace = sample_menace();
        assert_eq!(menace.active_phase_index(5), Some(0));
        assert_eq!(menace.active_phase_index(14), Some(0));
    }

    #[test]
    fn phase_transitions_correctly() {
        let menace = sample_menace();
        assert_eq!(menace.active_phase_index(15), Some(1));
        assert_eq!(menace.active_phase_index(30), Some(2));
    }

    #[test]
    fn final_phase_detected() {
        let menace = sample_menace();
        assert!(!menace.is_final_phase_active(29));
        assert!(menace.is_final_phase_active(30));
    }

    #[test]
    fn tick_emits_phase_entered_on_transition() {
        let mut state = MenaceState::new(sample_menace());
        // Advance to just before phase 0
        for _ in 0..4 {
            let outcome = state.tick();
            assert!(outcome.phase_entered.is_none());
        }
        // Tick 5 → phase 0 triggers
        let outcome = state.tick();
        assert_eq!(outcome.phase_entered, Some(0));
        assert!(!outcome.active_effects.is_empty());
    }

    #[test]
    fn tick_provides_telegraph_for_next_phase() {
        let mut state = MenaceState::new(sample_menace());
        // After phase 0 activates (month 5), there should be a telegraph for phase 1
        for _ in 0..5 {
            state.tick();
        }
        let outcome = state.tick(); // month 6
        assert!(outcome.telegraph.is_some());
        assert!(outcome
            .telegraph
            .unwrap()
            .contains("Radiation storm imminent"));
    }

    #[test]
    fn final_phase_reached_flag_set() {
        let mut state = MenaceState::new(sample_menace());
        let mut saw_final = false;
        for _ in 0..35 {
            let outcome = state.tick();
            if outcome.final_phase_reached {
                saw_final = true;
            }
        }
        assert!(saw_final, "final_phase_reached should have been set");
    }

    #[test]
    fn inactive_menace_does_not_advance() {
        let mut state = MenaceState::new(sample_menace());
        state.active = false;
        let outcome = state.tick();
        assert!(outcome.phase_entered.is_none());
        assert!(outcome.active_effects.is_empty());
        assert_eq!(state.elapsed_months, 0); // no advance
    }

    #[test]
    fn hazard_injection_attached_to_phase_entry() {
        let mut state = MenaceState::new(sample_menace());
        // Phase 1 at month 15 has hazard injection
        for _ in 0..15 {
            state.tick();
        }
        // Last tick was month 15, which entered phase 1
        // Let's re-check: after 14 ticks we're at month 14 (phase 0)
        // tick 15 → phase 1 transition
        let mut state2 = MenaceState::new(sample_menace());
        let mut injection = None;
        for _ in 0..15 {
            let out = state2.tick();
            if out.hazard_injection.is_some() {
                injection = out.hazard_injection;
            }
        }
        assert_eq!(injection, Some("radiation_storm".into()));
    }

    // ── Continuous-level pressure tests ──────────────────────────────────────

    fn level_menace() -> MenaceState {
        // escalation_rate = 5.0, critical_threshold = 10.0 → critical at tick 2
        MenaceState::new_with_level(
            sample_menace(),
            MenaceKind::EnvironmentalCollapse,
            5.0,
            10.0,
        )
    }

    #[test]
    fn level_escalates_each_tick() {
        let mut state = level_menace();
        assert_eq!(state.level, 0.0);
        state.tick();
        assert!((state.level - 5.0).abs() < f32::EPSILON);
        state.tick();
        assert!((state.level - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn critical_fires_when_threshold_crossed() {
        let mut state = level_menace();
        // Tick 1: level = 5.0, below threshold
        let out1 = state.tick();
        assert!(!out1.just_went_critical);
        // Tick 2: level = 10.0, exactly at threshold
        let out2 = state.tick();
        assert!(out2.just_went_critical);
        assert!(state.countdown.is_some());
    }

    #[test]
    fn countdown_decrements_each_tick() {
        let mut state = level_menace();
        // Advance to critical.
        state.tick();
        state.tick(); // now critical, countdown = DEFAULT_COUNTDOWN
        let initial = state.countdown.unwrap();
        state.tick();
        assert_eq!(state.countdown.unwrap(), initial - 1);
    }

    #[test]
    fn game_over_fires_when_countdown_reaches_zero() {
        let mut state = level_menace();
        state.tick(); // level 5
        state.tick(); // level 10, critical — countdown set to DEFAULT
        // Drain countdown.
        let mut saw_game_over = false;
        for _ in 0..=MenaceState::DEFAULT_COUNTDOWN {
            let out = state.tick();
            if out.game_over {
                saw_game_over = true;
                break;
            }
        }
        assert!(saw_game_over, "GameOver should fire when countdown reaches zero");
    }

    #[test]
    fn mitigate_reduces_level() {
        let mut state = level_menace();
        state.tick(); // level = 5.0
        state.mitigate(3.0);
        assert!((state.level - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn mitigate_cancels_countdown_when_drops_below_threshold() {
        let mut state = level_menace();
        state.tick();
        state.tick(); // critical, countdown started
        assert!(state.countdown.is_some());
        // Reduce level below threshold.
        state.mitigate(5.0); // level = 5.0 < 10.0 threshold
        assert!(state.countdown.is_none(), "countdown should clear when level drops below critical");
    }

    #[test]
    fn mitigate_clamps_level_at_zero() {
        let mut state = level_menace();
        state.tick(); // level = 5.0
        state.mitigate(100.0);
        assert_eq!(state.level, 0.0);
    }

    #[test]
    fn kind_preserved_in_outcome() {
        let mut state = MenaceState::new_with_level(
            sample_menace(),
            MenaceKind::RivalFaction,
            5.0,
            10.0,
        );
        let out = state.tick();
        assert_eq!(out.kind, MenaceKind::RivalFaction);
    }
}
