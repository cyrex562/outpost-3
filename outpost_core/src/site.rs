//! Evaluating a building's authored [`SiteRequirement`]s against a real site
//! (issue #410).
//!
//! Separate from `content::types`, where the requirement is *declared*,
//! because answering it needs the map and system types — and separate from
//! the command handlers, because the same answer is wanted in two places: the
//! engine rejecting a build, and the UI greying it out before the player
//! tries.
//!
//! # What a "site" is
//!
//! A colony sits on a hex of a body's [`PlanetMap`], so every requirement can
//! be answered for one. An outpost is anchored to a body with **no surface
//! hex** (see [`crate::outpost::Outpost`]), so hex-scoped requirements have no
//! meaningful answer there — [`SiteContext::coord`] is `None` and those
//! requirements report unmet rather than being waved through. Refusing is the
//! conservative reading: the engine cannot show the condition holds, and a
//! permissive default would let an outpost build something that depends on
//! terrain nobody has checked. If outposts ever gain a hex, they start
//! evaluating correctly with no change here.

use crate::content::types::{SiteCondition, SiteProperty, SiteRequirement, SiteScaling};
use crate::map::{HexCoord, PlanetMap};
use crate::system::Body;

/// Insolation treated as the bottom of the usable range, in units where Sol
/// at 1 AU is `1.0` (issue #415).
///
/// Roughly a body at 14 AU. Anything dimmer normalises to `0.0` rather than
/// running off toward negative infinity on the log scale.
pub const INSOLATION_FLOOR: f32 = 0.005;

/// Insolation treated as the top of the usable range (issue #415).
///
/// `4.0` is about 0.5 AU from a Sol-like star. Brighter sites clamp here:
/// past this point more light stops meaning more usable power, and letting
/// the curve keep climbing would make a scorched inner planet the best solar
/// site in the game despite being nearly uninhabitable.
pub const INSOLATION_CEILING: f32 = 4.0;

/// Where a building is being placed, as far as its requirements care.
#[derive(Debug, Clone, Copy)]
pub struct SiteContext<'a> {
    /// The body's surface map, if one has been generated.
    pub map: Option<&'a PlanetMap>,
    /// The site's own hex. `None` for a site with no surface position.
    pub coord: Option<HexCoord>,
    /// The body the site is on, if known.
    pub body: Option<&'a Body>,
    /// Technologies already researched, for requirements a tech can waive
    /// (issue #414). `None` means nothing is researched.
    pub researched: Option<&'a std::collections::HashSet<crate::tech::TechId>>,
    /// Starlight reaching this site, where Sol at 1 AU is `1.0` (issue #413).
    ///
    /// Supplied by the caller rather than derived here: insolation is a
    /// property of the body's orbit and its star, neither of which this
    /// context holds.
    pub insolation: Option<f32>,
    /// Bulk-water circulation at this body, already normalised to `0.0`–`1.0`
    /// (issue #440).
    ///
    /// Supplied by the caller for the same reason as `insolation`, and more
    /// so: deriving it needs the body's **parent**, which this context does
    /// not hold. See
    /// [`crate::system::SystemNodeMap::ocean_circulation_for`].
    pub ocean_circulation: Option<f32>,
}

impl<'a> SiteContext<'a> {
    /// A context that can answer nothing — every hex- or body-scoped
    /// requirement reports unmet.
    #[must_use]
    pub fn unknown() -> Self {
        Self {
            map: None,
            coord: None,
            body: None,
            researched: None,
            insolation: None,
            ocean_circulation: None,
        }
    }

