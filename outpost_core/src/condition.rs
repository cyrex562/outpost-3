//! Building condition and breakdown — the maintenance failure model (issue #384).
//!
//! Issue #180 made unmet maintenance *throttle* a building's output. That is a
//! smooth, immediate, fully-recoverable penalty, and it is all that happened:
//! nothing in the engine could ever actually break. §5A of `docs/DESIGN.md`
//! records the decision that maintenance should carry a real failure model,
//! and that it should be **both** halves rather than a choice between them:
//!
//! - a **condition** stat that degrades under unmet maintenance — deterministic
//!   and telegraphable, so a player can see trouble coming and act; and
//! - a per-sol **breakdown roll** whose odds worsen as condition falls —
//!   discrete, so failure lands as an event rather than as a slow fade.
//!
//! The two do different jobs. Condition alone would make failure perfectly
//! predictable and therefore ignorable until the last sol. A roll alone would
//! make it feel arbitrary — a building exploding with no warning and no way to
//! have known. Together, condition is the warning and the roll is the
//! consequence.
//!
//! This module is pure arithmetic over `f32`: no state, no RNG, no I/O. The
//! caller supplies the random draw, which is what keeps the whole system
//! deterministic for a fixed seed (see `TurnProcessor`'s `ChaCha8Rng`).

use crate::system::AtmosphereHazard;

/// Condition of a newly-built building — pristine.
pub const CONDITION_NEW: f32 = 1.0;

/// Condition lost per sol while a building's maintenance goes unpaid.
///
/// At `0.02`, a wholly unmaintained building falls from pristine to derelict
/// in 50 sols, and reaches [`BREAKDOWN_THRESHOLD`] — where it starts risking
/// failure at all — in 25. That is deliberately slow: the player has to be
/// able to notice, be warned, and still have time to react, which is the whole
/// point of pairing a visible stat with the roll.
pub const CONDITION_DECAY_PER_SOL: f32 = 0.02;

/// Condition regained per sol while maintenance is being paid in full.
///
/// Half the decay rate on purpose: letting a building rot is faster than
/// nursing it back, so neglect has a cost that outlasts the neglect itself.
/// This is ordinary wear being serviced — it does **not** revive a building
/// that has actually broken, which needs [`repair`](crate::Command::RepairBuilding)
/// and materials.
pub const CONDITION_RECOVERY_PER_SOL: f32 = 0.01;

/// Condition at or below which a building can break down at all.
///
/// Above this, upkeep is being deferred but nothing is at risk yet — the
/// building is merely wearing. This gives the degradation half of the model a
/// job to do before the roll half switches on, so the first thing a player
/// meets is a warning rather than a failure.
pub const BREAKDOWN_THRESHOLD: f32 = 0.5;

/// Per-sol breakdown chance for a building at zero condition, before any
/// atmospheric modifier.
///
/// `0.04` gives a derelict building a median life of about 17 sols. Low enough
/// that a single bad sol is not a catastrophe, high enough that running a
/// colony on wrecks is not a viable strategy.
pub const MAX_BREAKDOWN_CHANCE: f32 = 0.04;

/// Condition a building is left at once repaired.
///
/// Not `1.0`: a repaired building is serviceable, not new. The remaining gap
/// closes through ordinary maintenance via [`CONDITION_RECOVERY_PER_SOL`], so
/// repair gets a building working again without also erasing the history of
/// having neglected it.
pub const CONDITION_AFTER_REPAIR: f32 = 0.75;

/// Fraction of a building's construction cost charged to repair it, when its
/// content pack authors no explicit `repair_cost`.
///
/// A default rather than a required field: every building can break, so
/// requiring an authored repair cost on all of them would mean a content edit
/// per building and a trap for every building added later. An author who wants
/// a specific figure still sets one.
pub const DEFAULT_REPAIR_COST_FRACTION: f64 = 0.35;

/// How this atmosphere multiplies the per-sol breakdown chance (issue #384).
///
/// This is the discrete half of `OxidizingCombustible` that issue #438
/// deliberately left out of [`AtmosphereHazard::maintenance_multiplier`].
/// Oxidation is a steady drain and belongs in upkeep; **fire and explosion
/// risk is an event**, and modelling it as slightly dearer maintenance would
/// have described an occasional catastrophe as a predictable bill. It belongs
/// here, on the roll, which is what that issue's note on #384 promised.
///
/// `Corrosive` also raises the odds, more mildly: it already charges more
/// upkeep, and a corroded structure fails more readily than a sound one. Its
/// main cost stays in maintenance, so the multiplier here is small enough not
/// to double-charge for the same hazard.
#[must_use]
pub fn breakdown_multiplier(hazard: AtmosphereHazard) -> f32 {
    match hazard {
        // Toxic is lethal to people, not to equipment — the same reasoning
        // that makes it maintenance-neutral in #438 makes it breakdown-neutral
        // here, and for the same reason it is stated rather than assumed.
        AtmosphereHazard::None | AtmosphereHazard::Toxic => 1.0,
        AtmosphereHazard::Corrosive => 1.25,
        AtmosphereHazard::OxidizingCombustible => 2.0,
    }
}

/// Condition after one sol, given whether maintenance was met.
///
/// Clamped to `[0.0, 1.0]`. A building already at zero cannot go lower, and a
/// pristine one cannot bank surplus condition against future neglect.
#[must_use]
pub fn step_condition(condition: f32, maintenance_met: bool) -> f32 {
    let delta = if maintenance_met {
        CONDITION_RECOVERY_PER_SOL
    } else {
        -CONDITION_DECAY_PER_SOL
    };
    (condition + delta).clamp(0.0, 1.0)
}

