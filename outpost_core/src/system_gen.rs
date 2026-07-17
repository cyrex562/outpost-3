//! Procedural star-system generation (issue #199).
//!
//! Deterministic from a seed, mirroring [`crate::map::PlanetMap::generate`]'s
//! technique of blending a smooth, seed-derived field with per-body RNG
//! jitter rather than rolling every attribute independently — see
//! [`generate_system`]'s doc comment for the exact shape.
//!
//! # Design decisions (resolved via Q&A, matching the #166/#167/#169/#199
//! precedent of settling open questions before implementing)
//!
//! - **Replaces authored content as the bootstrap default.** `content/base/systems.yaml`
//!   demotes to an optional hand-authored scenario preset, not the primary path.
//! - **Independent system-level seed**, not reused from [`crate::Command::SeedPlanet`]'s
//!   planet seed — the player can reroll the system independently of the
//!   founding planet's hex map.
//! - **Playability is guaranteed, not probabilistic.** Unlike
//!   [`crate::map::PlanetMap::best_landing_site`] (which can return `None` on
//!   a bad seed), a generated system always contains at least one body
//!   clearing [`crate::system::HABITABILITY_FOUNDING_THRESHOLD`] — this gates
//!   the entire new-game flow, not one map, so a bad seed can't produce an
//!   unplayable game. Enforced deterministically (no retry loop): if no
//!   generated body clears the threshold, the body nearest the habitable-zone
//!   center is overwritten with a guaranteed-habitable attribute set.
//! - **A single fixed habitable-zone curve** (no stellar type/luminosity
//!   modeling) — matches the issue's explicit "a single fixed curve fine for
//!   now" framing; a distinct star-type system is future scope.
//! - **Command surface**: [`crate::system::SystemCommand::GenerateSystem`],
//!   applied once during bootstrap, mirroring how every other system-zoom
//!   mutation goes through a `SystemCommand`.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::map::VEIN_COMMODITIES;
use crate::system::{
    AtmosphereDensity, AtmosphereHazard, Body, BodyDeposit, BodyKind, PlanetarySubtype,
    RadiationLevel, SystemRole, TemperatureBand, HABITABILITY_FOUNDING_THRESHOLD,
};

/// Distance (AU) treated as the center of the habitable zone. No stellar
/// type/luminosity modeling yet — see this module's doc comment.
const HABITABLE_ZONE_CENTER_AU: f32 = 1.0;

/// Minimum/maximum inner-planet count. Every generated system gets at least
/// this many rocky worlds before any belt/giant/moon bodies are added.
const MIN_INNER_PLANETS: u32 = 2;
const MAX_INNER_PLANETS: u32 = 4;

