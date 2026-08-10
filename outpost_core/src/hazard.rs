//! Environmental hazard event system — §10.
//!
//! Dust storms, seismic events, meteor impacts, equipment failures, disease
//! outbreaks, and radiation leaks are the primary always-on threat layer.
//! Hazard rolls happen each colony-sol inside `TurnProcessor::run_colony_sol_pipeline`.
//!
//! All configuration is data-driven: callers load [`HazardConfig`] from YAML
//! and pass it to the engine; nothing is hardcoded here.

use serde::{Deserialize, Serialize};

// ─── Kind ────────────────────────────────────────────────────────────────────

/// All environmental hazard categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardKind {
    /// Reduced visibility and solar-power loss; may bury equipment.
    DustStorm,
    /// Ground tremors that damage structures and injure workers.
    SeismicEvent,
    /// High-velocity impactor; severe localised damage.
    MeteorImpact,
    /// Critical machinery breaks down; production halted.
    EquipmentFailure,
    /// Pathogen spreads through the colony; population damage.
    DiseaseOutbreak,
    /// Radiation leak from reactors or solar events; population damage.
    RadiationLeak,
}

impl HazardKind {
    /// All six hazard kinds, in declaration order.
    pub const ALL: [HazardKind; 6] = [
        HazardKind::DustStorm,
        HazardKind::SeismicEvent,
        HazardKind::MeteorImpact,
        HazardKind::EquipmentFailure,
        HazardKind::DiseaseOutbreak,
        HazardKind::RadiationLeak,
    ];
}

// ─── Per-kind tuning ─────────────────────────────────────────────────────────

/// Tunable parameters for a single hazard kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HazardKindConfig {
    /// Base probability that this hazard fires on any given colony-sol.
    ///
    /// The roll is `rng_float < base_probability × terrain_modifier`.
    /// Must be in `[0.0, 1.0]`.
    pub base_probability: f32,

    /// Minimum severity on `[0.0, 1.0]` when the hazard fires.
    pub severity_min: f32,

    /// Maximum severity on `[0.0, 1.0]` when the hazard fires.
    pub severity_max: f32,

    /// Fraction of colony stability lost per unit of severity.
    ///
    /// `stability_hit = severity × stability_damage_per_severity`
    pub stability_damage_per_severity: f32,

    /// Fraction of a randomly selected commodity pool drained per unit of severity.
    ///
    /// `commodity_loss = pool_amount × severity × commodity_loss_per_severity`
    pub commodity_loss_per_severity: f32,

    /// Fraction of population lost per unit of severity.
    ///
    /// `pop_loss = population × severity × population_damage_per_severity`
    pub population_damage_per_severity: f32,
}

impl HazardKindConfig {
    /// Sample a severity in `[severity_min, severity_max]` using a `[0, 1)` float.
    #[must_use]
    pub fn sample_severity(&self, unit: f32) -> f32 {
        let lo = self.severity_min.clamp(0.0, 1.0);
        let hi = self.severity_max.clamp(lo, 1.0);
        lo + unit * (hi - lo)
    }
}

// ─── Top-level config ─────────────────────────────────────────────────────────

/// System-wide hazard configuration.
///
/// Loaded from `hazards.yaml` in the content pack and injected into
/// [`TurnProcessor`] via `GameState`.  Not stored in `outpost_core`
/// itself — the engine is I/O-free.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HazardConfig {
    /// Per-kind tuning entries.
    pub kinds: Vec<HazardEntry>,
}

/// One entry in the hazard YAML list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HazardEntry {
    /// Which hazard kind this entry configures.
    pub kind: HazardKind,
    /// Tunable parameters for this kind.
    #[serde(flatten)]
    pub config: HazardKindConfig,
    /// Optional terrain-modifier map: `terrain_id → probability_multiplier`.
    ///
    /// When a colony's terrain matches a key in this map, the base probability
    /// is multiplied by the associated value.  Defaults to `1.0` if the
    /// terrain is not listed.
    #[serde(default)]
    pub terrain_modifiers: std::collections::HashMap<String, f32>,
}

impl HazardConfig {
    /// Find the config entry for `kind`, if present.
    #[must_use]
    pub fn entry_for(&self, kind: HazardKind) -> Option<&HazardEntry> {
        self.kinds.iter().find(|e| e.kind == kind)
    }

    /// Effective probability for `kind` given an optional terrain slug.
    ///
    /// Returns `0.0` when there is no entry for the kind.
    #[must_use]
    pub fn effective_probability(&self, kind: HazardKind, terrain: Option<&str>) -> f32 {
        let Some(entry) = self.entry_for(kind) else {
            return 0.0;
        };
        let terrain_mod = terrain
            .and_then(|t| entry.terrain_modifiers.get(t))
            .copied()
            .unwrap_or(1.0);
        (entry.config.base_probability * terrain_mod).clamp(0.0, 1.0)
    }
}