/// Per-sol probability that a building at `condition` breaks down.
///
/// Zero above [`BREAKDOWN_THRESHOLD`], then rising linearly to
/// `MAX_BREAKDOWN_CHANCE × hazard multiplier` at zero condition. Linear rather
/// than a curve because this number has to be *shown* to the player: a
/// straight line between two stated endpoints is a risk they can reason about,
/// where an exponent is one they can only be surprised by.
///
/// The result is clamped to `[0.0, 1.0]` so a hazard multiplier can never push
/// it into certainty.
#[must_use]
pub fn breakdown_chance(condition: f32, hazard: AtmosphereHazard) -> f32 {
    if condition > BREAKDOWN_THRESHOLD {
        return 0.0;
    }
    let severity = ((BREAKDOWN_THRESHOLD - condition) / BREAKDOWN_THRESHOLD).clamp(0.0, 1.0);
    (severity * MAX_BREAKDOWN_CHANCE * breakdown_multiplier(hazard)).clamp(0.0, 1.0)
}

/// Whether a building breaks down this sol.
///
/// `roll` is a uniform draw in `[0, 1)` supplied by the caller's seeded RNG —
/// this module stays pure so the whole model is reproducible for a fixed seed.
#[must_use]
pub fn breaks_down(condition: f32, hazard: AtmosphereHazard, roll: f32) -> bool {
    roll < breakdown_chance(condition, hazard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unmaintained_building_decays_and_a_maintained_one_recovers() {
        assert!(step_condition(1.0, false) < 1.0);
        assert!(step_condition(0.5, true) > 0.5);
    }

    #[test]
    fn condition_is_clamped_at_both_ends() {
        assert!((step_condition(1.0, true) - 1.0).abs() < f32::EPSILON);
        assert!((step_condition(0.0, false) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn decay_outpaces_recovery() {
        // Neglect must cost more than it saves, or deferring maintenance is
        // free whenever the player can afford a quiet spell afterwards.
        let decayed = 1.0 - step_condition(1.0, false);
        let recovered = step_condition(0.5, true) - 0.5;
        assert!(
            decayed > recovered,
            "decay {decayed} should outpace recovery {recovered}"
        );
    }

    #[test]
    fn a_healthy_building_never_breaks_down() {
        for condition in [1.0, 0.9, 0.75, BREAKDOWN_THRESHOLD + 0.01] {
            assert!(
                (breakdown_chance(condition, AtmosphereHazard::None)).abs() < f32::EPSILON,
                "condition {condition} should carry no breakdown risk"
            );
        }
    }

    #[test]
    fn breakdown_risk_rises_as_condition_falls() {
        let mild = breakdown_chance(0.4, AtmosphereHazard::None);
        let bad = breakdown_chance(0.2, AtmosphereHazard::None);
        let derelict = breakdown_chance(0.0, AtmosphereHazard::None);
        assert!(
            mild < bad && bad < derelict,
            "risk should climb: {mild} < {bad} < {derelict}"
        );
        assert!((derelict - MAX_BREAKDOWN_CHANCE).abs() < 1e-6);
    }

    /// Closes the loop left open in #438: the combustible half of an
    /// oxidising atmosphere lands on the breakdown roll, not on upkeep.
    #[test]
    fn an_oxidizing_atmosphere_raises_breakdown_odds() {
        let inert = breakdown_chance(0.2, AtmosphereHazard::None);
        let oxidizing = breakdown_chance(0.2, AtmosphereHazard::OxidizingCombustible);
        let corrosive = breakdown_chance(0.2, AtmosphereHazard::Corrosive);
        assert!(
            oxidizing > corrosive && corrosive > inert,
            "fire risk should dominate: inert {inert}, corrosive {corrosive}, \
             oxidizing {oxidizing}"
        );
    }

    #[test]
    fn a_toxic_atmosphere_does_not_raise_breakdown_odds() {
        // Same reasoning as its maintenance neutrality in #438: toxicity
        // threatens people, not machinery. Pinned so a later change is a
        // deliberate one.
        let inert = breakdown_chance(0.2, AtmosphereHazard::None);
        let toxic = breakdown_chance(0.2, AtmosphereHazard::Toxic);
        assert!((inert - toxic).abs() < f32::EPSILON);
    }

    #[test]
    fn a_hazard_multiplier_can_never_make_breakdown_certain() {
        for hazard in [
            AtmosphereHazard::None,
            AtmosphereHazard::Corrosive,
            AtmosphereHazard::Toxic,
            AtmosphereHazard::OxidizingCombustible,
        ] {
            let chance = breakdown_chance(0.0, hazard);
            assert!(
                (0.0..=1.0).contains(&chance),
                "{hazard:?} produced an out-of-range chance {chance}"
            );
        }
    }

    #[test]
    fn the_roll_is_a_strict_comparison_against_the_chance() {
        // A roll exactly at the chance must not fire, matching `roll_hazard`'s
        // convention so the two random systems read the same way.
        let chance = breakdown_chance(0.0, AtmosphereHazard::None);
        assert!(breaks_down(0.0, AtmosphereHazard::None, chance - 1e-6));
        assert!(!breaks_down(0.0, AtmosphereHazard::None, chance));
    }

    #[test]
    fn a_pristine_building_never_breaks_however_unlucky_the_roll() {
        assert!(!breaks_down(
            1.0,
            AtmosphereHazard::OxidizingCombustible,
            0.0
        ));
    }
}