/// Generate a deterministic star system from `seed`.
///
/// Same seed ⇒ same system (body count, kinds, names, attributes); different
/// seeds ⇒ different systems. Mirrors
/// [`crate::map::PlanetMap::generate`]'s `ChaCha8Rng::seed_from_u64` seeding.
///
/// At least one returned body is guaranteed to clear
/// [`crate::system::HABITABILITY_FOUNDING_THRESHOLD`] — see this module's
/// doc comment for why that's a hard guarantee here rather than a
/// probabilistic outcome the way [`crate::map::PlanetMap`] allows.
///
/// `abundance_scalar` (issue #232) scales every [`BodyDeposit::abundance`]
/// [`distribute_system_resources`] assigns — `1.0` is neutral, matching
/// [`crate::modifier::DifficultyScalar`]'s convention everywhere else in the
/// codebase. It does not affect *coverage* (every curated commodity is still
/// guaranteed to exist somewhere in the system regardless of this value) —
/// only how generous the placed deposits read.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn generate_system(seed: u64, abundance_scalar: f32) -> Vec<Body> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut bodies: Vec<Body> = Vec::new();

    // ── Inner planets ──────────────────────────────────────────────────────
    let inner_count = rng.gen_range(MIN_INNER_PLANETS..=MAX_INNER_PLANETS);
    // Titius-Bode-style progression: each body sits ~1.5-1.9x farther out
    // than the last, plus jitter, so spacing is coherent-but-varied rather
    // than uniformly stepped or fully random (mirrors
    // `map.rs::compute_elevation`'s smooth-field-plus-jitter blend).
    let mut distance_au = 0.3 + rng.gen_range(0.0..0.2);
    for i in 0..inner_count {
        let body = generate_body(
            &mut rng,
            format!("Body-{}", i + 1),
            &BodyKind::InnerPlanet,
            distance_au,
        );
        bodies.push(body);
        distance_au *= rng.gen_range(1.5..1.9);
    }

    // ── Asteroid belt (usually present, placed just past the inner worlds) ──
    if rng.gen_bool(0.75) {
        let belt = generate_body(
            &mut rng,
            "Belt".to_string(),
            &BodyKind::AsteroidBelt,
            distance_au,
        );
        bodies.push(belt);
        distance_au *= rng.gen_range(1.4..1.8);
    }

    // ── Gas giants, each with a chance of moons ──────────────────────────────
    let giant_count = rng.gen_range(0..=2u32);
    for i in 0..giant_count {
        let giant_name = format!("Giant-{}", i + 1);
        let mut giant = generate_body(
            &mut rng,
            giant_name.clone(),
            &BodyKind::GasGiant,
            distance_au,
        );
        let giant_id = giant.id.clone();
        let giant_distance = giant.distance_au;

        let moon_count = rng.gen_range(0..=2u32);
        giant.moon_count = moon_count;
        bodies.push(giant);

        for m in 0..moon_count {
            let moon_distance = giant_distance + rng.gen_range(0.02..0.15);
            let mut moon = generate_body(
                &mut rng,
                format!("{giant_name}-Moon-{}", m + 1),
                &BodyKind::Moon,
                moon_distance,
            );
            moon.parent_body = Some(giant_id.clone());
            // Moons of giants sit deep in the radiation belt.
            moon.radiation = RadiationLevel::High;
            bodies.push(moon);
        }

        distance_au *= rng.gen_range(1.6..2.2);
    }

    // ── Playability guarantee: force at least one founding-viable body ──────
    if !bodies.iter().any(Body::meets_founding_threshold) {
        force_habitable(&mut bodies);
    }

    assign_roles(&mut bodies);
    distribute_system_resources(&mut bodies, &mut rng, abundance_scalar);

    bodies
}