// ─── Outcome ─────────────────────────────────────────────────────────────────

/// The computed effects of one hazard firing on one colony.
#[derive(Debug, Clone)]
pub struct HazardOutcome {
    /// Which colony was hit.
    pub colony_id: crate::colony::ColonyId,
    /// Which hazard occurred.
    pub kind: HazardKind,
    /// Sampled severity `[0.0, 1.0]`.
    pub severity: f32,
    /// Stability reduction applied (absolute, negative).
    pub stability_delta: f32,
    /// Commodity losses: `(commodity_id, amount_lost)`.
    pub commodity_losses: Vec<(String, f64)>,
    /// Population lost (absolute head-count reduction).
    pub population_lost: f32,
}

// ─── Roll helper ─────────────────────────────────────────────────────────────

/// Roll a single hazard for one colony.
///
/// Returns `Some(HazardOutcome)` when the hazard triggers, `None` otherwise.
///
/// # Parameters
///
/// - `rng_prob`    — uniform float in `[0.0, 1.0)` used for the trigger roll.
/// - `rng_sev`     — uniform float in `[0.0, 1.0)` used to sample severity.
/// - `rng_comm`    — commodity index (modulo the pool size) for selecting which commodity is damaged.
/// - `kind`        — which hazard to roll for.
/// - `colony_id`   — colony being evaluated.
/// - `terrain`     — optional terrain slug for probability modification.
/// - `config`      — hazard configuration.
/// - `population`  — current colony head-count.
/// - `pool_entries`— snapshot of `(commodity_id, amount)` pairs from the colony pool.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn roll_hazard(
    rng_prob: f32,
    rng_sev: f32,
    rng_comm: usize,
    kind: HazardKind,
    colony_id: crate::colony::ColonyId,
    terrain: Option<&str>,
    config: &HazardConfig,
    population: f32,
    pool_entries: &[(String, f64)],
) -> Option<HazardOutcome> {
    let entry = config.entry_for(kind)?;
    let prob = config.effective_probability(kind, terrain);

    if rng_prob >= prob {
        return None;
    }

    let severity = entry.config.sample_severity(rng_sev);

    // Stability hit
    let stability_delta = -(severity * entry.config.stability_damage_per_severity);

    // Commodity loss — pick one commodity from the pool (if any) and drain it.
    let commodity_losses = if pool_entries.is_empty() {
        vec![]
    } else {
        let idx = rng_comm % pool_entries.len();
        let (comm_id, amount) = &pool_entries[idx];
        let loss = amount * f64::from(severity * entry.config.commodity_loss_per_severity);
        vec![(comm_id.clone(), loss)]
    };

    // Population damage
    let population_lost = population * severity * entry.config.population_damage_per_severity;

    Some(HazardOutcome {
        colony_id,
        kind,
        severity,
        stability_delta,
        commodity_losses,
        population_lost,
    })
}

// ─── Pack loading ────────────────────────────────────────────────────────────

/// Why a `hazards.yaml` could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum HazardLoadError {
    /// The YAML did not parse into a [`HazardConfig`].
    #[error("hazards.yaml parse error: {0}")]
    Parse(String),
    /// An entry declared a probability or severity outside `[0, 1]`, or a
    /// severity range that runs backwards.
    #[error("hazards.yaml: {0}")]
    Invalid(String),
}

