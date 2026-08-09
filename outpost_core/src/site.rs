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

use crate::content::types::SiteRequirement;
use crate::map::{HexCoord, PlanetMap};
use crate::system::Body;

/// Where a building is being placed, as far as its requirements care.
#[derive(Debug, Clone, Copy)]
pub struct SiteContext<'a> {
    /// The body's surface map, if one has been generated.
    pub map: Option<&'a PlanetMap>,
    /// The site's own hex. `None` for a site with no surface position.
    pub coord: Option<HexCoord>,
    /// The body the site is on, if known.
    pub body: Option<&'a Body>,
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

    /// Whether `req` holds at this site.
    #[must_use]
    pub fn satisfies(&self, req: &SiteRequirement) -> bool {
        match req {
            SiteRequirement::Terrain {
                any_of,
                within_hexes,
            } => self
                .cells_in_range(*within_hexes)
                .iter()
                .any(|cell| any_of.contains(&cell.terrain)),
            SiteRequirement::Deposit {
                commodity,
                within_hexes,
            } => self
                .cells_in_range(*within_hexes)
                .iter()
                .any(|cell| cell.deposits.iter().any(|d| &d.commodity_id == commodity)),
            SiteRequirement::MinAtmosphere { density } => self
                .body
                .is_some_and(|b| b.atmosphere_density.rank() >= density.rank()),
        }
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
        }
    }

    #[test]
    fn terrain_on_the_site_itself_satisfies_a_zero_radius_requirement() {
        let map = map_with(&[(0, 0, Terrain::Volcanic, &[])]);
        let req = SiteRequirement::Terrain {
            any_of: vec![Terrain::Volcanic],
            within_hexes: 0,
        };
        assert!(ctx(&map, 0, 0, None).satisfies(&req));
    }

    #[test]
    fn terrain_one_hex_away_needs_the_radius_to_reach_it() {
        let map = map_with(&[(0, 0, Terrain::Plains, &[]), (1, 0, Terrain::Ocean, &[])]);
        let ocean = |within_hexes| SiteRequirement::Terrain {
            any_of: vec![Terrain::Ocean],
            within_hexes,
        };
        assert!(!ctx(&map, 0, 0, None).satisfies(&ocean(0)));
        assert!(ctx(&map, 0, 0, None).satisfies(&ocean(1)));
    }

    #[test]
    fn any_of_is_satisfied_by_any_listed_terrain() {
        let map = map_with(&[(0, 0, Terrain::Wetlands, &[])]);
        let req = SiteRequirement::Terrain {
            any_of: vec![Terrain::Ocean, Terrain::Wetlands],
            within_hexes: 0,
        };
        assert!(ctx(&map, 0, 0, None).satisfies(&req));
    }

    #[test]
    fn a_deposit_in_range_satisfies_the_requirement() {
        let map = map_with(&[
            (0, 0, Terrain::Plains, &[]),
            (1, 0, Terrain::Plains, &["hydrocarbons"]),
        ]);
        let req = |within_hexes| SiteRequirement::Deposit {
            commodity: "hydrocarbons".into(),
            within_hexes,
        };
        assert!(!ctx(&map, 0, 0, None).satisfies(&req(0)));
        assert!(ctx(&map, 0, 0, None).satisfies(&req(1)));
    }

    #[test]
    fn a_different_commodity_does_not_satisfy_a_deposit_requirement() {
        let map = map_with(&[(0, 0, Terrain::Plains, &["silicates"])]);
        let req = SiteRequirement::Deposit {
            commodity: "hydrocarbons".into(),
            within_hexes: 1,
        };
        assert!(!ctx(&map, 0, 0, None).satisfies(&req));
    }

    #[test]
    fn a_hex_across_the_east_west_seam_is_still_in_range() {
        // `within_radius` produces q = -1 for a site at q = 0; the map stores
        // that column as q = width - 1. Without wrapping, the requirement
        // would not see it.
        let map = map_with(&[(0, 0, Terrain::Plains, &[]), (7, 0, Terrain::Ocean, &[])]);
        let req = SiteRequirement::Terrain {
            any_of: vec![Terrain::Ocean],
            within_hexes: 1,
        };
        assert!(
            ctx(&map, 0, 0, None).satisfies(&req),
            "the map wraps east-west, so q=7 is adjacent to q=0"
        );
    }

    #[test]
    fn atmosphere_is_met_at_or_above_the_required_density() {
        let map = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let req = SiteRequirement::MinAtmosphere {
            density: AtmosphereDensity::Thin,
        };
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
        };

        assert!(!ctx.satisfies(&SiteRequirement::Terrain {
            any_of: vec![Terrain::Ocean],
            within_hexes: 2,
        }));
        assert!(!ctx.satisfies(&SiteRequirement::Deposit {
            commodity: "hydrocarbons".into(),
            within_hexes: 2,
        }));
        // ...but a body-scoped one still answers.
        assert!(ctx.satisfies(&SiteRequirement::MinAtmosphere {
            density: AtmosphereDensity::Thin,
        }));
    }

    #[test]
    fn unmet_reports_every_failure_not_just_the_first() {
        let map = map_with(&[(0, 0, Terrain::Plains, &[])]);
        let body = body_with(AtmosphereDensity::Vacuum);
        let reqs = vec![
            SiteRequirement::Terrain {
                any_of: vec![Terrain::Ocean],
                within_hexes: 0,
            },
            SiteRequirement::Deposit {
                commodity: "hydrocarbons".into(),
                within_hexes: 0,
            },
            SiteRequirement::MinAtmosphere {
                density: AtmosphereDensity::Thin,
            },
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
            SiteRequirement::Terrain {
                any_of: vec![Terrain::Ocean],
                within_hexes: 2
            }
            .describe(),
            "ocean within 2 hexes"
        );
        assert_eq!(
            SiteRequirement::Deposit {
                commodity: "hydrocarbons".into(),
                within_hexes: 1
            }
            .describe(),
            "hydrocarbons deposit within 1 hex"
        );
        assert_eq!(
            SiteRequirement::MinAtmosphere {
                density: AtmosphereDensity::Thin
            }
            .describe(),
            "thin atmosphere or denser"
        );
    }
}