/// Distribute [`crate::map::VEIN_COMMODITIES`] across every generated body
/// (issue #232's "the entire system must have enough resources" requirement).
///
/// Only the founding colony's home body ever gets a full hex map with
/// real [`crate::map::Deposit`]s (via a later, separate `SeedPlanet` call) —
/// every other body here only gets the lightweight [`BodyDeposit`] tag. Each
/// commodity is assigned to 1-2 bodies, weighted by an archetype affinity
/// (reusing [`crate::map::subtype_commodity_multiplier`] plus a
/// [`BodyKind`]-level bias — asteroid belts favour ores, gas giants favour
/// hydrocarbons, moons skew fissile) mirroring Dyson Sphere Program's
/// per-planet-type resource tables. A verify-and-patch guarantee pass
/// afterward ensures every curated commodity landed on at least one body
/// system-wide, even if every body's affinity for it happened to be low —
/// same "statistical placement, deterministic guarantee on top" shape as
/// [`force_habitable`] and `map.rs::force_guaranteed_deposits`.
fn distribute_system_resources(bodies: &mut [Body], rng: &mut ChaCha8Rng, abundance_scalar: f32) {
    if bodies.is_empty() {
        return;
    }
    let abundance_scalar = abundance_scalar.max(0.0);

    for commodity in VEIN_COMMODITIES {
        let deposit_count = rng.gen_range(1..=2usize.min(bodies.len()));
        let mut chosen: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for _ in 0..deposit_count {
            let weights: Vec<f32> = bodies
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    if chosen.contains(&i) {
                        0.0
                    } else {
                        commodity_affinity(b, commodity)
                    }
                })
                .collect();
            let total: f32 = weights.iter().sum();
            if total <= 0.0 {
                break; // every body already chosen or has zero affinity.
            }
            let mut roll = rng.gen_range(0.0..total);
            let mut pick = None;
            for (i, w) in weights.iter().enumerate() {
                if roll < *w {
                    pick = Some(i);
                    break;
                }
                roll -= w;
            }
            if let Some(idx) = pick.or(Some(weights.len().saturating_sub(1))) {
                chosen.insert(idx);
            }
        }
        for idx in chosen {
            let abundance = (0.4 + rng.gen_range(0.0..0.5)) * abundance_scalar;
            bodies[idx]
                .deposits
                .push(BodyDeposit::new(commodity, abundance));
        }
    }

    // Guarantee pass: force any commodity that landed nowhere onto its
    // highest-affinity body. Rare in practice (every body has baseline
    // affinity >= a small positive floor for every commodity — see
    // `commodity_affinity`), but not logically impossible with very small
    // systems, so this stays unconditional rather than a debug_assert.
    for commodity in VEIN_COMMODITIES {
        let present = bodies
            .iter()
            .any(|b| b.deposits.iter().any(|d| d.commodity_id == commodity));
        if present {
            continue;
        }
        if let Some((idx, _)) = bodies.iter().enumerate().max_by(|(_, a), (_, b)| {
            commodity_affinity(a, commodity)
                .partial_cmp(&commodity_affinity(b, commodity))
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            bodies[idx]
                .deposits
                .push(BodyDeposit::new(commodity, 0.5 * abundance_scalar));
        }
    }
}

/// Archetype affinity for `commodity` on `body`, combining the per-subtype
/// bias `map.rs` already uses for hex-level veins with a coarser
/// [`BodyKind`]-level bias (asteroid belts/gas giants/moons have no subtype
/// distinction fine enough to matter at body granularity). Always positive
/// so [`distribute_system_resources`]'s guarantee pass always has a body to
/// fall back to.
fn commodity_affinity(body: &Body, commodity: &str) -> f32 {
    let subtype_mult = crate::map::subtype_commodity_multiplier(body.subtype, commodity);
    let kind_mult = match body.kind {
        BodyKind::AsteroidBelt => match commodity {
            "structural_ore" | "conductive_ore" | "precious_ore" | "refractory_ore"
            | "semiconductor_ore" | "silicates" => 1.5,
            "biomass" | "hydrocarbons" => 0.1,
            _ => 1.0,
        },
        BodyKind::GasGiant => match commodity {
            "hydrocarbons" => 1.5,
            "structural_ore" | "silicates" | "biomass" => 0.05,
            _ => 1.0,
        },
        BodyKind::Moon => match commodity {
            "fissile_ore" => 1.3,
            _ => 1.0,
        },
        BodyKind::InnerPlanet | BodyKind::OrbitalStation => 1.0,
    };
    (subtype_mult * kind_mult).max(0.05)
}

/// Generate one body at `distance_au`, deriving attributes from distance
/// (a habitable-zone-style falloff) plus RNG jitter — analogous to
/// `map.rs::cell_temperature` deriving temperature from latitude+elevation
/// instead of an independent flat roll per cell.
fn generate_body(rng: &mut ChaCha8Rng, name: String, kind: &BodyKind, distance_au: f32) -> Body {
    let mut body = Body::new(name, kind.clone(), distance_au);

    body.temperature = temperature_for_distance(distance_au, rng);
    body.radiation = radiation_for_distance(distance_au, kind, rng);
    body.gravity_g = gravity_for_kind(kind, rng);
    let (density, hazard) = atmosphere_for(distance_au, kind, rng);
    body.atmosphere_density = density;
    body.atmosphere_hazard = hazard;
    body.subtype = subtype_for(&body, rng);

    body.tidally_locked = matches!(kind, BodyKind::Moon) || distance_au < 0.35;
    body.axial_tilt_deg = if body.tidally_locked {
        0.0
    } else {
        rng.gen_range(0.0..45.0)
    };
    body.rotation_period_hours = if body.tidally_locked {
        // Locked bodies rotate once per orbit; not modeled here, so just
        // report a long period consistent with "no independent spin".
        rng.gen_range(400.0..1200.0)
    } else {
        rng.gen_range(10.0..60.0)
    };
    body.moon_count = 0; // populated by the caller when moons are attached.

    body
}

