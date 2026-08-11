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

/// One world property a hazard modifier can key off (issue #448).
///
/// Externally tagged, so a modifier reads as `{ terrain: volcanic, x: 3.0 }`
/// — the property name is the key and its value the payload.
///
/// Four axes rather than one because the authored table always spanned four:
/// dust storms keyed off *biome* (desert, tundra), seismic events off
/// *terrain* (volcanic, plains), meteor impacts off how thin the air is
/// ("exposed"), and radiation leaks off how irradiated the body is. Before
/// this those last two named nothing the engine knew, so they silently did
/// nothing — as did six of the eleven slugs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardCondition {
    /// The colony hex's terrain.
    Terrain(crate::map::Terrain),
    /// The colony hex's biome.
    Biome(crate::map::Biome),
    /// The body's atmospheric density — thin air lets more impactors through.
    Atmosphere(crate::system::AtmosphereDensity),
    /// The body's ambient radiation level.
    Radiation(crate::system::RadiationLevel),
}

/// One probability multiplier, and the world property that triggers it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HazardModifier {
    /// What must hold for this multiplier to apply.
    #[serde(flatten)]
    pub condition: HazardCondition,
    /// Probability multiplier applied when it does.
    pub x: f32,
}

/// The world properties of a colony's location, as hazards see it.
///
/// Every field is optional: a colony with no surface placement can still be
/// rolled for, it simply matches no hex-scoped modifier.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HazardSite {
    /// Terrain of the colony's hex.
    pub terrain: Option<crate::map::Terrain>,
    /// Biome of the colony's hex.
    pub biome: Option<crate::map::Biome>,
    /// Atmospheric density of the body.
    pub atmosphere: Option<crate::system::AtmosphereDensity>,
    /// Ambient radiation of the body.
    pub radiation: Option<crate::system::RadiationLevel>,
}

impl HazardSite {
    /// Whether this site satisfies `condition`.
    #[must_use]
    pub fn matches(&self, condition: HazardCondition) -> bool {
        match condition {
            HazardCondition::Terrain(t) => self.terrain == Some(t),
            HazardCondition::Biome(b) => self.biome == Some(b),
            HazardCondition::Atmosphere(a) => self.atmosphere == Some(a),
            HazardCondition::Radiation(r) => self.radiation == Some(r),
        }
    }
}

/// One entry in the hazard YAML list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HazardEntry {
    /// Which hazard kind this entry configures.
    pub kind: HazardKind,
    /// Tunable parameters for this kind.
    #[serde(flatten)]
    pub config: HazardKindConfig,
    /// Probability multipliers keyed off the site's world properties.
    ///
    /// **Every match multiplies** (issue #448): a volcanic-terrain,
    /// barren-biome site takes both a seismic terrain modifier and a seismic
    /// biome one. Each property is an independent statement about the place,
    /// and this is how `output_scaling`, the difficulty scalars, and hazard
    /// susceptibility already compose. The product is clamped to a
    /// probability, so stacking cannot run past certainty.
    #[serde(default)]
    pub modifiers: Vec<HazardModifier>,
}

impl HazardConfig {
    /// Find the config entry for `kind`, if present.
    #[must_use]
    pub fn entry_for(&self, kind: HazardKind) -> Option<&HazardEntry> {
        self.kinds.iter().find(|e| e.kind == kind)
    }