    /// Hexes within `radius` of the site, wrapped into storable coordinates.
    ///
    /// `PlanetMap::cell` is a plain map lookup and does **not** wrap, while
    /// `within_radius` happily produces `q` values off either edge — so every
    /// candidate goes through `wrap_coord` first, or a requirement would
    /// silently fail to see a hex just across the east-west seam.
    fn cells_in_range(&self, radius: u32) -> Vec<&'a crate::map::HexCell> {
        let (Some(map), Some(centre)) = (self.map, self.coord) else {
            return Vec::new();
        };
        centre
            .within_radius(radius)
            .into_iter()
            .filter_map(|c| map.cell(map.wrap_coord(c)))
            .collect()
    }

    /// Whether `req` holds at this site, or is waived by a researched tech.
    #[must_use]
    pub fn satisfies(&self, req: &SiteRequirement) -> bool {
        if let Some(tech) = &req.waived_by_tech {
            if self
                .researched
                .is_some_and(|set| set.contains(tech.as_str()))
            {
                return true;
            }
        }
        self.meets(&req.condition)
    }

    /// Whether the site meets `condition`, ignoring any tech waiver.
    #[must_use]
    pub fn meets(&self, condition: &SiteCondition) -> bool {
        match condition {
            SiteCondition::Terrain {
                any_of,
                within_hexes,
            } => self
                .cells_in_range(*within_hexes)
                .iter()
                .any(|cell| any_of.contains(&cell.terrain)),
            SiteCondition::Deposit {
                commodity,
                within_hexes,
            } => self
                .cells_in_range(*within_hexes)
                .iter()
                .any(|cell| cell.deposits.iter().any(|d| &d.commodity_id == commodity)),
            SiteCondition::MinAtmosphere { density } => self
                .body
                .is_some_and(|b| b.atmosphere_density.rank() >= density.rank()),
            SiteCondition::MinGeothermalGradient { min } => self
                .own_cell()
                .is_some_and(|cell| cell.geothermal_gradient >= *min),
        }
    }

    /// A normalised `[0.0, 1.0]` reading of `property` at this site, or `None`
    /// when the site cannot answer it.
    #[must_use]
    pub fn read(&self, property: &SiteProperty) -> Option<f64> {
        match property {
            SiteProperty::DepositRichness { commodity } => {
                let cell = self.own_cell()?;
                Some(f64::from(
                    cell.deposits
                        .iter()
                        .filter(|d| &d.commodity_id == commodity)
                        .map(|d| d.richness)
                        .fold(0.0_f32, f32::max),
                ))
            }
            SiteProperty::AtmosphereDensity => {
                let body = self.body?;
                // Rank 0..=3 normalised onto 0..=1.
                Some(f64::from(body.atmosphere_density.rank()) / 3.0)
            }
            SiteProperty::Elevation => Some(f64::from(self.own_cell()?.elevation)),
            SiteProperty::GeothermalGradient => {
                Some(f64::from(self.own_cell()?.geothermal_gradient))
            }
            SiteProperty::Insolation => Some(f64::from(normalise_insolation(self.insolation?))),
            // Already normalised by its source, unlike insolation — the
            // mapping needs the parent body, so it happens where the parent
            // is reachable rather than here.
            SiteProperty::OceanCirculation => Some(f64::from(self.ocean_circulation?)),
        }
    }

    /// The output multiplier this site implies for `scaling`.
    ///
    /// `1.0` when the building declares no scaling, and also when the site
    /// cannot answer the property. The second case is deliberately neutral
    /// rather than zero: an unknown site should leave a building performing
    /// exactly as it did before this mechanism existed, not silently produce
    /// nothing. That is the opposite of the choice made for site
    /// *requirements*, where an unanswerable condition refuses — a
    /// requirement that cannot be shown to hold is a reason not to build,
    /// while an unreadable property is simply no information.
    #[must_use]
    pub fn output_multiplier(&self, scaling: Option<&SiteScaling>) -> f64 {
        let Some(scaling) = scaling else { return 1.0 };
        match self.read(&scaling.property) {
            Some(reading) => scaling.multiplier_at(reading),
            None => 1.0,
        }
    }

    /// The site's own hex, if it has one.
    fn own_cell(&self) -> Option<&'a crate::map::HexCell> {
        let (map, coord) = (self.map?, self.coord?);
        map.cell(map.wrap_coord(coord))
    }

    /// Every requirement in `reqs` this site fails, in authored order.
    ///
    /// Returns all of them rather than short-circuiting on the first: a site
    /// short of two conditions should say so, the same reasoning behind the
    /// per-requirement build badges (issue #423).
    #[must_use]
    pub fn unmet<'r>(&self, reqs: &'r [SiteRequirement]) -> Vec<&'r SiteRequirement> {
        reqs.iter().filter(|r| !self.satisfies(r)).collect()
    }
}