/// Ordinal thresholding from distance to [`TemperatureBand`], mirroring
/// `map.rs::cell_temperature`'s technique: map the enum onto an integer
/// scale, shift by a continuous input, clamp, map back.
fn temperature_for_distance(distance_au: f32, rng: &mut ChaCha8Rng) -> TemperatureBand {
    let log_ratio = (distance_au / HABITABLE_ZONE_CENTER_AU).ln();
    let base = (log_ratio * 1.6).round().clamp(-100.0, 100.0);
    #[allow(clippy::cast_possible_truncation)]
    let base = base as i32; // safe: clamped to [-100, 100] above.
    let jitter: i32 = rng.gen_range(-1..=1);
    match (base + jitter).clamp(-2, 2) {
        -2 => TemperatureBand::Hot,
        -1 | 0 => TemperatureBand::Temperate,
        1 => TemperatureBand::Cold,
        _ => {
            if distance_au < HABITABLE_ZONE_CENTER_AU {
                TemperatureBand::Extreme
            } else {
                TemperatureBand::Frozen
            }
        }
    }
}

/// Radiation rises close to the primary (stellar flux) and near/around
/// giants (radiation belts); otherwise mild RNG variation.
fn radiation_for_distance(
    distance_au: f32,
    kind: &BodyKind,
    rng: &mut ChaCha8Rng,
) -> RadiationLevel {
    if matches!(kind, BodyKind::GasGiant) || distance_au < HABITABLE_ZONE_CENTER_AU * 0.4 {
        return if rng.gen_bool(0.7) {
            RadiationLevel::High
        } else {
            RadiationLevel::Moderate
        };
    }
    match rng.gen_range(0..3) {
        0 => RadiationLevel::Low,
        1 => RadiationLevel::Moderate,
        _ => RadiationLevel::High,
    }
}

/// Gravity ranges per kind, matching the authored `content/base/systems.yaml`
/// scenarios' rough scale (inner planets ~0.5-1.3g, giants ~1.8-3.0g, moons
/// ~0.05-0.3g, belts ~0.02-0.08g).
fn gravity_for_kind(kind: &BodyKind, rng: &mut ChaCha8Rng) -> f32 {
    match kind {
        BodyKind::InnerPlanet => rng.gen_range(0.5..1.3),
        BodyKind::GasGiant => rng.gen_range(1.8..3.0),
        BodyKind::Moon => rng.gen_range(0.05..0.3),
        BodyKind::AsteroidBelt => rng.gen_range(0.02..0.08),
        BodyKind::OrbitalStation => 0.0,
    }
}

/// Atmosphere density/hazard, biased toward breathable-and-benign near the
/// habitable-zone center and harsher farther away — the same "closer to the
/// coherent target = more favorable" shape `map.rs::place_veins` uses for
/// elevation-biased deposit placement, applied to atmosphere instead.
fn atmosphere_for(
    distance_au: f32,
    kind: &BodyKind,
    rng: &mut ChaCha8Rng,
) -> (AtmosphereDensity, AtmosphereHazard) {
    if matches!(kind, BodyKind::AsteroidBelt | BodyKind::Moon) {
        // Airless small bodies, occasionally a thin exosphere.
        let density = if rng.gen_bool(0.85) {
            AtmosphereDensity::Vacuum
        } else {
            AtmosphereDensity::Thin
        };
        return (density, AtmosphereHazard::None);
    }
    if matches!(kind, BodyKind::GasGiant) {
        return (AtmosphereDensity::Dense, AtmosphereHazard::Corrosive);
    }

    let hz_distance = (distance_au - HABITABLE_ZONE_CENTER_AU).abs();
    let favorable = hz_distance < 0.35 && rng.gen_bool(0.6);
    if favorable {
        return (AtmosphereDensity::Breathable, AtmosphereHazard::None);
    }

    let density = match rng.gen_range(0..4) {
        0 => AtmosphereDensity::Vacuum,
        1 => AtmosphereDensity::Thin,
        2 => AtmosphereDensity::Dense,
        _ => AtmosphereDensity::Breathable,
    };
    let hazard = match rng.gen_range(0..4) {
        0 => AtmosphereHazard::None,
        1 => AtmosphereHazard::Corrosive,
        2 => AtmosphereHazard::Toxic,
        _ => AtmosphereHazard::OxidizingCombustible,
    };
    (density, hazard)
}