/// Load and validate a [`HazardConfig`] from raw YAML (issue #421).
///
/// Mirrors [`crate::tech::load_tech_registry`]: hazards are content, and both
/// hosts read this file themselves rather than going through `PackLoader`,
/// which only knows the ten tables that make up a `ContentRegistry`.
///
/// Validation is not merely defensive. `roll_hazard` compares a `[0, 1)` roll
/// against the configured probability, so a negative probability silently
/// disables a hazard kind and a probability above `1.0` silently makes it
/// fire every single sol — neither of which would surface as an error, only
/// as a game that feels wrong. Catching it at load turns a mystifying
/// play-time symptom into a message naming the offending entry.
///
/// # Errors
///
/// Returns [`HazardLoadError::Parse`] if the YAML is malformed, or
/// [`HazardLoadError::Invalid`] if an entry is out of range.
pub fn load_hazard_config(yaml: &str) -> Result<HazardConfig, HazardLoadError> {
    let config: HazardConfig =
        serde_yaml::from_str(yaml).map_err(|e| HazardLoadError::Parse(e.to_string()))?;

    for entry in &config.kinds {
        let c = &entry.config;
        let kind = entry.kind;
        if !(0.0..=1.0).contains(&c.base_probability) {
            return Err(HazardLoadError::Invalid(format!(
                "{kind:?} has base_probability {}, which must be in [0, 1]",
                c.base_probability
            )));
        }
        if !(0.0..=1.0).contains(&c.severity_min) || !(0.0..=1.0).contains(&c.severity_max) {
            return Err(HazardLoadError::Invalid(format!(
                "{kind:?} has severity range [{}, {}], which must lie in [0, 1]",
                c.severity_min, c.severity_max
            )));
        }
        if c.severity_min > c.severity_max {
            return Err(HazardLoadError::Invalid(format!(
                "{kind:?} has severity_min {} above severity_max {}",
                c.severity_min, c.severity_max
            )));
        }
        for (terrain, modifier) in &entry.terrain_modifiers {
            if *modifier < 0.0 || !modifier.is_finite() {
                return Err(HazardLoadError::Invalid(format!(
                    "{kind:?} has terrain modifier {modifier} for {terrain:?}, \
                     which must be finite and non-negative"
                )));
            }
        }
    }

    Ok(config)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::ColonyId;

    fn make_config(probability: f32) -> HazardConfig {
        let entry = HazardEntry {
            kind: HazardKind::DustStorm,
            config: HazardKindConfig {
                base_probability: probability,
                severity_min: 0.1,
                severity_max: 0.9,
                stability_damage_per_severity: 0.5,
                commodity_loss_per_severity: 0.1,
                population_damage_per_severity: 0.02,
            },
            terrain_modifiers: Default::default(),
        };
        HazardConfig { kinds: vec![entry] }
    }

    fn dummy_id() -> ColonyId {
        uuid::Uuid::new_v4()
    }

    // ── Loading the authored pack (issue #421) ──────────────────────────────

    fn read_real_hazards_yaml() -> Option<String> {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
        let root = std::path::Path::new(&manifest).parent()?.to_path_buf();
        let path = root.join("content").join("base").join("hazards.yaml");
        std::fs::read_to_string(path).ok()
    }

    /// The authored file must load, and load with every kind present.
    ///
    /// `hazards.yaml` sat in the pack unread for the whole life of the
    /// project — `PackLoader` only knows the ten tables that make up a
    /// `ContentRegistry`, and nothing else parsed it — so nothing had ever
    /// checked that it was even valid.
    #[test]
    fn the_authored_hazards_yaml_loads_with_every_kind_configured() {
        let Some(yaml) = read_real_hazards_yaml() else {
            return; // content/ not present in this checkout layout; skip.
        };
        let config = load_hazard_config(&yaml).expect("content/base/hazards.yaml must load");
        for kind in HazardKind::ALL {
            assert!(
                config.entry_for(kind).is_some(),
                "{kind:?} has no authored entry, so it could never fire"
            );
        }
    }

    /// One well-formed `dust_storm` entry with the three fields these tests
    /// vary. Built rather than written inline: a hand-indented multi-line
    /// literal is one `cargo fmt` away from becoming malformed YAML, which
    /// turns a validation test into a parse test without failing loudly.
    fn one_entry_yaml(probability: &str, severity_min: &str, severity_max: &str) -> String {
        format!(
            "kinds:\n  - kind: dust_storm\n    base_probability: {probability}\n    severity_min: {severity_min}\n    severity_max: {severity_max}\n    stability_damage_per_severity: 0.1\n    commodity_loss_per_severity: 0.1\n    population_damage_per_severity: 0.1\n"
        )
    }

    #[test]
    fn the_entry_builder_produces_yaml_that_actually_loads() {
        // Guards the three tests below: each asserts a *rejection*, and would
        // pass just as happily if the builder emitted garbage.
        assert!(load_hazard_config(&one_entry_yaml("0.1", "0.2", "0.8")).is_ok());
    }

    #[test]
    fn a_probability_outside_the_unit_range_is_rejected_at_load() {
        // Above 1.0 would fire every sol; below 0.0 would never fire. Both
        // are silent at runtime, which is exactly why they are caught here.
        for probability in ["1.5", "-0.2"] {
            assert!(
                matches!(
                    load_hazard_config(&one_entry_yaml(probability, "0.1", "0.5")),
                    Err(HazardLoadError::Invalid(_))
                ),
                "base_probability {probability} must be rejected"
            );
        }
    }

    #[test]
    fn a_backwards_severity_range_is_rejected_at_load() {
        assert!(matches!(
            load_hazard_config(&one_entry_yaml("0.1", "0.9", "0.2")),
            Err(HazardLoadError::Invalid(_))
        ));
    }

    /// The heart of issue #421: with the authored table attached, hazards
    /// must actually fire over a long game.
    ///
    /// Asserted rather than eyeballed, and deliberately end-to-end through
    /// `TurnProcessor::advance` rather than calling `roll_hazard` directly —
    /// the bug was never in the rolling, which was correct and unit-tested
    /// throughout. It was that nothing ever populated `hazard_config`, so the
    /// whole step was skipped. Only a test that runs the real pipeline with
    /// the real file could have caught that.
    #[test]
    fn the_authored_table_actually_fires_hazards_over_a_long_game() {
        use crate::colony::Colony;
        use crate::turn::{GameState, TurnProcessor};

        let Some(yaml) = read_real_hazards_yaml() else {
            return;
        };
        let config = load_hazard_config(&yaml).expect("hazards.yaml must load");

        let mut state = GameState::new();
        state.add_colony(Colony::new("Exposed"), 1000);
        state.hazard_config = Some(config);

        let mut processor = TurnProcessor::new(20_240_421);
        let mut kinds_seen = std::collections::HashSet::new();
        let mut total = 0usize;
        for _ in 0..500 {
            for outcome in processor.advance(&mut state).hazard_outcomes {
                kinds_seen.insert(outcome.kind);
                total += 1;
            }
        }

        assert!(
            total > 0,
            "500 sols with the authored hazard table produced no hazards at all —              the threat layer is inert"
        );
        // The rarest authored kind is meteor_impact at 0.001/sol, so ~0.5
        // expected occurrences over 500 sols; requiring every kind would make
        // this flaky. Requiring most of them still proves the table is being
        // read as a whole rather than one entry happening to work.
        assert!(
            kinds_seen.len() >= 4,
            "only {} of 6 hazard kinds ever fired over 500 sols: {kinds_seen:?}",
            kinds_seen.len()
        );
    }

    /// The other half of the same bug: a colony that never recorded its
    /// terrain cannot have terrain modifiers applied to it (issue #421).
    #[test]
    fn terrain_modifiers_only_bite_when_a_colony_records_its_terrain() {
        let Some(yaml) = read_real_hazards_yaml() else {
            return;
        };
        let config = load_hazard_config(&yaml).expect("hazards.yaml must load");

        // seismic_event authors volcanic 3.0 and plains 0.6.
        let volcanic = config.effective_probability(HazardKind::SeismicEvent, Some("volcanic"));
        let plains = config.effective_probability(HazardKind::SeismicEvent, Some("plains"));
        let unknown = config.effective_probability(HazardKind::SeismicEvent, None);

        assert!(
            volcanic > unknown && unknown > plains,
            "volcanic {volcanic} should exceed the unmodified {unknown}, which \
             should exceed plains {plains}"
        );
    }

    #[test]
    fn malformed_yaml_is_a_parse_error_not_a_panic() {
        assert!(matches!(
            load_hazard_config("kinds: [this is not a hazard entry]"),
            Err(HazardLoadError::Parse(_))
        ));
    }

    fn pool() -> Vec<(String, f64)> {
        vec![("food".to_string(), 1000.0)]
    }

    #[test]
    fn zero_probability_hazard_never_fires() {
        let cfg = make_config(0.0);
        let id = dummy_id();
        // Any rng_prob value in [0,1) should never pass the 0.0 probability gate.
        for i in 0..20 {
            let rng_prob = i as f32 / 20.0;
            let result = roll_hazard(
                rng_prob,
                0.5,
                0,
                HazardKind::DustStorm,
                id,
                None,
                &cfg,
                100.0,
                &pool(),
            );
            assert!(
                result.is_none(),
                "expected no trigger at prob=0, rng={rng_prob}"
            );
        }
    }

    #[test]
    fn probability_one_always_fires() {
        let cfg = make_config(1.0);
        let id = dummy_id();
        // With probability=1.0 any rng_prob < 1.0 triggers.
        for i in 0..10 {
            let rng_prob = i as f32 / 10.0;
            let result = roll_hazard(
                rng_prob,
                0.5,
                0,
                HazardKind::DustStorm,
                id,
                None,
                &cfg,
                100.0,
                &pool(),
            );
            assert!(
                result.is_some(),
                "expected trigger at prob=1, rng={rng_prob}"
            );
        }
    }

    #[test]
    fn severity_scales_effects() {
        let cfg = make_config(1.0);
        let id = dummy_id();

        let low = roll_hazard(
            0.0,
            0.0,
            0,
            HazardKind::DustStorm,
            id,
            None,
            &cfg,
            200.0,
            &pool(),
        )
        .unwrap();
        let high = roll_hazard(
            0.0,
            1.0,
            0,
            HazardKind::DustStorm,
            id,
            None,
            &cfg,
            200.0,
            &pool(),
        )
        .unwrap();

        assert!(
            high.stability_delta.abs() >= low.stability_delta.abs(),
            "higher severity should produce equal or greater stability damage"
        );
        assert!(
            high.population_lost >= low.population_lost,
            "higher severity should produce equal or greater population loss"
        );
    }

    #[test]
    fn terrain_modifier_scales_probability() {
        let mut entry = HazardEntry {
            kind: HazardKind::DustStorm,
            config: HazardKindConfig {
                base_probability: 0.5,
                severity_min: 0.1,
                severity_max: 0.9,
                stability_damage_per_severity: 0.5,
                commodity_loss_per_severity: 0.1,
                population_damage_per_severity: 0.02,
            },
            terrain_modifiers: Default::default(),
        };
        entry.terrain_modifiers.insert("desert".to_string(), 2.0);
        let cfg = HazardConfig { kinds: vec![entry] };

        // base 0.5 × 2.0 = 1.0 → any rng_prob < 1.0 fires
        let id = dummy_id();
        let result = roll_hazard(
            0.99,
            0.5,
            0,
            HazardKind::DustStorm,
            id,
            Some("desert"),
            &cfg,
            100.0,
            &pool(),
        );
        assert!(
            result.is_some(),
            "desert terrain should double prob to 1.0 and always fire"
        );

        // without terrain modifier at rng_prob=0.6 (>0.5) should not fire
        let result2 = roll_hazard(
            0.6,
            0.5,
            0,
            HazardKind::DustStorm,
            id,
            None,
            &cfg,
            100.0,
            &pool(),
        );
        assert!(
            result2.is_none(),
            "without terrain modifier, prob=0.5 should not fire at 0.6"
        );
    }

    #[test]
    fn all_six_hazard_kinds_can_be_configured_and_rolled() {
        let kinds_entries: Vec<HazardEntry> = HazardKind::ALL
            .iter()
            .map(|&kind| HazardEntry {
                kind,
                config: HazardKindConfig {
                    base_probability: 1.0,
                    severity_min: 0.5,
                    severity_max: 0.5,
                    stability_damage_per_severity: 0.1,
                    commodity_loss_per_severity: 0.05,
                    population_damage_per_severity: 0.01,
                },
                terrain_modifiers: Default::default(),
            })
            .collect();
        let cfg = HazardConfig {
            kinds: kinds_entries,
        };
        let id = dummy_id();

        for kind in HazardKind::ALL {
            let result = roll_hazard(0.0, 0.5, 0, kind, id, None, &cfg, 100.0, &pool());
            assert!(
                result.is_some(),
                "kind {kind:?} should fire with probability=1.0"
            );
        }
    }

    #[test]
    fn commodity_loss_applied_to_pool_entry() {
        let cfg = make_config(1.0);
        let id = dummy_id();
        // pool has 1000 food
        let pool_entries = vec![("food".to_string(), 1000.0_f64)];
        let result = roll_hazard(
            0.0,
            1.0,
            0,
            HazardKind::DustStorm,
            id,
            None,
            &cfg,
            100.0,
            &pool_entries,
        )
        .unwrap();
        // severity=0.9 (max), commodity_loss_per_severity=0.1
        // loss = 1000 * 0.9 * 0.1 = 90
        assert!(!result.commodity_losses.is_empty());
        let (_id, loss) = &result.commodity_losses[0];
        assert!(*loss > 0.0, "expected positive commodity loss, got {loss}");
    }

    #[test]
    fn yaml_roundtrip() {
        let entry = HazardEntry {
            kind: HazardKind::RadiationLeak,
            config: HazardKindConfig {
                base_probability: 0.01,
                severity_min: 0.2,
                severity_max: 0.8,
                stability_damage_per_severity: 0.3,
                commodity_loss_per_severity: 0.05,
                population_damage_per_severity: 0.04,
            },
            terrain_modifiers: Default::default(),
        };
        let cfg = HazardConfig { kinds: vec![entry] };
        let yaml = serde_yaml::to_string(&cfg).expect("should serialise");
        let back: HazardConfig = serde_yaml::from_str(&yaml).expect("should deserialise");
        assert_eq!(back.kinds.len(), 1);
        assert_eq!(back.kinds[0].kind, HazardKind::RadiationLeak);
        assert!((back.kinds[0].config.base_probability - 0.01).abs() < 1e-5);
    }
}