    /// Effective probability for `kind` at `site`.
    ///
    /// Every matching modifier multiplies (issue #448); the result is clamped
    /// into `[0, 1]` so a stack of them cannot exceed certainty. Returns `0.0`
    /// when there is no entry for the kind.
    #[must_use]
    pub fn effective_probability(&self, kind: HazardKind, site: &HazardSite) -> f32 {
        let Some(entry) = self.entry_for(kind) else {
            return 0.0;
        };
        let multiplier: f32 = entry
            .modifiers
            .iter()
            .filter(|m| site.matches(m.condition))
            .map(|m| m.x)
            .product();
        (entry.config.base_probability * multiplier).clamp(0.0, 1.0)
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
/// - `site`        — the location's world properties, for modifiers.
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
    site: &HazardSite,
    config: &HazardConfig,
    population: f32,
    pool_entries: &[(String, f64)],
) -> Option<HazardOutcome> {
    let entry = config.entry_for(kind)?;
    let prob = config.effective_probability(kind, site);

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
        for m in &entry.modifiers {
            if m.x < 0.0 || !m.x.is_finite() {
                return Err(HazardLoadError::Invalid(format!(
                    "{kind:?} has modifier {} for {:?}, which must be finite \
                     and non-negative",
                    m.x, m.condition
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
            modifiers: Vec::new(),
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

    /// The authored table must actually discriminate between places — a
    /// modifier that reads the same everywhere is not a modifier.
    #[test]
    fn the_authored_table_discriminates_between_sites() {
        let Some(yaml) = read_real_hazards_yaml() else {
            return;
        };
        let config = load_hazard_config(&yaml).expect("hazards.yaml must load");

        // seismic_event authors volcanic 3.0 and plains 0.6.
        let site = |terrain| HazardSite {
            terrain: Some(terrain),
            ..HazardSite::default()
        };
        let volcanic = config.effective_probability(
            HazardKind::SeismicEvent,
            &site(crate::map::Terrain::Volcanic),
        );
        let plains = config
            .effective_probability(HazardKind::SeismicEvent, &site(crate::map::Terrain::Plains));
        let unknown =
            config.effective_probability(HazardKind::SeismicEvent, &HazardSite::default());

        assert!(
            volcanic > unknown && unknown > plains,
            "volcanic {volcanic} should exceed the unmodified {unknown}, which \
             should exceed plains {plains}"
        );
    }

    // ── Multi-axis modifiers (issue #448) ───────────────────────────────

    /// Every matching modifier multiplies — a site that is bad in two
    /// independent ways is worse than one bad in a single way.
    #[test]
    fn matching_modifiers_multiply_together() {
        let cfg = HazardConfig {
            kinds: vec![HazardEntry {
                kind: HazardKind::SeismicEvent,
                config: HazardKindConfig {
                    base_probability: 0.1,
                    severity_min: 0.1,
                    severity_max: 0.2,
                    stability_damage_per_severity: 0.0,
                    commodity_loss_per_severity: 0.0,
                    population_damage_per_severity: 0.0,
                },
                modifiers: vec![
                    HazardModifier {
                        condition: HazardCondition::Terrain(crate::map::Terrain::Volcanic),
                        x: 3.0,
                    },
                    HazardModifier {
                        condition: HazardCondition::Biome(crate::map::Biome::Barren),
                        x: 1.5,
                    },
                ],
            }],
        };
        let both = cfg.effective_probability(
            HazardKind::SeismicEvent,
            &HazardSite {
                terrain: Some(crate::map::Terrain::Volcanic),
                biome: Some(crate::map::Biome::Barren),
                ..HazardSite::default()
            },
        );
        let one = cfg.effective_probability(
            HazardKind::SeismicEvent,
            &HazardSite {
                terrain: Some(crate::map::Terrain::Volcanic),
                ..HazardSite::default()
            },
        );
        assert!(
            (one - 0.3).abs() < 1e-6,
            "one modifier: 0.1 x 3.0, got {one}"
        );
        assert!(
            (both - 0.45).abs() < 1e-6,
            "both should multiply: 0.1 x 3.0 x 1.5, got {both}"
        );
    }

    /// Stacking cannot run past certainty.
    #[test]
    fn a_stack_of_modifiers_is_clamped_to_a_probability() {
        let cfg = HazardConfig {
            kinds: vec![HazardEntry {
                kind: HazardKind::DustStorm,
                config: HazardKindConfig {
                    base_probability: 0.5,
                    severity_min: 0.1,
                    severity_max: 0.2,
                    stability_damage_per_severity: 0.0,
                    commodity_loss_per_severity: 0.0,
                    population_damage_per_severity: 0.0,
                },
                modifiers: vec![
                    HazardModifier {
                        condition: HazardCondition::Biome(crate::map::Biome::Desert),
                        x: 5.0,
                    },
                    HazardModifier {
                        condition: HazardCondition::Atmosphere(
                            crate::system::AtmosphereDensity::Dense,
                        ),
                        x: 5.0,
                    },
                ],
            }],
        };
        let p = cfg.effective_probability(
            HazardKind::DustStorm,
            &HazardSite {
                biome: Some(crate::map::Biome::Desert),
                atmosphere: Some(crate::system::AtmosphereDensity::Dense),
                ..HazardSite::default()
            },
        );
        assert!((p - 1.0).abs() < 1e-6, "expected clamping to 1.0, got {p}");
    }

    /// A slug the engine does not know is a **load error**, not a silent
    /// no-op. Silence is exactly how eleven authored modifiers — six of them
    /// naming nothing at all — did nothing for the life of the project.
    #[test]
    fn an_unrecognised_modifier_value_is_rejected_at_load() {
        let yaml = "kinds:\n  - kind: seismic_event\n    base_probability: 0.1\n    severity_min: 0.1\n    severity_max: 0.5\n    stability_damage_per_severity: 0.1\n    commodity_loss_per_severity: 0.1\n    population_damage_per_severity: 0.1\n    modifiers:\n      - { terrain: rocky, x: 1.5 }\n";
        let err = load_hazard_config(yaml).expect_err("\"rocky\" is not a Terrain");
        assert!(
            matches!(err, HazardLoadError::Parse(_)),
            "expected a parse rejection, got {err:?}"
        );
        // The message has to name the offender, or the author is left hunting.
        assert!(
            format!("{err}").contains("rocky"),
            "the error should name the bad value, got {err}"
        );
    }

    #[test]
    fn an_unrecognised_property_name_is_rejected_at_load() {
        let yaml = "kinds:\n  - kind: seismic_event\n    base_probability: 0.1\n    severity_min: 0.1\n    severity_max: 0.5\n    stability_damage_per_severity: 0.1\n    commodity_loss_per_severity: 0.1\n    population_damage_per_severity: 0.1\n    modifiers:\n      - { elevation: high, x: 1.5 }\n";
        assert!(matches!(
            load_hazard_config(yaml),
            Err(HazardLoadError::Parse(_))
        ));
    }

    /// The whole point of #448: every authored modifier must name something
    /// the engine can actually match, so none of them is decoration.
    #[test]
    fn every_authored_modifier_can_actually_fire() {
        let Some(yaml) = read_real_hazards_yaml() else {
            return;
        };
        let config = load_hazard_config(&yaml).expect("hazards.yaml must load");
        let mut total = 0;
        for entry in &config.kinds {
            for m in &entry.modifiers {
                total += 1;
                // Build the one site that satisfies this condition and check
                // the probability actually moves off the base.
                let mut site = HazardSite::default();
                match m.condition {
                    HazardCondition::Terrain(t) => site.terrain = Some(t),
                    HazardCondition::Biome(b) => site.biome = Some(b),
                    HazardCondition::Atmosphere(a) => site.atmosphere = Some(a),
                    HazardCondition::Radiation(r) => site.radiation = Some(r),
                }
                let base = config.effective_probability(entry.kind, &HazardSite::default());
                let modified = config.effective_probability(entry.kind, &site);
                assert!(
                    (modified - base).abs() > f32::EPSILON,
                    "{:?}'s modifier {:?} x{} changes nothing — it is decoration",
                    entry.kind,
                    m.condition,
                    m.x
                );
            }
        }
        assert!(
            total >= 10,
            "expected a real table, found {total} modifiers"
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
                &HazardSite::default(),
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
                &HazardSite::default(),
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
            &HazardSite::default(),
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
            &HazardSite::default(),
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
            modifiers: Vec::new(),
        };
        entry.modifiers.push(HazardModifier {
            condition: HazardCondition::Biome(crate::map::Biome::Desert),
            x: 2.0,
        });
        let cfg = HazardConfig { kinds: vec![entry] };

        // base 0.5 × 2.0 = 1.0 → any rng_prob < 1.0 fires
        let id = dummy_id();
        let result = roll_hazard(
            0.99,
            0.5,
            0,
            HazardKind::DustStorm,
            id,
            &HazardSite {
                biome: Some(crate::map::Biome::Desert),
                ..HazardSite::default()
            },
            &cfg,
            100.0,
            &pool(),
        );
        assert!(
            result.is_some(),
            "a desert biome should double prob to 1.0 and always fire"
        );

        // with no matching modifier at rng_prob=0.6 (>0.5) should not fire
        let result2 = roll_hazard(
            0.6,
            0.5,
            0,
            HazardKind::DustStorm,
            id,
            &HazardSite::default(),
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
                modifiers: Vec::new(),
            })
            .collect();
        let cfg = HazardConfig {
            kinds: kinds_entries,
        };
        let id = dummy_id();

        for kind in HazardKind::ALL {
            let result = roll_hazard(
                0.0,
                0.5,
                0,
                kind,
                id,
                &HazardSite::default(),
                &cfg,
                100.0,
                &pool(),
            );
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
            &HazardSite::default(),
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
            modifiers: Vec::new(),
        };
        let cfg = HazardConfig { kinds: vec![entry] };
        let yaml = serde_yaml::to_string(&cfg).expect("should serialise");
        let back: HazardConfig = serde_yaml::from_str(&yaml).expect("should deserialise");
        assert_eq!(back.kinds.len(), 1);
        assert_eq!(back.kinds[0].kind, HazardKind::RadiationLeak);
        assert!((back.kinds[0].config.base_probability - 0.01).abs() < 1e-5);
    }
}