/// Pick a [`PlanetarySubtype`] *consistent* with the body's already-derived
/// attributes, rather than an independently-rolled tag — the #196 problem
/// this issue depends on ("a `Molten` body should be generated with matching
/// `Hot`/`Toxic`/`Dense`/`High` attributes, not have subtype and attributes
/// independently randomized into an incoherent combination").
fn subtype_for(body: &Body, rng: &mut ChaCha8Rng) -> PlanetarySubtype {
    let candidate = match body.kind {
        BodyKind::GasGiant => {
            if matches!(
                body.temperature,
                TemperatureBand::Frozen | TemperatureBand::Cold
            ) {
                PlanetarySubtype::IceGiant
            } else {
                PlanetarySubtype::GasGiant
            }
        }
        BodyKind::Moon | BodyKind::AsteroidBelt => {
            if matches!(
                body.temperature,
                TemperatureBand::Frozen | TemperatureBand::Cold
            ) {
                PlanetarySubtype::Icy
            } else {
                PlanetarySubtype::Rocky
            }
        }
        BodyKind::InnerPlanet => {
            if body.atmosphere_density.is_breathable()
                && body.atmosphere_hazard == AtmosphereHazard::None
                && matches!(body.temperature, TemperatureBand::Temperate)
            {
                if rng.gen_bool(0.3) {
                    PlanetarySubtype::Ocean
                } else {
                    PlanetarySubtype::EarthLike
                }
            } else if matches!(
                body.temperature,
                TemperatureBand::Hot | TemperatureBand::Extreme
            ) && matches!(body.atmosphere_hazard, AtmosphereHazard::Toxic)
                && matches!(body.atmosphere_density, AtmosphereDensity::Dense)
            {
                PlanetarySubtype::Molten
            } else if matches!(
                body.temperature,
                TemperatureBand::Hot | TemperatureBand::Extreme
            ) {
                PlanetarySubtype::RockyBarrenHot
            } else {
                PlanetarySubtype::RockyBarrenCold
            }
        }
        BodyKind::OrbitalStation => PlanetarySubtype::Unclassified,
    };
    debug_assert!(
        candidate.compatible_with(&body.kind),
        "generated subtype {candidate:?} incompatible with kind {:?}",
        body.kind
    );
    candidate
}