/// Map raw insolation onto the `[0.0, 1.0]` reading [`SiteScaling`] expects
/// (issue #415).
///
/// **Logarithmic, deliberately.** Insolation is inverse-square in the world,
/// which spans roughly 3500-fold between an inner planet and an outer moon —
/// applied linearly to output, an 18 AU colony would get 0.3% of an inner
/// one's solar power, which does not make solar a tradeoff so much as delete
/// it past about 2 AU.
///
/// A log scale keeps the *ordering* honest — nearer is always better, by a
/// lot — while leaving a distant colony with solar panels that do something.
/// This is a game-feel decision, not a physical one, and it is written down
/// here so nobody later "fixes" it back to inverse-square: the physics is
/// already correct in [`crate::system::Star::insolation_at`]; this is the
/// separate question of how much that should matter to a building's yield.
fn normalise_insolation(insolation: f32) -> f32 {
    let clamped = insolation.clamp(INSOLATION_FLOOR, INSOLATION_CEILING);
    let lo = INSOLATION_FLOOR.log10();
    let hi = INSOLATION_CEILING.log10();
    ((clamped.log10() - lo) / (hi - lo)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Biome, Deposit, HexCell, Terrain};
    use crate::system::{AtmosphereDensity, AtmosphereHazard, Body, BodyKind};

    /// An 8x8 map whose listed cells are overwritten; anything unlisted keeps
    /// whatever `generate` produced, which the assertions never depend on.
    fn map_with(cells: &[(i32, i32, Terrain, &[&str])]) -> PlanetMap {
        let mut map = PlanetMap::generate(1, 8, 8);
        for (q, r, terrain, deposits) in cells {
            let coord = HexCoord::new(*q, *r);
            let mut cell = HexCell::new(coord, *terrain, Biome::Desert);
            cell.deposits = deposits
                .iter()
                .map(|id| Deposit::new((*id).to_string(), 0.5))
                .collect();
            map.cells.insert(coord, cell);
        }
        map
    }

    fn body_with(density: AtmosphereDensity) -> Body {
        let mut b = Body::new("Test", BodyKind::InnerPlanet, 1.0);
        b.atmosphere_density = density;
        b.atmosphere_hazard = AtmosphereHazard::None;
        b
    }

    fn ctx<'a>(map: &'a PlanetMap, q: i32, r: i32, body: Option<&'a Body>) -> SiteContext<'a> {
        SiteContext {
            map: Some(map),
            coord: Some(HexCoord::new(q, r)),
            body,
            researched: None,
            insolation: None,
            ocean_circulation: None,
        }
    }

    #[test]
    fn terrain_on_the_site_itself_satisfies_a_zero_radius_requirement() {
        let map = map_with(&[(0, 0, Terrain::Volcanic, &[])]);
        let req = SiteRequirement::new(SiteCondition::Terrain {
            any_of: vec![Terrain::Volcanic],
            within_hexes: 0,
        });
        assert!(ctx(&map, 0, 0, None).satisfies(&req));
    }

    #[test]
    fn terrain_one_hex_away_needs_the_radius_to_reach_it() {
        let map = map_with(&[(0, 0, Terrain::Plains, &[]), (1, 0, Terrain::Ocean, &[])]);
        let ocean = |within_hexes| {
            SiteRequirement::new(SiteCondition::Terrain {
                any_of: vec![Terrain::Ocean],
                within_hexes,
            })
        };
        assert!(!ctx(&map, 0, 0, None).satisfies(&ocean(0)));
        assert!(ctx(&map, 0, 0, None).satisfies(&ocean(1)));
    }

    #[test]
    fn any_of_is_satisfied_by_any_listed_terrain() {
        let map = map_with(&[(0, 0, Terrain::Wetlands, &[])]);
        let req = SiteRequirement::new(SiteCondition::Terrain {
            any_of: vec![Terrain::Ocean, Terrain::Wetlands],
            within_hexes: 0,
        });
        assert!(ctx(&map, 0, 0, None).satisfies(&req));
    }

    #[test]
    fn a_deposit_in_range_satisfies_the_requirement() {
        let map = map_with(&[
            (0, 0, Terrain::Plains, &[]),
            (1, 0, Terrain::Plains, &["hydrocarbons"]),
        ]);
        let req = |within_hexes| {
            SiteRequirement::new(SiteCondition::Deposit {
                commodity: "hydrocarbons".into(),
                within_hexes,
            })
        };
        assert!(!ctx(&map, 0, 0, None).satisfies(&req(0)));
        assert!(ctx(&map, 0, 0, None).satisfies(&req(1)));
    }

    #[test]
    fn a_different_commodity_does_not_satisfy_a_deposit_requirement() {
        let map = map_with(&[(0, 0, Terrain::Plains, &["silicates"])]);
        let req = SiteRequirement::new(SiteCondition::Deposit {
            commodity: "hydrocarbons".into(),
            within_hexes: 1,
        });
        assert!(!ctx(&map, 0, 0, None).satisfies(&req));
    }

    #[test]
    fn a_hex_across_the_east_west_seam_is_still_in_range() {
        // `within_radius` produces q = -1 for a site at q = 0; the map stores
        // that column as q = width - 1. Without wrapping, the requirement
        // would not see it.
        let map = map_with(&[(0, 0, Terrain::Plains, &[]), (7, 0, Terrain::Ocean, &[])]);
        let req = SiteRequirement::new(SiteCondition::Terrain {
            any_of: vec![Terrain::Ocean],
            within_hexes: 1,
        });
        assert!(
            ctx(&map, 0, 0, None).satisfies(&req),
            "the map wraps east-west, so q=7 is adjacent to q=0"
        );
    }

    #[test]
    fn atmosphere_is_met_at_or_above_the_required_density() {
        let map = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let req = SiteRequirement::new(SiteCondition::MinAtmosphere {
            density: AtmosphereDensity::Thin,
        });
        for (density, expected) in [
            (AtmosphereDensity::Vacuum, false),
            (AtmosphereDensity::Thin, true),
            (AtmosphereDensity::Breathable, true),
            (AtmosphereDensity::Dense, true),
        ] {
            let body = body_with(density);
            assert_eq!(
                ctx(&map, 0, 0, Some(&body)).satisfies(&req),
                expected,
                "{density:?} against a Thin minimum"
            );
        }
    }

    #[test]
    fn a_site_with_no_surface_hex_fails_hex_scoped_requirements() {
        // An outpost is anchored to a body without a hex. Refusing is the
        // conservative reading — the engine cannot show the condition holds.
        let body = body_with(AtmosphereDensity::Dense);
        let ctx = SiteContext {
            map: None,
            coord: None,
            body: Some(&body),
            researched: None,
            insolation: None,
            ocean_circulation: None,
        };

        assert!(
            !ctx.satisfies(&SiteRequirement::new(SiteCondition::Terrain {
                any_of: vec![Terrain::Ocean],
                within_hexes: 2,
            }))
        );
        assert!(
            !ctx.satisfies(&SiteRequirement::new(SiteCondition::Deposit {
                commodity: "hydrocarbons".into(),
                within_hexes: 2,
            }))
        );
        // ...but a body-scoped one still answers.
        assert!(
            ctx.satisfies(&SiteRequirement::new(SiteCondition::MinAtmosphere {
                density: AtmosphereDensity::Thin,
            }))
        );
    }

    #[test]
    fn unmet_reports_every_failure_not_just_the_first() {
        let map = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let body = body_with(AtmosphereDensity::Vacuum);
        let reqs = vec![
            SiteRequirement::new(SiteCondition::Terrain {
                any_of: vec![Terrain::Ocean],
                within_hexes: 0,
            }),
            SiteRequirement::new(SiteCondition::Deposit {
                commodity: "hydrocarbons".into(),
                within_hexes: 0,
            }),
            SiteRequirement::new(SiteCondition::MinAtmosphere {
                density: AtmosphereDensity::Thin,
            }),
        ];
        assert_eq!(ctx(&map, 0, 0, Some(&body)).unmet(&reqs).len(), 3);
    }

    #[test]
    fn an_empty_requirement_list_is_always_satisfied() {
        assert!(SiteContext::unknown().unmet(&[]).is_empty());
    }

    #[test]
    fn describe_reads_as_the_condition_rather_than_a_failure() {
        assert_eq!(
            SiteRequirement::new(SiteCondition::Terrain {
                any_of: vec![Terrain::Ocean],
                within_hexes: 2
            })
            .describe(),
            "ocean within 2 hexes"
        );
        assert_eq!(
            SiteRequirement::new(SiteCondition::Deposit {
                commodity: "hydrocarbons".into(),
                within_hexes: 1
            })
            .describe(),
            "hydrocarbons deposit within 1 hex"
        );
        assert_eq!(
            SiteRequirement::new(SiteCondition::MinAtmosphere {
                density: AtmosphereDensity::Thin
            })
            .describe(),
            "thin atmosphere or denser"
        );
    }

    // ── Output scaling (issue #411) ─────────────────────────────────────────

    fn scaling(property: SiteProperty, at_min: f64, at_max: f64) -> SiteScaling {
        SiteScaling {
            property,
            at_min,
            at_max,
        }
    }

    #[test]
    fn deposit_richness_drives_the_multiplier_between_the_authored_endpoints() {
        let lean = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let rich = {
            let mut m = map_with(&[(0, 0, Terrain::Plains, &[])]);
            let coord = HexCoord::new(0, 0);
            let mut cell = HexCell::new(coord, Terrain::Plains, Biome::Desert);
            cell.deposits = vec![Deposit::new("hydrocarbons", 1.0)];
            m.cells.insert(coord, cell);
            m
        };
        let s = scaling(
            SiteProperty::DepositRichness {
                commodity: "hydrocarbons".into(),
            },
            0.25,
            1.5,
        );

        assert!((ctx(&lean, 0, 0, None).output_multiplier(Some(&s)) - 0.25).abs() < 1e-9);
        assert!((ctx(&rich, 0, 0, None).output_multiplier(Some(&s)) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn a_mid_range_reading_lands_between_the_endpoints() {
        let mut m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let coord = HexCoord::new(0, 0);
        let mut cell = HexCell::new(coord, Terrain::Plains, Biome::Desert);
        cell.deposits = vec![Deposit::new("ore", 0.5)];
        m.cells.insert(coord, cell);

        let s = scaling(
            SiteProperty::DepositRichness {
                commodity: "ore".into(),
            },
            0.0,
            2.0,
        );
        assert!((ctx(&m, 0, 0, None).output_multiplier(Some(&s)) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_descending_curve_is_allowed() {
        // Nothing assumes the relationship is positive — some outputs should
        // fall as a property rises.
        let mut m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let coord = HexCoord::new(0, 0);
        let mut cell = HexCell::new(coord, Terrain::Plains, Biome::Desert);
        cell.deposits = vec![Deposit::new("ore", 1.0)];
        m.cells.insert(coord, cell);

        let s = scaling(
            SiteProperty::DepositRichness {
                commodity: "ore".into(),
            },
            2.0,
            0.5,
        );
        assert!((ctx(&m, 0, 0, None).output_multiplier(Some(&s)) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn atmosphere_density_reads_across_its_whole_range() {
        let m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let s = scaling(SiteProperty::AtmosphereDensity, 0.0, 3.0);
        for (density, expected) in [
            (AtmosphereDensity::Vacuum, 0.0),
            (AtmosphereDensity::Dense, 3.0),
        ] {
            let body = body_with(density);
            let got = ctx(&m, 0, 0, Some(&body)).output_multiplier(Some(&s));
            assert!((got - expected).abs() < 1e-9, "{density:?} gave {got}");
        }
        // Thin is one third of the way up the rank scale.
        let thin = body_with(AtmosphereDensity::Thin);
        let got = ctx(&m, 0, 0, Some(&thin)).output_multiplier(Some(&s));
        assert!((got - 1.0).abs() < 1e-9, "Thin gave {got}");
    }

    /// The current plant's authored curve, end to end through a site (issue
    /// #440) — a lively sea must out-produce a dead one by the authored
    /// margin, not merely by "more".
    #[test]
    fn ocean_circulation_drives_the_multiplier_between_the_authored_endpoints() {
        let s = scaling(SiteProperty::OceanCirculation, 0.75, 1.25);
        let at = |reading: f32| {
            let mut ctx = SiteContext::unknown();
            ctx.ocean_circulation = Some(reading);
            ctx.output_multiplier(Some(&s))
        };

        // A tidally locked planet, and a strongly tide-driven moon.
        assert!((at(0.0) - 0.75).abs() < 1e-9, "dead sea: {}", at(0.0));
        assert!((at(1.0) - 1.25).abs() < 1e-9, "best case: {}", at(1.0));
        // The measured median moon sits just below neutral.
        assert!(
            at(0.442) > at(0.013),
            "a tide-driven moon must beat a locked planet: {} vs {}",
            at(0.442),
            at(0.013)
        );
    }

    /// A body whose circulation cannot be read leaves the plant unscaled
    /// rather than dead — the same neutral-on-unknown rule every other
    /// property follows.
    #[test]
    fn a_site_that_cannot_report_circulation_is_neutral() {
        let s = scaling(SiteProperty::OceanCirculation, 0.75, 1.25);
        assert!((SiteContext::unknown().output_multiplier(Some(&s)) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_building_with_no_scaling_is_neutral() {
        assert!((SiteContext::unknown().output_multiplier(None) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn an_unreadable_site_is_neutral_rather_than_zero() {
        // The opposite of the choice made for site *requirements*: a condition
        // that cannot be shown to hold is a reason not to build, but a
        // property that cannot be read is simply no information, and must
        // leave the building performing as it did before scaling existed.
        let s = scaling(
            SiteProperty::DepositRichness {
                commodity: "ore".into(),
            },
            0.0,
            2.0,
        );
        assert!((SiteContext::unknown().output_multiplier(Some(&s)) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_reading_outside_the_normalised_range_is_clamped_not_extrapolated() {
        // A property implementation returning out-of-range is a bug; silently
        // extrapolating would turn it into a wildly wrong yield instead of a
        // merely capped one.
        let s = scaling(SiteProperty::Elevation, 1.0, 2.0);
        assert!((s.multiplier_at(5.0) - 2.0).abs() < 1e-9);
        assert!((s.multiplier_at(-3.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_multiplier_never_goes_negative() {
        let s = scaling(SiteProperty::Elevation, -4.0, -1.0);
        assert!(s.multiplier_at(0.0) >= 0.0);
        assert!(s.multiplier_at(1.0) >= 0.0);
    }

    // ── Tech-waived requirements + geothermal (issue #414) ──────────────────

    fn researched(ids: &[&str]) -> std::collections::HashSet<crate::tech::TechId> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_gradient_requirement_reads_the_hex() {
        let mut m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let coord = HexCoord::new(0, 0);
        let mut cell = HexCell::new(coord, Terrain::Plains, Biome::Desert);
        cell.geothermal_gradient = 0.35;
        m.cells.insert(coord, cell);

        let req = |min| SiteRequirement::new(SiteCondition::MinGeothermalGradient { min });
        assert!(ctx(&m, 0, 0, None).satisfies(&req(0.2)));
        assert!(!ctx(&m, 0, 0, None).satisfies(&req(0.6)));
    }

    #[test]
    fn a_tech_waives_a_requirement_the_site_does_not_meet() {
        // The conditional gate issue #414 needed: the *site* decides whether
        // the tech is required, which `tech_prerequisite` cannot express.
        let mut m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let coord = HexCoord::new(0, 0);
        let mut cell = HexCell::new(coord, Terrain::Plains, Biome::Desert);
        cell.geothermal_gradient = 0.05;
        m.cells.insert(coord, cell);

        let req = SiteRequirement::waivable(
            SiteCondition::MinGeothermalGradient { min: 0.2 },
            "deep_drilling",
        );

        let without = SiteContext {
            researched: None,
            ..ctx(&m, 0, 0, None)
        };
        assert!(
            !without.satisfies(&req),
            "cold site must refuse without the tech"
        );

        let techs = researched(&["deep_drilling"]);
        let with = SiteContext {
            researched: Some(&techs),
            ..ctx(&m, 0, 0, None)
        };
        assert!(with.satisfies(&req), "the tech must lift it");
    }

    #[test]
    fn an_unrelated_tech_does_not_waive_a_requirement() {
        let m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let req = SiteRequirement::waivable(
            SiteCondition::Terrain {
                any_of: vec![Terrain::Ocean],
                within_hexes: 0,
            },
            "deep_drilling",
        );
        let techs = researched(&["automation", "fusion_basics"]);
        let c = SiteContext {
            researched: Some(&techs),
            ..ctx(&m, 0, 0, None)
        };
        assert!(!c.satisfies(&req));
    }

    #[test]
    fn a_site_that_already_meets_a_waivable_requirement_needs_no_tech() {
        let mut m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let coord = HexCoord::new(0, 0);
        let mut cell = HexCell::new(coord, Terrain::Plains, Biome::Desert);
        cell.geothermal_gradient = 0.9;
        m.cells.insert(coord, cell);

        let req = SiteRequirement::waivable(
            SiteCondition::MinGeothermalGradient { min: 0.2 },
            "deep_drilling",
        );
        assert!(ctx(&m, 0, 0, None).satisfies(&req));
    }

    #[test]
    fn describe_names_the_waiving_tech_so_it_reads_as_a_choice() {
        let req = SiteRequirement::waivable(
            SiteCondition::MinGeothermalGradient { min: 0.2 },
            "deep_drilling",
        );
        let text = req.describe();
        assert!(text.contains("20%"), "{text}");
        assert!(text.contains("deep_drilling"), "{text}");
    }

    #[test]
    fn output_scales_with_the_geothermal_gradient() {
        let build = |g: f32| {
            let mut m = map_with(&[(0, 0, Terrain::Plains, &[])]);
            let coord = HexCoord::new(0, 0);
            let mut cell = HexCell::new(coord, Terrain::Plains, Biome::Desert);
            cell.geothermal_gradient = g;
            m.cells.insert(coord, cell);
            m
        };
        // The authored curve from content/base/buildings.yaml.
        let s = SiteScaling {
            property: SiteProperty::GeothermalGradient,
            at_min: 0.15,
            at_max: 1.4,
        };
        let cold = build(0.2);
        let hot = build(0.85);
        let cold_m = ctx(&cold, 0, 0, None).output_multiplier(Some(&s));
        let hot_m = ctx(&hot, 0, 0, None).output_multiplier(Some(&s));

        assert!(cold_m < 0.5, "cold site multiplier {cold_m}");
        assert!(hot_m > 1.1, "hot site multiplier {hot_m}");
        assert!(
            hot_m > cold_m * 2.5,
            "{hot_m} vs {cold_m} — too flat to matter"
        );
    }

    // ── Insolation-scaled output (issue #415) ───────────────────────────────

    fn solar_curve() -> SiteScaling {
        // The authored curve from content/base/buildings.yaml.
        SiteScaling {
            property: SiteProperty::Insolation,
            at_min: 0.12,
            at_max: 1.23,
        }
    }

    fn solar_multiplier_at(insolation: f32) -> f64 {
        let m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        SiteContext {
            insolation: Some(insolation),
            ..ctx(&m, 0, 0, None)
        }
        .output_multiplier(Some(&solar_curve()))
    }

    #[test]
    fn solar_output_falls_with_distance_from_the_star() {
        // insolation = 1 / au², so these are 0.5 / 1 / 5 / 18 AU.
        let near = solar_multiplier_at(4.0);
        let earth = solar_multiplier_at(1.0);
        let far = solar_multiplier_at(0.04);
        let very_far = solar_multiplier_at(0.003);

        assert!(near > earth, "{near} vs {earth}");
        assert!(earth > far, "{earth} vs {far}");
        assert!(far > very_far, "{far} vs {very_far}");
    }

    #[test]
    fn a_sol_like_body_at_one_au_gets_exactly_nominal_output() {
        // Preserves the ladder issue #427 tuned: solar's authored numbers
        // were calibrated at this point, so it has to land on 1.0 or every
        // one of those comparisons silently shifts.
        let m = solar_multiplier_at(1.0);
        assert!((m - 1.0).abs() < 0.02, "1 AU multiplier {m}");
    }

    #[test]
    fn the_falloff_is_softened_rather_than_inverse_square() {
        // The game-feel decision the content comment records. Applied
        // linearly, 18 AU would be 0.3% of 1 AU; the log mapping keeps a
        // distant colony's panels doing something.
        let earth = solar_multiplier_at(1.0);
        let very_far = solar_multiplier_at(0.003);
        let raw_ratio = 0.003 / 1.0;
        let curve_ratio = very_far / earth;

        assert!(
            curve_ratio > raw_ratio * 20.0,
            "curve ratio {curve_ratio} is barely softer than the raw {raw_ratio}"
        );
        assert!(
            very_far > 0.1,
            "a distant colony's panels must still do something, got {very_far}"
        );
    }

    #[test]
    fn a_scorching_inner_body_does_not_run_away_with_the_curve() {
        // Clamped at the ceiling: past ~0.5 AU more light stops meaning more
        // usable power, and an unclamped curve would make a nearly
        // uninhabitable inner planet the best solar site in the game.
        let at_ceiling = solar_multiplier_at(4.0);
        let far_past = solar_multiplier_at(50.0);
        assert!((at_ceiling - far_past).abs() < 1e-9);
    }

    #[test]
    fn an_outer_system_colony_has_a_viable_non_solar_power_path() {
        // The knock-on issue #415 asks to check: if solar becomes weak past
        // some distance, the outer system must not be a dead end at tech 0.
        // Geothermal (issue #414) is the answer — its output depends on the
        // ground, not the star.
        let mut m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let coord = HexCoord::new(0, 0);
        let mut cell = HexCell::new(coord, Terrain::Plains, Biome::Desert);
        cell.geothermal_gradient = 0.5; // ordinary crust, not a lucky hotspot
        m.cells.insert(coord, cell);

        let far = SiteContext {
            insolation: Some(0.01), // ~10 AU
            ..ctx(&m, 0, 0, None)
        };

        // Nominal power per slot from content: solar mk1 24, geothermal 36.
        let solar = 24.0 * far.output_multiplier(Some(&solar_curve()));
        let geothermal = 36.0
            * far.output_multiplier(Some(&SiteScaling {
                property: SiteProperty::GeothermalGradient,
                at_min: 0.15,
                at_max: 1.4,
            }));

        assert!(
            geothermal > solar * 3.0,
            "at 10 AU geothermal gives {geothermal:.1} against solar's {solar:.1} — \
             the outer system needs a clearly better option than weak sunlight"
        );
    }

    #[test]
    fn a_site_with_no_known_insolation_is_neutral() {
        let m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let c = SiteContext {
            insolation: None,
            ..ctx(&m, 0, 0, None)
        };
        assert!((c.output_multiplier(Some(&solar_curve())) - 1.0).abs() < 1e-9);
    }

    // ── Wind: atmosphere gate and density scaling (issue #416) ──────────────

    fn wind_curve() -> SiteScaling {
        // The authored curve from content/base/buildings.yaml.
        SiteScaling {
            property: SiteProperty::AtmosphereDensity,
            at_min: 0.15,
            at_max: 1.5,
        }
    }

    fn wind_multiplier_on(density: AtmosphereDensity) -> f64 {
        let m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let body = body_with(density);
        ctx(&m, 0, 0, Some(&body)).output_multiplier(Some(&wind_curve()))
    }

    #[test]
    fn a_turbine_is_refused_on_an_airless_body() {
        // The point of the requirement: a turbine on an airless moon is
        // nonsense, and roughly two thirds of foundable bodies are vacuum.
        let m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let req = SiteRequirement::new(SiteCondition::MinAtmosphere {
            density: AtmosphereDensity::Thin,
        });
        let vacuum = body_with(AtmosphereDensity::Vacuum);
        assert!(!ctx(&m, 0, 0, Some(&vacuum)).satisfies(&req));
    }

    #[test]
    fn a_turbine_is_allowed_on_any_body_with_air() {
        let m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let req = SiteRequirement::new(SiteCondition::MinAtmosphere {
            density: AtmosphereDensity::Thin,
        });
        for density in [
            AtmosphereDensity::Thin,
            AtmosphereDensity::Breathable,
            AtmosphereDensity::Dense,
        ] {
            let body = body_with(density);
            assert!(
                ctx(&m, 0, 0, Some(&body)).satisfies(&req),
                "{density:?} must allow a turbine"
            );
        }
    }

    #[test]
    fn output_rises_with_atmospheric_density() {
        // Not a binary present/absent: a thin atmosphere is a poor site, a
        // dense one is a good one.
        let thin = wind_multiplier_on(AtmosphereDensity::Thin);
        let breathable = wind_multiplier_on(AtmosphereDensity::Breathable);
        let dense = wind_multiplier_on(AtmosphereDensity::Dense);

        assert!(thin < breathable, "{thin} vs {breathable}");
        assert!(breathable < dense, "{breathable} vs {dense}");
        assert!(
            dense > thin * 2.0,
            "density barely matters: thin {thin}, dense {dense}"
        );
    }

    #[test]
    fn a_thin_atmosphere_still_leaves_a_turbine_worth_building() {
        // Thin is the most common atmosphere among foundable bodies, so it
        // has to be a genuine option rather than a token.
        let thin = wind_multiplier_on(AtmosphereDensity::Thin);
        // Nominal 21 power from content; anything under a few power per slot
        // would not be worth the slot at all.
        assert!(
            21.0 * thin > 10.0,
            "thin yields only {:.1} power",
            21.0 * thin
        );
    }

    #[test]
    fn wind_beats_solar_where_the_air_is_thick_and_the_sun_is_far() {
        // The niche issue #409 promised and #415/#416 deliver: solar fades
        // with distance, wind does not care about distance at all.
        let m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let dense = body_with(AtmosphereDensity::Dense);
        let far_dense = SiteContext {
            insolation: Some(0.04), // ~5 AU
            ..ctx(&m, 0, 0, Some(&dense))
        };

        let wind = 21.0 * far_dense.output_multiplier(Some(&wind_curve()));
        let solar = 24.0 * far_dense.output_multiplier(Some(&solar_curve()));
        assert!(
            wind > solar * 2.0,
            "at 5 AU on a dense world, wind {wind:.1} should clearly beat solar {solar:.1}"
        );
    }

    #[test]
    fn solar_still_beats_wind_near_the_star_on_a_thin_atmosphere() {
        // The trade has to run both ways, or one of them is simply better.
        let m = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let thin = body_with(AtmosphereDensity::Thin);
        let near_thin = SiteContext {
            insolation: Some(1.0), // 1 AU
            ..ctx(&m, 0, 0, Some(&thin))
        };

        let wind = 21.0 * near_thin.output_multiplier(Some(&wind_curve()));
        let solar = 24.0 * near_thin.output_multiplier(Some(&solar_curve()));
        assert!(
            solar > wind,
            "solar {solar:.1} should beat wind {wind:.1} here"
        );
    }
}