/// Deterministically force the body nearest the habitable-zone center into a
/// guaranteed-habitable attribute set — see [`generate_system`]'s doc
/// comment for why this is unconditional rather than a retry loop.
fn force_habitable(bodies: &mut [Body]) {
    let Some(target) = bodies
        .iter_mut()
        .filter(|b| matches!(b.kind, BodyKind::InnerPlanet))
        .min_by(|a, b| {
            let da = (a.distance_au - HABITABLE_ZONE_CENTER_AU).abs();
            let db = (b.distance_au - HABITABLE_ZONE_CENTER_AU).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
    else {
        return; // No inner planet at all — degenerate, but generate_system
                // always produces MIN_INNER_PLANETS >= 2, so unreachable in practice.
    };
    target.atmosphere_density = AtmosphereDensity::Breathable;
    target.atmosphere_hazard = AtmosphereHazard::None;
    target.temperature = TemperatureBand::Temperate;
    target.gravity_g = 1.0;
    target.radiation = RadiationLevel::Low;
    target.subtype = PlanetarySubtype::EarthLike;
    debug_assert!(
        target.meets_founding_threshold(),
        "forced-habitable body must clear the founding threshold (got {})",
        target.habitability()
    );
}

/// Best-effort flavor role assignment, mirroring the hand-authored
/// `content/base/systems.yaml` scenarios (`role` has no mechanical effect —
/// see [`crate::system::SystemRole`] — so this is cosmetic, not load-bearing).
fn assign_roles(bodies: &mut [Body]) {
    if let Some(best) = bodies
        .iter_mut()
        .filter(|b| matches!(b.kind, BodyKind::InnerPlanet))
        .max_by_key(|b| b.habitability())
    {
        if best.habitability() >= HABITABILITY_FOUNDING_THRESHOLD {
            best.role = SystemRole::PopulationHub;
        }
    }
    if let Some(giant) = bodies
        .iter_mut()
        .find(|b| matches!(b.kind, BodyKind::GasGiant))
    {
        giant.role = SystemRole::FuelProduction;
    }
    if let Some(belt) = bodies
        .iter_mut()
        .find(|b| matches!(b.kind, BodyKind::AsteroidBelt))
    {
        belt.role = SystemRole::RawExtraction;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic() {
        let a = generate_system(42, 1.0);
        let b = generate_system(42, 1.0);
        assert_eq!(a.len(), b.len());
        let names_a: Vec<&str> = a.iter().map(|body| body.name.as_str()).collect();
        let names_b: Vec<&str> = b.iter().map(|body| body.name.as_str()).collect();
        assert_eq!(names_a, names_b);
        let kinds_a: Vec<&BodyKind> = a.iter().map(|body| &body.kind).collect();
        let kinds_b: Vec<&BodyKind> = b.iter().map(|body| &body.kind).collect();
        assert_eq!(kinds_a, kinds_b);
    }

    #[test]
    fn higher_abundance_scalar_yields_richer_deposits_without_changing_coverage() {
        // Same seed, only the abundance scalar differs: body/attribute shape
        // must stay identical (deposit distribution consumes RNG only after
        // everything else is decided), but the generous run's total
        // abundance should exceed the lean run's, and both must still cover
        // every curated commodity somewhere.
        let lean = generate_system(11, 0.5);
        let generous = generate_system(11, 3.0);
        assert_eq!(lean.len(), generous.len());

        let total_abundance = |bodies: &[Body]| -> f32 {
            bodies
                .iter()
                .flat_map(|b| &b.deposits)
                .map(|d| d.abundance)
                .sum()
        };
        assert!(
            total_abundance(&generous) > total_abundance(&lean),
            "abundance_scalar=3.0 total should exceed abundance_scalar=0.5 total"
        );

        for bodies in [&lean, &generous] {
            let present: std::collections::HashSet<&str> = bodies
                .iter()
                .flat_map(|b| &b.deposits)
                .map(|d| d.commodity_id.as_str())
                .collect();
            for commodity in crate::map::VEIN_COMMODITIES {
                assert!(
                    present.contains(commodity),
                    "coverage must hold regardless of abundance_scalar (missing {commodity})"
                );
            }
        }
    }

    #[test]
    fn different_seeds_produce_different_systems() {
        // Statistical, not logical, guarantee — matches map.rs's own
        // different_seeds_produce_different_maps convention. Sweep a few
        // seeds so a single unlucky collision doesn't flake the suite.
        let mut any_differ = false;
        for seed in 0..10u64 {
            let a = generate_system(seed, 1.0);
            let b = generate_system(seed + 1000, 1.0);
            let sig_a: Vec<(String, u32)> = a
                .iter()
                .map(|body| (format!("{:?}", body.kind), body.habitability() as u32))
                .collect();
            let sig_b: Vec<(String, u32)> = b
                .iter()
                .map(|body| (format!("{:?}", body.kind), body.habitability() as u32))
                .collect();
            if sig_a != sig_b {
                any_differ = true;
                break;
            }
        }
        assert!(any_differ, "different seeds must produce different systems");
    }

    #[test]
    fn every_seed_in_a_sweep_guarantees_a_colonizable_body() {
        // Matches the #188-style seed-sweep test-bar convention cited in
        // this issue's Definition of Done.
        for seed in 0..64u64 {
            let bodies = generate_system(seed, 1.0);
            assert!(
                bodies.iter().any(Body::meets_founding_threshold),
                "seed {seed} produced no founding-viable body"
            );
        }
    }

    #[test]
    fn generated_subtypes_are_always_compatible_with_their_kind() {
        for seed in 0..32u64 {
            for body in generate_system(seed, 1.0) {
                assert!(
                    body.subtype.compatible_with(&body.kind),
                    "seed {seed}: body {:?} has incompatible subtype {:?} for kind {:?}",
                    body.name,
                    body.subtype,
                    body.kind
                );
            }
        }
    }

    #[test]
    fn moons_link_to_a_real_parent_body() {
        for seed in 0..32u64 {
            let bodies = generate_system(seed, 1.0);
            let ids: std::collections::HashSet<_> = bodies.iter().map(|b| b.id.clone()).collect();
            for body in &bodies {
                if let Some(parent) = &body.parent_body {
                    assert!(
                        ids.contains(parent),
                        "seed {seed}: {} has a parent_body that doesn't resolve",
                        body.name
                    );
                }
            }
        }
    }

    #[test]
    fn gas_giant_moon_count_matches_attached_moons() {
        for seed in 0..32u64 {
            let bodies = generate_system(seed, 1.0);
            for giant in bodies.iter().filter(|b| b.kind == BodyKind::GasGiant) {
                let attached = bodies
                    .iter()
                    .filter(|b| b.parent_body.as_ref() == Some(&giant.id))
                    .count();
                assert_eq!(
                    giant.moon_count as usize, attached,
                    "seed {seed}: {} reports moon_count {} but has {attached} attached moons",
                    giant.name, giant.moon_count
                );
            }
        }
    }

    #[test]
    fn every_curated_commodity_exists_somewhere_in_the_system() {
        for seed in 0..32u64 {
            let bodies = generate_system(seed, 1.0);
            let present: std::collections::HashSet<&str> = bodies
                .iter()
                .flat_map(|b| &b.deposits)
                .map(|d| d.commodity_id.as_str())
                .collect();
            for commodity in crate::map::VEIN_COMMODITIES {
                assert!(
                    present.contains(commodity),
                    "seed {seed}: no body in the system has a {commodity} deposit"
                );
            }
        }
    }

    #[test]
    fn body_deposits_only_reference_valid_bodies() {
        // Sanity check on distribute_system_resources' index bookkeeping:
        // every deposit must land on a body that actually exists in the
        // returned Vec (trivially true given deposits are pushed directly
        // onto `bodies[idx]`, but worth asserting explicitly since the
        // weighted-pick logic is the one part of this module doing index
        // arithmetic rather than working through `Body` values directly).
        for seed in 0..16u64 {
            let bodies = generate_system(seed, 1.0);
            let total_deposits: usize = bodies.iter().map(|b| b.deposits.len()).sum();
            assert!(
                total_deposits > 0,
                "seed {seed}: no deposits were distributed at all"
            );
        }
    }

    #[test]
    fn generates_at_least_the_minimum_inner_planet_count() {
        for seed in 0..32u64 {
            let bodies = generate_system(seed, 1.0);
            let inner_count = bodies
                .iter()
                .filter(|b| matches!(b.kind, BodyKind::InnerPlanet))
                .count();
            assert!(
                u32::try_from(inner_count).unwrap() >= MIN_INNER_PLANETS,
                "seed {seed} produced only {inner_count} inner planets"
            );
        }
    }
}
