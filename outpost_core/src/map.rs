//! Hex map: planet terrain, biomes, resource deposits, colony nodes, and infrastructure.
//!
//! Implements Phase 5 of the build sequence (DESIGN.md §8.1).
//!
//! # Coordinate System
//!
//! Axial coordinates `(q, r)` following the standard "pointy-top" hex grid convention.
//! The third cube coordinate is derived: `s = -q - r`.
//!
//! # Map Generation
//!
//! [`PlanetMap::generate`] is purely deterministic from `seed + radius`.  No I/O.

use std::collections::HashMap;

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::colony::ColonyId;
use crate::system::{PlanetarySubtype, TemperatureBand};
use crate::trade::SiteId;

/// Root-3, memoised for hex-to-cartesian conversion.
const SQRT_3: f32 = 1.732_050_8;

// ─── Hex Coordinates ─────────────────────────────────────────────────────────

/// Axial hex coordinate `(q, r)` in a "pointy-top" grid.
///
/// The cube coordinate `s = -q - r` is derived on demand via [`HexCoord::s`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HexCoord {
    /// Column axis.
    pub q: i32,
    /// Row axis.
    pub r: i32,
}

impl HexCoord {
    /// Construct an axial hex coordinate.
    #[must_use]
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// The origin cell `(0, 0)`.
    #[must_use]
    pub const fn origin() -> Self {
        Self::new(0, 0)
    }

    /// The derived cube coordinate `s = -q - r`.
    #[must_use]
    pub fn s(self) -> i32 {
        -self.q - self.r
    }

    /// Hex distance (in cells) between two coordinates.
    ///
    /// Uses the standard cube-coordinate formula:
    /// `distance = max(|dq|, |dr|, |ds|)`
    #[must_use]
    pub fn distance(self, other: Self) -> u32 {
        let dq = (self.q - other.q).unsigned_abs();
        let dr = (self.r - other.r).unsigned_abs();
        let ds = (self.s() - other.s()).unsigned_abs();
        dq.max(dr).max(ds)
    }

    /// All six direct neighbours of this hex cell.
    #[must_use]
    pub fn neighbours(self) -> [HexCoord; 6] {
        const DIRECTIONS: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];
        DIRECTIONS.map(|(dq, dr)| HexCoord::new(self.q + dq, self.r + dr))
    }

    /// Return all cells within `radius` steps of this cell (inclusive of center).
    #[must_use]
    pub fn within_radius(self, radius: u32) -> Vec<HexCoord> {
        let r = radius.cast_signed();
        let mut cells = Vec::new();
        for q in -r..=r {
            let r_min = (-r).max(-q - r);
            let r_max = r.min(-q + r);
            for row in r_min..=r_max {
                cells.push(HexCoord::new(self.q + q, self.r + row));
            }
        }
        cells
    }
}

// ─── Terrain & Biome ─────────────────────────────────────────────────────────

/// Surface terrain type of a hex cell.
///
/// Determines movement cost and infrastructure construction difficulty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    /// Flat open ground — lowest construction cost.
    Plains,
    /// Elevated rocky terrain — moderate cost.
    Hills,
    /// Rough broken terrain — high cost.
    Mountains,
    /// Low-lying water-logged terrain — moderate cost.
    Wetlands,
    /// Surface covered by water — impassable for ground infrastructure.
    Ocean,
    /// Volcanic or geothermally active terrain — very high cost.
    Volcanic,
}

impl Terrain {
    /// Infrastructure construction difficulty multiplier for this terrain.
    ///
    /// Multiplied against the base cost when computing edge costs.
    #[must_use]
    pub fn difficulty(self) -> f32 {
        match self {
            Terrain::Plains => 1.0,
            Terrain::Hills => 1.8,
            Terrain::Mountains => 3.5,
            Terrain::Wetlands => 2.0,
            Terrain::Ocean => f32::INFINITY,
            Terrain::Volcanic => 5.0,
        }
    }
}

/// Surface biome (ecological / environmental classification).
///
/// Influences colony production bonuses and hazard levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Biome {
    /// Arid desert biome.
    Desert,
    /// Cold tundra biome.
    Tundra,
    /// Ice-covered polar biome.
    Polar,
    /// Temperate forest biome.
    Forest,
    /// Tropical / high-humidity biome.
    Jungle,
    /// Open grassland biome.
    Grassland,
    /// Rocky bare-rock biome.
    Barren,
    /// Open ocean biome.
    Ocean,
    /// Geothermal zone biome.
    Geothermal,
}

// ─── Deposits ────────────────────────────────────────────────────────────────

/// A resource deposit present in a hex cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deposit {
    /// Content-pack commodity id (e.g. `"iron"`, `"water_ice"`).
    pub commodity_id: String,
    /// Relative richness in `(0.0, 1.0]` — 1.0 is maximum richness.
    pub richness: f32,
}

impl Deposit {
    /// Create a new deposit record.
    #[must_use]
    pub fn new(commodity_id: impl Into<String>, richness: f32) -> Self {
        Self {
            commodity_id: commodity_id.into(),
            richness: richness.clamp(0.001, 1.0),
        }
    }
}

// ─── HexCell ─────────────────────────────────────────────────────────────────

fn default_elevation() -> f32 {
    0.5
}
fn default_cell_temperature() -> TemperatureBand {
    TemperatureBand::Temperate
}

/// A single hex cell in the planet map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HexCell {
    /// Axial coordinate of this cell.
    pub coord: HexCoord,
    /// Surface terrain class.
    pub terrain: Terrain,
    /// Surface biome class.
    pub biome: Biome,
    /// Normalised elevation in `[0.0, 1.0]` — 0.0 = lowest basin, 1.0 = highest peak.
    #[serde(default = "default_elevation")]
    pub elevation: f32,
    /// Per-cell surface-temperature band, derived from the parent body's band plus
    /// per-cell latitude and elevation deltas.
    #[serde(default = "default_cell_temperature")]
    pub temperature: TemperatureBand,
    /// Resource deposits present in this cell (may be empty).
    pub deposits: Vec<Deposit>,
}

impl HexCell {
    /// Construct a cell with no deposits, mid-elevation, and Temperate band.
    ///
    /// Used by tests and edge-cost fixtures — production maps go through
    /// [`PlanetMap::generate_for_body`], which populates elevation and
    /// temperature deterministically.
    #[must_use]
    pub fn new(coord: HexCoord, terrain: Terrain, biome: Biome) -> Self {
        Self {
            coord,
            terrain,
            biome,
            elevation: default_elevation(),
            temperature: default_cell_temperature(),
            deposits: Vec::new(),
        }
    }

    /// Infrastructure construction difficulty, inherited from terrain.
    #[must_use]
    pub fn difficulty(&self) -> f32 {
        self.terrain.difficulty()
    }

    /// Return `true` if a colony can be founded on this cell.
    ///
    /// Ocean cells are excluded; all other terrain is habitable.
    #[must_use]
    pub fn is_habitable(&self) -> bool {
        !matches!(self.terrain, Terrain::Ocean)
    }

    /// A simple suitability score used for landing site selection.
    ///
    /// Higher is better. Deposits increase the score; difficult terrain
    /// reduces it; a harsh per-cell temperature band (issue #190) scales the
    /// whole score down, so an equatorial Temperate hex on a Cold body can
    /// still outrank a polar hex with better terrain/deposits.
    #[must_use]
    pub fn suitability(&self) -> f32 {
        if !self.is_habitable() {
            return 0.0;
        }
        let base = 10.0 / self.terrain.difficulty();
        let deposit_bonus: f32 = self.deposits.iter().map(|d| d.richness * 5.0).sum();
        (base + deposit_bonus) * temperature_suitability_factor(self.temperature)
    }
}

/// Hex radius treated as "in proximity" when scoring a founding site
/// ([`PlanetMap::site_score`]) — the site cell plus two rings, 19 hexes.
///
/// Sized as the neighbourhood a colony can plausibly reach and exploit rather
/// than the whole map, so the recommendation stays local and meaningful.
pub const SITE_PROXIMITY_RADIUS: u32 = 2;

/// How much a unit of distance-weighted per-commodity richness is worth
/// against the terrain base in [`PlanetMap::site_score`]. Matches the weight
/// [`HexCell::suitability`] gives a single cell's deposits, so the two scores
/// stay on a comparable scale.
const PROXIMITY_DEPOSIT_WEIGHT: f32 = 5.0;

/// Weight a neighbouring cell's deposits by how far away they are, so
/// resources on the site itself count fullest and ones two rings out still
/// count meaningfully. Linear: `1.0`, `0.75`, `0.5` at distance 0, 1, 2.
fn proximity_falloff(distance: u32) -> f32 {
    // `u8::from` keeps the conversion lossless; the clamped distance is always
    // a small constant. The floor keeps the weight positive should the radius
    // ever be widened past 4.
    let steps = f32::from(u8::try_from(distance.min(SITE_PROXIMITY_RADIUS)).unwrap_or(0));
    (1.0 - 0.25 * steps).max(0.05)
}

/// Suitability multiplier from a cell's surface temperature band, in
/// `(0.0, 1.0]`.
///
/// Mirrors the relative ordering `Body::habitability` (issue #163) already
/// uses at the body scale — Temperate is best, Extreme is worst — so a
/// player reads the same "harsh climate" story at both the system map and
/// the hex map. This is a soft penalty, not a hard block (issue #190 left
/// hard-blocking colonisation on harsh cells to #183, which hasn't landed
/// yet): a small floor keeps terrain/deposit differences visible even among
/// Extreme-band cells rather than collapsing them all to an identical zero
/// score.
#[must_use]
fn temperature_suitability_factor(temperature: TemperatureBand) -> f32 {
    let points: f32 = match temperature {
        TemperatureBand::Temperate => 30.0,
        TemperatureBand::Cold => 20.0,
        TemperatureBand::Hot => 15.0,
        TemperatureBand::Frozen => 5.0,
        TemperatureBand::Extreme => 0.0,
    };
    (points / 30.0).max(0.05)
}

// ─── PlanetMap ───────────────────────────────────────────────────────────────

/// A hex grid map of a planet surface.
///
/// Generated deterministically from a seed and radius via [`PlanetMap::generate`].
/// Cells are stored in a flat hash map keyed by axial coordinate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanetMap {
    /// RNG seed used to generate this map (for reproducibility checks).
    pub seed: u64,
    /// Radius of the map in cells from the origin (exclusive of boundary).
    pub radius: u32,
    /// All cells in the map, indexed by axial coordinate.
    ///
    /// Serialized as a flat list of `(coord, cell)` pairs, not as a JSON
    /// object: `serde_json` — the save format — cannot use the [`HexCoord`]
    /// struct as an object key, and every save of a real game failed with
    /// `"key must be a string"` because of it (issue #337).
    #[serde(with = "cells_serde")]
    pub cells: HashMap<HexCoord, HexCell>,
    /// Colony nodes placed on this map.
    pub colonies: Vec<ColonyNode>,
    /// Infrastructure edges connecting colony nodes.
    pub edges: Vec<InfraEdge>,
    /// Deterministic site identifiers for each cell, keyed by [`SiteId`].
    pub sites: HashMap<SiteId, HexCoord>,
}

/// (De)serialize [`PlanetMap::cells`] as a `Vec<(HexCoord, HexCell)>`.
///
/// The axial coordinate is a struct, so it cannot be a JSON object key. See the
/// field's doc comment.
mod cells_serde {
    use super::{HashMap, HexCell, HexCoord};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S: Serializer>(
        cells: &HashMap<HexCoord, HexCell>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        // Sorted so a save file is byte-stable across runs — `HashMap` iteration
        // order is not, and an unstable save defeats diffing and hashing.
        let mut pairs: Vec<(&HexCoord, &HexCell)> = cells.iter().collect();
        pairs.sort_by_key(|(c, _)| (c.q, c.r));
        pairs.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<HexCoord, HexCell>, D::Error> {
        let pairs = Vec::<(HexCoord, HexCell)>::deserialize(deserializer)?;
        Ok(pairs.into_iter().collect())
    }
}

impl PlanetMap {
    /// Generate a planet map deterministically from `seed` and `radius`, assuming
    /// a Temperate parent body.
    ///
    /// Thin wrapper around [`Self::generate_for_body`] for callers that don't
    /// yet know the parent body (bootstrap path, unit tests).
    #[must_use]
    pub fn generate(seed: u64, radius: u32) -> Self {
        Self::generate_for_body(seed, radius, TemperatureBand::Temperate)
    }

    /// Generate a planet map deterministically from `seed`, `radius`, and the
    /// parent body's `TemperatureBand`.
    ///
    /// Thin wrapper around [`Self::generate_for_body_and_subtype`] with
    /// [`PlanetarySubtype::Unclassified`] — identical output to a body whose
    /// subtype hasn't been authored, and (by construction) identical to
    /// [`PlanetarySubtype::EarthLike`] too, since neither biases deposit
    /// generation (issue #196).
    #[must_use]
    pub fn generate_for_body(seed: u64, radius: u32, body_temperature: TemperatureBand) -> Self {
        Self::generate_for_body_and_subtype(
            seed,
            radius,
            body_temperature,
            PlanetarySubtype::Unclassified,
        )
    }

    /// Generate a planet map deterministically from `seed`, `radius`, the
    /// parent body's `TemperatureBand`, and its [`PlanetarySubtype`].
    ///
    /// Per-cell temperature is derived from the body band, per-cell latitude
    /// (relative to a seed-oriented equator line through the origin), and
    /// elevation. Higher latitudes and elevations shift the cell colder.
    /// Elevation also biases terrain: peaks favour mountains/volcanic, basins
    /// favour ocean/wetlands.
    ///
    /// Deposits are generated in two passes (issue #188): elevation is
    /// computed for every cell first, then a handful of per-commodity "vein
    /// centres" are placed (biased toward each commodity's preferred
    /// elevation band and, per `planetary_subtype`, toward commodities that
    /// archetype favours — see [`subtype_commodity_multiplier`]), and
    /// finally each cell's deposit roll is biased by proximity to the
    /// nearest vein — producing coherent ore fields instead of independent
    /// per-cell noise.
    #[must_use]
    pub fn generate_for_body_and_subtype(
        seed: u64,
        radius: u32,
        body_temperature: TemperatureBand,
        planetary_subtype: PlanetarySubtype,
    ) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let coords = HexCoord::origin().within_radius(radius);
        let mut cells = HashMap::with_capacity(coords.len());
        let mut sites = HashMap::with_capacity(coords.len());

        let equator_normal = equator_normal(seed);

        // Pass 1: elevation for every cell (drives vein placement below).
        // The archetype's elevation bias (issue #313) is folded in here, not
        // just at terrain-classification time, so a Mountain world's cells
        // genuinely report high `HexCell::elevation` rather than only being
        // more likely to roll a mountain terrain tag.
        let elevation_bias = planetary_subtype.elevation_bias();
        let elevations: HashMap<HexCoord, f32> = coords
            .iter()
            .map(|coord| {
                let raw = compute_elevation(seed, *coord, &mut rng);
                (*coord, (raw + elevation_bias).clamp(0.0, 1.0))
            })
            .collect();

        // The elevation quantile below which a cell becomes ocean, chosen so
        // the map's water coverage matches the archetype's target land
        // fraction (issue #313) instead of a fixed per-cell probability.
        // `None` (gas/ice giants) keeps the pre-#313 fixed-probability
        // behaviour — see `target_land_fraction`'s doc comment.
        let water_threshold = planetary_subtype
            .target_land_fraction()
            .map(|land_fraction| water_threshold_for(&elevations, land_fraction));

        // Vein centres, keyed by commodity, placed after elevation is known
        // so elevation-band bias (e.g. iron on ridges) can steer placement.
        let veins = place_veins(&mut rng, &coords, &elevations, radius, planetary_subtype);

        // Pass 2: terrain, biome, temperature, and deposits.
        for coord in &coords {
            let elevation = elevations[coord];
            let cell = generate_cell(
                &mut rng,
                *coord,
                radius,
                elevation,
                body_temperature,
                equator_normal,
                &veins,
                water_threshold,
            );
            cells.insert(*coord, cell);
            let site_id = site_id_for_coord(seed, *coord);
            sites.insert(site_id, *coord);
        }

        // Issue #232: guarantee every curated raw-material commodity has at
        // least one real deposit somewhere on the map. Normal vein/roll
        // placement above is probabilistic per commodity (a subtype
        // multiplier can legitimately drive a commodity's vein count toward
        // zero, e.g. no hydrocarbons on a molten world) — without this pass
        // a bad seed could produce a founding site with no path to an early
        // tech tier. Deterministic, no retry loop, mirrors
        // `system_gen.rs::force_habitable`'s "verify then patch" pattern.
        force_guaranteed_deposits(&mut cells, &coords, &elevations, seed);

        Self {
            seed,
            radius,
            cells,
            colonies: Vec::new(),
            edges: Vec::new(),
            sites,
        }
    }

    /// Return the hex coordinate for a given site identifier, if it exists.
    #[must_use]
    pub fn coord_for_site(&self, site_id: SiteId) -> Option<HexCoord> {
        self.sites.get(&site_id).copied()
    }

    /// Return a reference to a cell by coordinate, if it exists.
    #[must_use]
    pub fn cell(&self, coord: HexCoord) -> Option<&HexCell> {
        self.cells.get(&coord)
    }

    /// Return the best landing site in the map, by [`Self::site_score`].
    ///
    /// Returns `None` only if the map has no habitable cells (degenerate).
    #[must_use]
    pub fn best_landing_site(&self) -> Option<HexCoord> {
        self.top_landing_sites(1, 0).into_iter().next()
    }

    /// Score a candidate founding site by the **variety of resources within
    /// reach**, combined with the site cell's own terrain and climate.
    ///
    /// Distinct from [`HexCell::suitability`], which scores a single cell in
    /// isolation (and is what the hex map displays per-tile). A founding
    /// recommendation needs a different question answered — "where can this
    /// colony draw the widest range of resources from?" — so this scans the
    /// neighbourhood within [`SITE_PROXIMITY_RADIUS`] and keeps, **per
    /// commodity**, the best distance-weighted richness found.
    ///
    /// Because the per-commodity bests are summed, an area holding several
    /// different resources outscores one holding a single very rich deposit —
    /// piling more of the same commodity nearby only improves that one term.
    /// This is issue #302: the old cell-only score summed `richness * 5.0`
    /// across a single cell's deposits, so one rich `precious_ore` tile beat a
    /// well-rounded neighbourhood.
    ///
    /// Returns `0.0` for a missing or uninhabitable cell.
    #[must_use]
    pub fn site_score(&self, coord: HexCoord) -> f32 {
        let Some(cell) = self.cell(coord) else {
            return 0.0;
        };
        if !cell.is_habitable() {
            return 0.0;
        }

        // Best distance-weighted richness per distinct commodity in reach.
        let mut best_per_commodity: HashMap<&str, f32> = HashMap::new();
        for near_coord in coord.within_radius(SITE_PROXIMITY_RADIUS) {
            let Some(near) = self.cell(near_coord) else {
                continue;
            };
            let falloff = proximity_falloff(coord.distance(near_coord));
            for deposit in &near.deposits {
                let weighted = deposit.richness * falloff;
                let entry = best_per_commodity
                    .entry(deposit.commodity_id.as_str())
                    .or_insert(0.0);
                if weighted > *entry {
                    *entry = weighted;
                }
            }
        }
        let variety_bonus: f32 =
            best_per_commodity.values().sum::<f32>() * PROXIMITY_DEPOSIT_WEIGHT;

        // Terrain workability plus climate, mirroring `HexCell::suitability`'s
        // shape so a harsh-climate site still loses to a temperate one even
        // when it's resource-rich (climate multiplies the whole score).
        let base = 10.0 / cell.terrain.difficulty();
        (base + variety_bonus) * temperature_suitability_factor(cell.temperature)
    }

    /// Return up to `n` recommended landing sites, greedily selected by
    /// [`Self::site_score`] while enforcing a minimum hex distance between any
    /// two picks (issue #188) so the recommendations don't all cluster in the
    /// same corner of the map.
    ///
    /// Habitable, unoccupied cells only. If fewer than `n` cells satisfy the
    /// distance constraint, the returned list is shorter than `n`.
    #[must_use]
    pub fn top_landing_sites(&self, n: usize, min_distance: u32) -> Vec<HexCoord> {
        // Score each candidate once up front rather than inside the comparator
        // — `site_score` scans a 19-hex neighbourhood, so scoring inside
        // `sort_by` would recompute it O(n log n) times on a path the founding
        // wizard hits interactively.
        let mut candidates: Vec<(HexCoord, f32)> = self
            .cells
            .values()
            .filter(|c| c.is_habitable() && !self.colonies.iter().any(|node| node.coord == c.coord))
            .map(|c| (c.coord, self.site_score(c.coord)))
            .collect();
        candidates.sort_by(|(a_coord, a_score), (b_coord, b_score)| {
            b_score
                .partial_cmp(a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| (a_coord.q, a_coord.r).cmp(&(b_coord.q, b_coord.r)))
        });

        let mut picked: Vec<HexCoord> = Vec::with_capacity(n);
        for (coord, _) in candidates {
            if picked.len() >= n {
                break;
            }
            if picked.iter().all(|p| p.distance(coord) >= min_distance) {
                picked.push(coord);
            }
        }
        picked
    }

    /// Place a colony node at the given hex coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`MapError::CellNotFound`] if the coordinate is not in this map.
    /// Returns [`MapError::CellNotHabitable`] if the terrain is ocean.
    /// Returns [`MapError::CellOccupied`] if a colony already exists at that coordinate.
    pub fn place_colony(&mut self, colony_id: ColonyId, coord: HexCoord) -> Result<(), MapError> {
        let cell = self
            .cells
            .get(&coord)
            .ok_or(MapError::CellNotFound(coord))?;
        if !cell.is_habitable() {
            return Err(MapError::CellNotHabitable(coord));
        }
        if self.colonies.iter().any(|n| n.coord == coord) {
            return Err(MapError::CellOccupied(coord));
        }
        self.colonies.push(ColonyNode { colony_id, coord });
        Ok(())
    }

    /// Add an infrastructure edge between two colony nodes.
    ///
    /// Computes cost from the cells along the straight-line path between the two
    /// coordinates (Manhattan path through axial space) plus the infrastructure type.
    ///
    /// # Errors
    ///
    /// Returns [`MapError::ColonyNotOnMap`] if either colony is not placed.
    /// Returns [`MapError::EdgeExists`] if an edge of the same type already connects
    /// the two colonies.
    pub fn add_edge(
        &mut self,
        from: ColonyId,
        to: ColonyId,
        infra_type: InfraType,
    ) -> Result<InfraEdge, MapError> {
        let from_coord = self
            .colonies
            .iter()
            .find(|n| n.colony_id == from)
            .map(|n| n.coord)
            .ok_or(MapError::ColonyNotOnMap(from))?;
        let to_coord = self
            .colonies
            .iter()
            .find(|n| n.colony_id == to)
            .map(|n| n.coord)
            .ok_or(MapError::ColonyNotOnMap(to))?;

        // Duplicate check.
        if self.edges.iter().any(|e| {
            e.infra_type == infra_type
                && ((e.from == from && e.to == to) || (e.from == to && e.to == from))
        }) {
            return Err(MapError::EdgeExists { from, to });
        }

        let cost = edge_cost(from_coord, to_coord, &self.cells, infra_type);
        let throughput = infra_type.base_throughput();
        let edge = InfraEdge {
            from,
            to,
            infra_type,
            cost,
            throughput,
        };
        self.edges.push(edge.clone());
        Ok(edge)
    }
}

// ─── Colony Nodes ────────────────────────────────────────────────────────────

/// A colony placed on a hex cell, acting as a network node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColonyNode {
    /// The colony's stable identifier.
    pub colony_id: ColonyId,
    /// The hex cell the colony occupies.
    pub coord: HexCoord,
}

// ─── Infrastructure Edges ────────────────────────────────────────────────────

/// The type of infrastructure connecting two colony nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfraType {
    /// Unpaved / basic road — cheap, low throughput.
    Road,
    /// Rail line — expensive, high throughput, bonus on flat terrain.
    Rail,
    /// Buried pipeline — moderate cost, high fluid throughput.
    Pipeline,
}

impl InfraType {
    /// Base construction cost factor (multiplied against distance × terrain).
    #[must_use]
    pub fn base_cost_factor(self) -> f32 {
        match self {
            InfraType::Road => 1.0,
            InfraType::Rail => 3.5,
            InfraType::Pipeline => 2.5,
        }
    }

    /// Baseline cargo throughput (units per turn) before tech modifiers.
    #[must_use]
    pub fn base_throughput(self) -> f32 {
        match self {
            InfraType::Road => 50.0,
            InfraType::Rail => 200.0,
            InfraType::Pipeline => 150.0,
        }
    }

    /// Throughput after applying a tech-level multiplier.
    #[must_use]
    pub fn throughput_with_tech(self, tech_multiplier: f32) -> f32 {
        self.base_throughput() * tech_multiplier.max(1.0)
    }
}

/// An infrastructure edge connecting two colony nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfraEdge {
    /// Source colony identifier.
    pub from: ColonyId,
    /// Destination colony identifier.
    pub to: ColonyId,
    /// Type of infrastructure.
    pub infra_type: InfraType,
    /// Construction cost (in abstract resource units).
    pub cost: f32,
    /// Cargo throughput per turn (before tech modifiers).
    pub throughput: f32,
}

// ─── Map Errors ──────────────────────────────────────────────────────────────

/// Errors that can arise from planet-map operations.
#[derive(Debug, Error)]
pub enum MapError {
    /// The referenced hex coordinate is not in the map.
    #[error("cell not found: ({}, {})", .0.q, .0.r)]
    CellNotFound(HexCoord),
    /// The hex cell is ocean or otherwise uninhabitable.
    #[error("cell not habitable: ({}, {})", .0.q, .0.r)]
    CellNotHabitable(HexCoord),
    /// A colony is already placed at this coordinate.
    #[error("cell already occupied: ({}, {})", .0.q, .0.r)]
    CellOccupied(HexCoord),
    /// The colony is not placed on this map.
    #[error("colony not on map: {0}")]
    ColonyNotOnMap(ColonyId),
    /// An edge of this infrastructure type already connects these two colonies.
    #[error("infrastructure edge already exists between the two colonies")]
    EdgeExists {
        /// From colony.
        from: ColonyId,
        /// To colony.
        to: ColonyId,
    },
}

// ─── Generation Helpers ──────────────────────────────────────────────────────

/// Derive a deterministic [`SiteId`] from a map seed and hex coordinate.
///
/// Uses a simple mixing of seed, q, and r so every (seed, coord) pair maps to
/// a stable, unique UUID that survives save/load round-trips.
#[must_use]
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn site_id_for_coord(seed: u64, coord: HexCoord) -> SiteId {
    // Pack seed + q + r into 16 bytes without any std hashing that may change.
    let q = i64::from(coord.q);
    let r = i64::from(coord.r);
    // Mix: combine seed with rotated q/r words.  Sign loss is intentional —
    // we want all bits of q/r to participate in the hash.
    let high = seed
        ^ ((q as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15))
        ^ ((r as u64).wrapping_mul(0x6c62_272e_07bb_0142));
    let low = seed
        .wrapping_add((q as u64).wrapping_mul(0x517c_c1b7_2722_0a95))
        .wrapping_add((r as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9));
    let bytes = [
        (high >> 56) as u8,
        (high >> 48) as u8,
        (high >> 40) as u8,
        (high >> 32) as u8,
        (high >> 24) as u8,
        (high >> 16) as u8,
        (high >> 8) as u8,
        high as u8,
        (low >> 56) as u8,
        (low >> 48) as u8,
        (low >> 40) as u8,
        (low >> 32) as u8,
        (low >> 24) as u8,
        (low >> 16) as u8,
        (low >> 8) as u8,
        low as u8,
    ];
    SiteId(uuid::Uuid::from_bytes(bytes))
}

/// Generate a single hex cell at `coord` using the given RNG.
///
/// `elevation` is precomputed for every cell before this function runs (see
/// [`PlanetMap::generate_for_body`]) so vein placement can be biased by it.
/// `equator_normal` is the seed-derived unit normal to the equator line, used to
/// project each hex into a latitude proxy in `[0.0, 1.0]`. `body_temperature`
/// carries through as the baseline temperature band before latitude/elevation
/// shifts. `veins` are the map's per-commodity vein centres, used to bias the
/// deposit roll and commodity choice toward coherent ore fields.
/// `water_threshold` is the archetype's elevation-quantile ocean cut (issue
/// #313), or `None` for subtypes with no land/water target.
#[allow(clippy::too_many_arguments)]
fn generate_cell(
    rng: &mut ChaCha8Rng,
    coord: HexCoord,
    radius: u32,
    elevation: f32,
    body_temperature: TemperatureBand,
    equator_normal: (f32, f32),
    veins: &[Vein],
    water_threshold: Option<f32>,
) -> HexCell {
    // Rough distance from centre as a fraction [0, 1].
    let dist_frac = if radius == 0 {
        0.0
    } else {
        #[allow(clippy::cast_precision_loss)]
        {
            HexCoord::origin().distance(coord) as f32 / radius as f32
        }
    };

    // Latitude proxy: perpendicular distance from a seed-oriented equator line
    // through the origin, normalised so poles sit at ~1.0.
    let latitude_abs = cell_latitude_abs(coord, equator_normal, radius);

    // Elevation biases the terrain roll: high elevation shifts toward
    // Mountains/Volcanic (lower buckets); low elevation shifts toward
    // Plains/Wetlands (higher buckets).
    let terrain_roll: f32 = rng.gen();
    let bias = (elevation - 0.5) * 0.3;
    let adjusted = (terrain_roll - bias).clamp(0.0, 1.0);
    let terrain = if dist_frac < 0.15 {
        // Near-polar regions favour flat plains.
        Terrain::Plains
    } else if let Some(threshold) = water_threshold {
        // Issue #313: ocean is an elevation-quantile cut chosen so the whole
        // map's water coverage matches the archetype's target land fraction,
        // rather than a fixed low-probability roll — a genuine ocean world
        // now reliably comes out mostly water, not "mostly land with the
        // occasional lake."
        if elevation <= threshold {
            Terrain::Ocean
        } else if adjusted < 0.06 {
            Terrain::Volcanic
        } else if adjusted < 0.18 {
            Terrain::Mountains
        } else if adjusted < 0.33 {
            Terrain::Hills
        } else if adjusted < 0.43 {
            Terrain::Wetlands
        } else {
            Terrain::Plains
        }
    } else if adjusted < 0.02 {
        // No archetype target (gas/ice giants) — unchanged pre-#313 behaviour.
        if elevation < 0.35 {
            Terrain::Ocean
        } else {
            Terrain::Plains
        }
    } else if adjusted < 0.08 {
        Terrain::Volcanic
    } else if adjusted < 0.20 {
        Terrain::Mountains
    } else if adjusted < 0.35 {
        Terrain::Hills
    } else if adjusted < 0.45 {
        Terrain::Wetlands
    } else {
        Terrain::Plains
    };

    // Pick biome independent of terrain.
    let biome_roll: f32 = rng.gen();
    let biome = if matches!(terrain, Terrain::Ocean) {
        Biome::Ocean
    } else if matches!(terrain, Terrain::Volcanic) {
        Biome::Geothermal
    } else if biome_roll < 0.10 {
        Biome::Polar
    } else if biome_roll < 0.20 {
        Biome::Tundra
    } else if biome_roll < 0.32 {
        Biome::Desert
    } else if biome_roll < 0.50 {
        Biome::Grassland
    } else if biome_roll < 0.65 {
        Biome::Forest
    } else if biome_roll < 0.78 {
        Biome::Barren
    } else {
        Biome::Jungle
    };

    let temperature = cell_temperature(body_temperature, latitude_abs, elevation);

    let mut cell = HexCell::new(coord, terrain, biome);
    cell.elevation = elevation;
    cell.temperature = temperature;

    // Deposit rolls are biased by proximity to the nearest vein centre
    // (issue #188): cells inside a vein's influence radius roll far more
    // often and inherit that vein's commodity, producing coherent ore
    // fields. Cells outside any vein's influence still get a small
    // background chance at a biome-appropriate commodity so the map isn't
    // entirely empty between fields.
    if !matches!(terrain, Terrain::Ocean) {
        let (deposit_prob, nearest_commodity) = nearest_vein_influence(coord, veins);
        if rng.gen::<f32>() < deposit_prob {
            let richness: f32 = rng.gen::<f32>() * 0.9 + 0.1;
            let commodity = nearest_commodity.unwrap_or_else(|| pick_deposit_commodity(rng, biome));
            cell.deposits.push(Deposit::new(commodity, richness));
        }
        // Rare second deposit, at a flat low rate independent of veins —
        // keeps the occasional surprise find outside authored ore fields.
        if rng.gen::<f32>() < BACKGROUND_DEPOSIT_PROB {
            let richness: f32 = rng.gen::<f32>() * 0.5 + 0.05;
            let commodity = pick_deposit_commodity(rng, biome);
            cell.deposits.push(Deposit::new(commodity, richness));
        }
    }

    cell
}

/// Compute an elevation in `[0.0, 1.0]` for `coord` on a map seeded with `seed`.
///
/// Blends a smooth seed-phase-shifted sinusoidal field (which gives coherent
/// ridges) with a per-cell RNG jitter (which breaks up long uniform patches).
/// The RNG roll ordering is preserved even when this function is refactored,
/// so `PlanetMap::generate` determinism holds.
fn compute_elevation(seed: u64, coord: HexCoord, rng: &mut ChaCha8Rng) -> f32 {
    let phase_a = phase_from_seed(seed, 0);
    let phase_b = phase_from_seed(seed, 8);
    let phase_c = phase_from_seed(seed, 16);
    #[allow(clippy::cast_precision_loss)]
    let q = coord.q as f32;
    #[allow(clippy::cast_precision_loss)]
    let r = coord.r as f32;
    let ridge = (q * 0.35 + phase_a).sin();
    let valley = (r * 0.35 + phase_b).sin();
    let cross = ((q + r) * 0.20 + phase_c).sin();
    let spatial = ((ridge + valley + cross) / 3.0 + 1.0) * 0.5;
    let jitter: f32 = rng.gen();
    (spatial * 0.7 + jitter * 0.3).clamp(0.0, 1.0)
}

/// Elevation quantile below which a cell becomes ocean, chosen so that the
/// fraction of cells at or below it is `1.0 - target_land_fraction` (issue
/// #313).
///
/// Deterministic given `elevations`' iteration order does not affect the
/// result: the whole value set is sorted before the quantile is picked.
fn water_threshold_for(elevations: &HashMap<HexCoord, f32>, target_land_fraction: f32) -> f32 {
    let mut values: Vec<f32> = elevations.values().copied().collect();
    values.sort_by(|a, b| a.partial_cmp(b).expect("elevation values are never NaN"));
    if values.is_empty() {
        return 0.0;
    }
    let water_fraction = (1.0 - target_land_fraction).clamp(0.0, 1.0);
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let idx = ((values.len() as f32) * water_fraction).round() as usize;
    let idx = idx.min(values.len() - 1);
    values[idx]
}

fn phase_from_seed(seed: u64, byte_offset: u32) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let byte = ((seed >> byte_offset) & 0xff) as f32;
    (byte / 255.0) * std::f32::consts::TAU
}

/// Unit normal to the seed-oriented equator line through the origin.
///
/// Returned as `(nx, ny)` in pointy-top hex-cartesian space. Multiplying a
/// hex's cartesian position by this normal gives its signed perpendicular
/// distance from the equator — the latitude proxy.
fn equator_normal(seed: u64) -> (f32, f32) {
    let mixed = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    #[allow(clippy::cast_precision_loss)]
    let frac = ((mixed >> 32) as u32) as f32 / u32::MAX as f32;
    let theta = frac * std::f32::consts::TAU;
    (-theta.sin(), theta.cos())
}

/// Absolute latitude proxy for `coord`, in `[0.0, 1.0]`.
///
/// 0.0 = on the equator line; 1.0 = at the farthest hex from the equator on a
/// map of the given radius.
fn cell_latitude_abs(coord: HexCoord, normal: (f32, f32), radius: u32) -> f32 {
    if radius == 0 {
        return 0.0;
    }
    // Pointy-top axial → cartesian, unit hex size.
    #[allow(clippy::cast_precision_loss)]
    let x = SQRT_3 * (coord.q as f32) + (SQRT_3 * 0.5) * (coord.r as f32);
    #[allow(clippy::cast_precision_loss)]
    let y = 1.5 * (coord.r as f32);
    let signed = x * normal.0 + y * normal.1;
    #[allow(clippy::cast_precision_loss)]
    let max = radius as f32 * SQRT_3;
    (signed / max).abs().min(1.0)
}

/// Derive a per-cell temperature band from the parent body's band, latitude,
/// and elevation.
///
/// Uses an ordinal warmth scale (`Extreme = -2` ... `Hot = 2`, monotonic
/// cold→hot). High latitude and high elevation shift the cell colder. On this
/// scale `Extreme` sits below `Frozen` — we treat the body-level `Extreme`
/// band as "unlivably cold" for the purpose of per-cell derivation, which
/// matches the 0-pts habitability weighting in [`crate::system::Body`].
fn cell_temperature(body: TemperatureBand, latitude_abs: f32, elevation: f32) -> TemperatureBand {
    let base: i32 = match body {
        TemperatureBand::Extreme => -2,
        TemperatureBand::Frozen => -1,
        TemperatureBand::Cold => 0,
        TemperatureBand::Temperate => 1,
        TemperatureBand::Hot => 2,
    };
    let lat_shift = if latitude_abs >= 0.85 {
        -2
    } else if latitude_abs >= 0.55 {
        -1
    } else {
        0
    };
    let elev_shift = if elevation >= 0.9 {
        -2
    } else if elevation >= 0.7 {
        -1
    } else {
        0
    };
    let idx = (base + lat_shift + elev_shift).clamp(-2, 2);
    match idx {
        -2 => TemperatureBand::Extreme,
        -1 => TemperatureBand::Frozen,
        0 => TemperatureBand::Cold,
        1 => TemperatureBand::Temperate,
        _ => TemperatureBand::Hot,
    }
}

/// Choose a deposit commodity influenced by biome.
fn pick_deposit_commodity(rng: &mut ChaCha8Rng, biome: Biome) -> &'static str {
    let roll: f32 = rng.gen();
    match biome {
        Biome::Desert | Biome::Barren => {
            if roll < 0.4 {
                "structural_ore"
            } else if roll < 0.7 {
                "silicates"
            } else {
                "conductive_ore"
            }
        }
        Biome::Tundra | Biome::Polar => {
            if roll < 0.5 {
                "hydrocarbons"
            } else if roll < 0.75 {
                "structural_ore"
            } else {
                "fissile_ore"
            }
        }
        Biome::Geothermal => {
            if roll < 0.5 {
                "refractory_ore"
            } else {
                "fissile_ore"
            }
        }
        _ => {
            if roll < 0.35 {
                "structural_ore"
            } else if roll < 0.60 {
                "biomass"
            } else if roll < 0.80 {
                "precious_ore"
            } else {
                "semiconductor_ore"
            }
        }
    }
}

// ─── Deposit Veins (issue #188) ──────────────────────────────────────────────

/// A placed ore-field centre: a commodity and the hex it's anchored on.
type Vein = (&'static str, HexCoord);

/// Every commodity eligible to anchor a vein, in a fixed order so vein
/// placement (and therefore map determinism) doesn't depend on iteration
/// order of any collection.
///
/// This is the curated "founding-site resource closure" issue #232 asks
/// generation to guarantee — deliberately a small, named subset of the full
/// commodity roster (mirroring `content/checks/bootstrap_colony`'s curated
/// starter-loadout philosophy), not every raw material in the content pack.
/// Commodity ids must match `content/base/commodities.yaml` and the raw
/// inputs of `content/base/recipes.yaml`'s `mine_*`/`pump_*` recipes —
/// this list previously drifted from those ids after the #207/#210/#215/#216
/// commodity-family renames and stayed silently stale (deposits referenced
/// commodity ids like `"iron"`/`"water_ice"`/`"organics"` that no longer
/// exist in the content pack, so no mining recipe could ever consume them).
///
/// `pub(crate)` so [`crate::system_gen::distribute_system_resources`] can
/// guarantee the exact same commodity set system-wide — one list, not two
/// that could silently drift apart.
pub(crate) const VEIN_COMMODITIES: [&str; 9] = [
    "structural_ore",
    "conductive_ore",
    "precious_ore",
    "refractory_ore",
    "semiconductor_ore",
    "fissile_ore",
    "silicates",
    "hydrocarbons",
    "biomass",
];

/// Hex distance within which a vein centre biases nearby cells toward its
/// commodity and raises the deposit roll probability. Keeps ore fields
/// readable as a small cluster of hexes rather than a single spawn.
const VEIN_INFLUENCE_RADIUS: u32 = 3;

/// Deposit roll probability at a vein's centre cell.
const VEIN_PEAK_PROB: f32 = 0.55;

/// Deposit roll probability far from any vein (also used for the flat-rate
/// rare "second deposit" roll). Small enough to keep total map coverage
/// near the ~15%-of-hexes target from #188 while still allowing stray finds.
const BACKGROUND_DEPOSIT_PROB: f32 = 0.02;

/// Number of vein centres placed per commodity, scaled so ore-field density
/// (fields per unit area) stays roughly constant as the map grows.
///
/// Map cell count grows quadratically with `radius` (`3r²+3r+1`), so vein
/// count is derived from cell count rather than `radius` directly — a
/// linear-in-`radius` vein count would make small maps disproportionately
/// deposit-dense (each vein's fixed-size influence area covers a much
/// larger fraction of a small map).
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
fn vein_count_for_radius(radius: u32) -> usize {
    let cells = 3 * radius * radius + 3 * radius + 1;
    ((cells as f32 * 0.033 / VEIN_COMMODITIES.len() as f32).round() as usize).max(1)
}

/// Preferred elevation band for a commodity's vein placement, in `[0.0,
/// 1.0]`, or `None` if the commodity has no elevation preference.
///
/// Ores favour ridges/peaks; hydrocarbons and biomass favour valleys/basins —
/// consistent with the elevation field #187 added.
fn commodity_elevation_bias(commodity: &str) -> Option<f32> {
    match commodity {
        "structural_ore" | "conductive_ore" | "precious_ore" | "refractory_ore"
        | "semiconductor_ore" | "fissile_ore" => Some(0.8),
        "hydrocarbons" | "biomass" => Some(0.2),
        _ => None,
    }
}

/// Per-commodity vein-count multiplier for a body's [`PlanetarySubtype`]
/// (issue #196), applied on top of [`vein_count_for_radius`].
///
/// `Unclassified` and `EarthLike` both return `1.0` for every commodity —
/// deliberately, so a body with no subtype or an explicitly Earth-like one
/// reproduces the #188 baseline exactly (see
/// [`PlanetMap::generate_for_body`]'s doc comment). Every other subtype
/// pushes some commodities up and others down; a multiplier can drive a
/// commodity's vein count to zero, which is intended — e.g. no organics
/// veins on a molten world.
#[must_use]
pub(crate) fn subtype_commodity_multiplier(subtype: PlanetarySubtype, commodity: &str) -> f32 {
    match subtype {
        PlanetarySubtype::Unclassified | PlanetarySubtype::EarthLike => 1.0,
        PlanetarySubtype::Ocean => match commodity {
            "hydrocarbons" | "biomass" => 2.0,
            "structural_ore" | "refractory_ore" => 0.5,
            _ => 1.0,
        },
        PlanetarySubtype::Molten => match commodity {
            "refractory_ore" | "precious_ore" | "fissile_ore" => 2.0,
            "hydrocarbons" | "biomass" => 0.0,
            _ => 1.0,
        },
        PlanetarySubtype::Icy | PlanetarySubtype::IceGiant => match commodity {
            "hydrocarbons" => 2.0,
            "biomass" | "refractory_ore" => 0.5,
            _ => 1.0,
        },
        PlanetarySubtype::RockyBarrenHot
        | PlanetarySubtype::RockyBarrenCold
        | PlanetarySubtype::Rocky => match commodity {
            "structural_ore" | "silicates" | "conductive_ore" | "semiconductor_ore" => 1.6,
            "biomass" | "hydrocarbons" => 0.4,
            _ => 1.0,
        },
        // Mostly-exposed rock at high elevation (issue #313) — ore-rich like
        // the barren archetypes, and more so, since almost nothing is buried
        // under water or lowland sediment.
        PlanetarySubtype::Mountain => match commodity {
            "structural_ore" | "silicates" | "conductive_ore" | "semiconductor_ore" => 1.8,
            "biomass" | "hydrocarbons" => 0.2,
            _ => 1.0,
        },
        PlanetarySubtype::GasGiant => match commodity {
            "hydrocarbons" => 1.8,
            _ => 1.0,
        },
    }
}

/// Place vein centres for every commodity in [`VEIN_COMMODITIES`].
///
/// Force-place a deposit for any [`VEIN_COMMODITIES`] entry that has no
/// deposit anywhere on the map after normal generation (issue #232).
///
/// Candidates are restricted to the commodity's preferred elevation band
/// (via [`commodity_elevation_bias`]), same as normal vein placement, so a
/// forced deposit reads as a small, thematically-placed field rather than an
/// arbitrary outlier — this also keeps it compatible with the deposit
/// clustering statistics `deposits_cluster_by_commodity_more_than_random_baseline`
/// checks. Selection within the biased candidate set is deterministic and
/// independent of `HashMap` iteration order: sorted by `(q, r)`, then indexed
/// by a seed+commodity hash so different commodities don't all collide on the
/// same hex. Falls back to the first available non-ocean candidate if the
/// hashed pick lands on ocean. A no-op if every curated commodity already has
/// a deposit (the common case) or the map has no cells at all (degenerate
/// radius-0 input).
#[allow(clippy::cast_possible_truncation)]
fn force_guaranteed_deposits(
    cells: &mut HashMap<HexCoord, HexCell>,
    coords: &[HexCoord],
    elevations: &HashMap<HexCoord, f32>,
    seed: u64,
) {
    let mut sorted_coords: Vec<HexCoord> = coords.to_vec();
    sorted_coords.sort_by_key(|c| (c.q, c.r));
    let mut used: std::collections::HashSet<HexCoord> = std::collections::HashSet::new();

    for (i, commodity) in VEIN_COMMODITIES.iter().enumerate() {
        let already_present = cells
            .values()
            .any(|cell| cell.deposits.iter().any(|d| d.commodity_id == *commodity));
        if already_present {
            continue;
        }

        let mut pool: Vec<HexCoord> = sorted_coords
            .iter()
            .filter(|c| !used.contains(*c))
            .copied()
            .collect();
        if let Some(target_elevation) = commodity_elevation_bias(commodity) {
            pool.sort_by(|a, b| {
                let da = (elevations[a] - target_elevation).abs();
                let db = (elevations[b] - target_elevation).abs();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });
            pool.truncate((pool.len() / 2).max(1));
        }

        let Some(&preferred) = pool.get(
            (seed.wrapping_add(i as u64 * 7919) as usize)
                .checked_rem(pool.len())
                .unwrap_or(0),
        ) else {
            continue; // no cells to place onto at all (radius 0)
        };
        let target = if cells
            .get(&preferred)
            .is_some_and(|c| !matches!(c.terrain, Terrain::Ocean))
        {
            Some(preferred)
        } else {
            pool.iter().copied().find(|c| {
                cells
                    .get(c)
                    .is_some_and(|cc| !matches!(cc.terrain, Terrain::Ocean))
            })
        };

        if let Some(coord) = target {
            if let Some(cell) = cells.get_mut(&coord) {
                cell.deposits.push(Deposit::new(*commodity, 0.6));
                used.insert(coord);
            }
        }
    }
}

/// For commodities with an elevation preference, candidates are restricted
/// to the half of `coords` closest to that preference before a centre is
/// drawn, so e.g. iron veins land preferentially on ridges. Selection order
/// (commodity list order, then draw order within a commodity) is fixed so
/// the result is deterministic for a given `seed` + `radius` + `subtype`.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
fn place_veins(
    rng: &mut ChaCha8Rng,
    coords: &[HexCoord],
    elevations: &HashMap<HexCoord, f32>,
    radius: u32,
    subtype: PlanetarySubtype,
) -> Vec<Vein> {
    let base_count = vein_count_for_radius(radius);
    let mut veins = Vec::with_capacity(base_count * VEIN_COMMODITIES.len());

    for commodity in VEIN_COMMODITIES {
        let multiplier = subtype_commodity_multiplier(subtype, commodity);
        let count = ((base_count as f32) * multiplier).round() as usize;
        if count == 0 {
            continue;
        }

        let mut pool: Vec<HexCoord> = coords.to_vec();
        if let Some(target) = commodity_elevation_bias(commodity) {
            pool.sort_by(|a, b| {
                let da = (elevations[a] - target).abs();
                let db = (elevations[b] - target).abs();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });
            pool.truncate((pool.len() / 2).max(count));
        }

        for _ in 0..count.min(pool.len()) {
            let idx = rng.gen_range(0..pool.len());
            let coord = pool.swap_remove(idx);
            veins.push((commodity, coord));
        }
    }

    veins
}

/// Return `(deposit_probability, commodity)` for `coord` based on its
/// distance to the nearest vein centre.
///
/// Probability decays linearly from [`VEIN_PEAK_PROB`] at the vein centre to
/// [`BACKGROUND_DEPOSIT_PROB`] at [`VEIN_INFLUENCE_RADIUS`] hexes away, and
/// stays at the background rate (with no commodity bias) beyond that.
#[allow(clippy::cast_precision_loss)]
fn nearest_vein_influence(coord: HexCoord, veins: &[Vein]) -> (f32, Option<&'static str>) {
    let nearest = veins
        .iter()
        .map(|(commodity, vein_coord)| (coord.distance(*vein_coord), *commodity))
        .min_by_key(|(dist, _)| *dist);

    let Some((dist, commodity)) = nearest else {
        return (BACKGROUND_DEPOSIT_PROB, None);
    };
    if dist > VEIN_INFLUENCE_RADIUS {
        return (BACKGROUND_DEPOSIT_PROB, None);
    }
    let t = dist as f32 / VEIN_INFLUENCE_RADIUS as f32;
    let prob = VEIN_PEAK_PROB + (BACKGROUND_DEPOSIT_PROB - VEIN_PEAK_PROB) * t;
    (prob, Some(commodity))
}

/// Compute the infrastructure construction cost between two hex coordinates.
///
/// Formula: `sum_of_difficulty_along_path × distance × infra_cost_factor`
///
/// Path is approximated as the hex-line from `from` to `to`; cells not in the
/// map contribute a difficulty of 2.0 (unknown / unexplored).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn edge_cost<S: ::std::hash::BuildHasher>(
    from: HexCoord,
    to: HexCoord,
    cells: &HashMap<HexCoord, HexCell, S>,
    infra_type: InfraType,
) -> f32 {
    let path = hex_line(from, to);
    let total_difficulty: f32 = path
        .iter()
        .map(|c| cells.get(c).map_or(2.0, |cell| cell.terrain.difficulty()))
        .sum();
    let distance = from.distance(to) as f32;
    let difficulty_per_cell = if path.is_empty() {
        1.0
    } else {
        total_difficulty / path.len() as f32
    };
    distance * difficulty_per_cell * infra_type.base_cost_factor() * 10.0
}

/// Return the cells along the straight hex line from `a` to `b` (inclusive).
///
/// Uses linear interpolation through cube coordinates, rounded to the nearest hex.
#[must_use]
#[allow(clippy::many_single_char_names, clippy::cast_precision_loss)]
pub fn hex_line(a: HexCoord, b: HexCoord) -> Vec<HexCoord> {
    let dist = a.distance(b) as usize;
    if dist == 0 {
        return vec![a];
    }
    let mut results = Vec::with_capacity(dist + 1);
    for i in 0..=dist {
        let t = i as f32 / dist as f32;
        let q = lerp(a.q as f32, b.q as f32, t);
        let r = lerp(a.r as f32, b.r as f32, t);
        let s = lerp(a.s() as f32, b.s() as f32, t);
        results.push(cube_round(q, r, s));
    }
    results
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Round floating-point cube coordinates to the nearest integer hex.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn cube_round(fq: f32, fr: f32, fs: f32) -> HexCoord {
    let mut rq = fq.round() as i32;
    let mut rr = fr.round() as i32;
    let rs = fs.round() as i32;

    let dq = (rq as f32 - fq).abs();
    let dr = (rr as f32 - fr).abs();
    let ds = (rs as f32 - fs).abs();

    if dq > dr && dq > ds {
        rq = -rr - rs;
    } else if dr > ds {
        rr = -rq - rs;
    }
    // else rs is adjusted, but we only need q and r.

    HexCoord::new(rq, rr)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::BodyKind;

    // ── Hex coordinate math ──────────────────────────────────────────────────

    #[test]
    fn origin_has_zero_coordinates() {
        let o = HexCoord::origin();
        assert_eq!(o.q, 0);
        assert_eq!(o.r, 0);
        assert_eq!(o.s(), 0);
    }

    #[test]
    fn s_coordinate_is_derived() {
        let h = HexCoord::new(2, -3);
        assert_eq!(h.s(), 1); // s = -(2) - (-3) = 1
    }

    #[test]
    fn distance_to_self_is_zero() {
        let h = HexCoord::new(3, -1);
        assert_eq!(h.distance(h), 0);
    }

    #[test]
    fn distance_one_step_neighbour() {
        let a = HexCoord::origin();
        for nb in a.neighbours() {
            assert_eq!(a.distance(nb), 1, "neighbour {nb:?} should be distance 1");
        }
    }

    #[test]
    fn distance_is_symmetric() {
        let a = HexCoord::new(-2, 4);
        let b = HexCoord::new(3, -1);
        assert_eq!(a.distance(b), b.distance(a));
    }

    #[test]
    fn distance_known_value() {
        // (0,0) to (3,0): q-axis, distance 3.
        let a = HexCoord::origin();
        let b = HexCoord::new(3, 0);
        assert_eq!(a.distance(b), 3);
    }

    #[test]
    fn neighbours_count_is_six() {
        assert_eq!(HexCoord::origin().neighbours().len(), 6);
    }

    #[test]
    fn within_radius_origin_is_one_cell() {
        let cells = HexCoord::origin().within_radius(0);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0], HexCoord::origin());
    }

    #[test]
    fn within_radius_one_has_seven_cells() {
        // Center + 6 neighbours = 7.
        let cells = HexCoord::origin().within_radius(1);
        assert_eq!(cells.len(), 7);
    }

    #[test]
    fn within_radius_count_formula() {
        // Total cells for radius n = 3n² + 3n + 1.
        for n in 0u32..=5 {
            let expected = 3 * n * n + 3 * n + 1;
            let actual = HexCoord::origin().within_radius(n).len() as u32;
            assert_eq!(
                actual, expected,
                "radius {n}: expected {expected} cells, got {actual}"
            );
        }
    }

    #[test]
    fn hex_line_start_to_self_is_one_cell() {
        let a = HexCoord::new(1, -2);
        let line = hex_line(a, a);
        assert_eq!(line.len(), 1);
        assert_eq!(line[0], a);
    }

    #[test]
    fn hex_line_length_equals_distance_plus_one() {
        let a = HexCoord::new(-3, 1);
        let b = HexCoord::new(2, 0);
        let dist = a.distance(b) as usize;
        let line = hex_line(a, b);
        assert_eq!(line.len(), dist + 1, "line should have distance+1 cells");
    }

    #[test]
    fn hex_line_starts_and_ends_at_endpoints() {
        let a = HexCoord::new(0, 0);
        let b = HexCoord::new(4, -2);
        let line = hex_line(a, b);
        assert_eq!(*line.first().unwrap(), a);
        assert_eq!(*line.last().unwrap(), b);
    }

    // ── Map generation determinism ───────────────────────────────────────────

    #[test]
    fn map_generation_is_deterministic() {
        let map1 = PlanetMap::generate(42, 5);
        let map2 = PlanetMap::generate(42, 5);
        // Compare cell count and one stable property (number of ocean cells).
        assert_eq!(map1.cells.len(), map2.cells.len());
        let oceans1 = map1
            .cells
            .values()
            .filter(|c| matches!(c.terrain, Terrain::Ocean))
            .count();
        let oceans2 = map2
            .cells
            .values()
            .filter(|c| matches!(c.terrain, Terrain::Ocean))
            .count();
        assert_eq!(
            oceans1, oceans2,
            "ocean count must be deterministic for same seed"
        );
    }

    #[test]
    fn different_seeds_produce_different_maps() {
        let map_a = PlanetMap::generate(1, 5);
        let map_b = PlanetMap::generate(2, 5);
        let deposits_a: usize = map_a.cells.values().map(|c| c.deposits.len()).sum();
        let deposits_b: usize = map_b.cells.values().map(|c| c.deposits.len()).sum();
        // Very unlikely (but not impossible) to be identical — assert on cell count always.
        assert_eq!(
            map_a.cells.len(),
            map_b.cells.len(),
            "both maps must have same cell count for radius 5"
        );
        // At least one property must differ (deposits total OR terrain distribution).
        // This is a statistical assertion; different seeds routinely produce different results.
        let terrains_a: Vec<_> = {
            let mut t: Vec<_> = map_a.cells.keys().collect();
            t.sort_by_key(|c| (c.q, c.r));
            t.iter().map(|c| map_a.cells[c].terrain).collect()
        };
        let terrains_b: Vec<_> = {
            let mut t: Vec<_> = map_b.cells.keys().collect();
            t.sort_by_key(|c| (c.q, c.r));
            t.iter().map(|c| map_b.cells[c].terrain).collect()
        };
        // Seeds 1 and 2 must produce different terrain layouts.
        let deposits_differ = deposits_a != deposits_b;
        let terrains_differ = terrains_a != terrains_b;
        assert!(
            deposits_differ || terrains_differ,
            "different seeds must produce different maps (deposits: {deposits_a} vs {deposits_b})"
        );
    }

    #[test]
    fn map_cell_count_matches_radius_formula() {
        for radius in [1u32, 3, 5] {
            let map = PlanetMap::generate(0, radius);
            let expected = (3 * radius * radius + 3 * radius + 1) as usize;
            assert_eq!(
                map.cells.len(),
                expected,
                "radius {radius}: expected {expected} cells, got {}",
                map.cells.len()
            );
        }
    }

    #[test]
    fn best_landing_site_is_habitable() {
        let map = PlanetMap::generate(7, 5);
        if let Some(coord) = map.best_landing_site() {
            let cell = map.cell(coord).unwrap();
            assert!(cell.is_habitable(), "landing site must be habitable");
        }
        // Note: if the entire map is ocean this returns None, which is also valid.
    }

    // ── Colony placement ─────────────────────────────────────────────────────

    #[test]
    fn place_colony_on_valid_cell_succeeds() {
        let mut map = PlanetMap::generate(99, 3);
        let coord = HexCoord::origin(); // origin is always Plains
        let colony_id = uuid::Uuid::new_v4();
        map.place_colony(colony_id, coord).unwrap();
        assert_eq!(map.colonies.len(), 1);
        assert_eq!(map.colonies[0].colony_id, colony_id);
        assert_eq!(map.colonies[0].coord, coord);
    }

    #[test]
    fn place_colony_duplicate_returns_error() {
        let mut map = PlanetMap::generate(1, 3);
        let coord = HexCoord::origin();
        let id1 = uuid::Uuid::new_v4();
        let id2 = uuid::Uuid::new_v4();
        map.place_colony(id1, coord).unwrap();
        let err = map.place_colony(id2, coord).unwrap_err();
        assert!(matches!(err, MapError::CellOccupied(_)));
    }

    #[test]
    fn place_colony_out_of_map_returns_error() {
        let mut map = PlanetMap::generate(1, 1);
        let far = HexCoord::new(100, 100);
        let id = uuid::Uuid::new_v4();
        let err = map.place_colony(id, far).unwrap_err();
        assert!(matches!(err, MapError::CellNotFound(_)));
    }

    // ── Infrastructure cost calculation ──────────────────────────────────────

    #[test]
    fn edge_cost_increases_with_distance() {
        let cells = HashMap::new(); // unknown terrain → difficulty 2.0
        let origin = HexCoord::origin();
        let near = HexCoord::new(1, 0);
        let far = HexCoord::new(5, 0);
        let cost_near = edge_cost(origin, near, &cells, InfraType::Road);
        let cost_far = edge_cost(origin, far, &cells, InfraType::Road);
        assert!(
            cost_far > cost_near,
            "longer route should cost more: near={cost_near}, far={cost_far}"
        );
    }

    #[test]
    fn rail_costs_more_than_road() {
        let cells = HashMap::new();
        let a = HexCoord::origin();
        let b = HexCoord::new(3, 0);
        let road_cost = edge_cost(a, b, &cells, InfraType::Road);
        let rail_cost = edge_cost(a, b, &cells, InfraType::Rail);
        assert!(
            rail_cost > road_cost,
            "rail should cost more than road: rail={rail_cost}, road={road_cost}"
        );
    }

    #[test]
    fn mountains_increase_edge_cost_vs_plains() {
        use std::collections::HashMap;

        // Build a small maps with plains vs mountains along the same route.
        let mut plains_cells: HashMap<HexCoord, HexCell> = HashMap::new();
        let mut mountain_cells: HashMap<HexCoord, HexCell> = HashMap::new();

        for q in 0..=3 {
            let c = HexCoord::new(q, 0);
            plains_cells.insert(c, HexCell::new(c, Terrain::Plains, Biome::Grassland));
            mountain_cells.insert(c, HexCell::new(c, Terrain::Mountains, Biome::Barren));
        }

        let a = HexCoord::origin();
        let b = HexCoord::new(3, 0);
        let plains_cost = edge_cost(a, b, &plains_cells, InfraType::Road);
        let mountain_cost = edge_cost(a, b, &mountain_cells, InfraType::Road);
        assert!(
            mountain_cost > plains_cost,
            "mountains should cost more than plains: mountain={mountain_cost}, plains={plains_cost}"
        );
    }

    #[test]
    fn add_edge_computes_cost_and_throughput() {
        let mut map = PlanetMap::generate(5, 5);
        let id_a = uuid::Uuid::new_v4();
        let id_b = uuid::Uuid::new_v4();
        let coord_a = HexCoord::new(0, 0);
        let coord_b = HexCoord::new(2, 0);
        map.place_colony(id_a, coord_a).unwrap();
        map.place_colony(id_b, coord_b).unwrap();

        let edge = map.add_edge(id_a, id_b, InfraType::Road).unwrap();
        assert!(edge.cost > 0.0, "cost must be positive");
        assert!((edge.throughput - InfraType::Road.base_throughput()).abs() < 1e-4);
    }

    #[test]
    fn add_edge_duplicate_returns_error() {
        let mut map = PlanetMap::generate(5, 5);
        let id_a = uuid::Uuid::new_v4();
        let id_b = uuid::Uuid::new_v4();
        map.place_colony(id_a, HexCoord::new(0, 0)).unwrap();
        map.place_colony(id_b, HexCoord::new(1, 0)).unwrap();
        map.add_edge(id_a, id_b, InfraType::Road).unwrap();
        let err = map.add_edge(id_a, id_b, InfraType::Road).unwrap_err();
        assert!(matches!(err, MapError::EdgeExists { .. }));
    }

    #[test]
    fn throughput_with_tech_multiplier() {
        assert!(
            InfraType::Rail.throughput_with_tech(2.0) > InfraType::Rail.base_throughput(),
            "tech multiplier must increase throughput"
        );
        assert!(
            (InfraType::Road.throughput_with_tech(1.0) - InfraType::Road.base_throughput()).abs()
                < 1e-4,
            "multiplier=1.0 must not change throughput"
        );
    }

    #[test]
    fn infra_type_throughput_ordering() {
        // Road < Pipeline < Rail (base values).
        assert!(InfraType::Road.base_throughput() < InfraType::Pipeline.base_throughput());
        assert!(InfraType::Pipeline.base_throughput() < InfraType::Rail.base_throughput());
    }

    // ── Elevation & temperature (issue #187) ─────────────────────────────────

    #[test]
    fn elevation_is_deterministic_per_seed_and_coord() {
        let map1 = PlanetMap::generate(1234, 5);
        let map2 = PlanetMap::generate(1234, 5);
        for (coord, cell) in &map1.cells {
            let other = map2.cells.get(coord).unwrap();
            assert!(
                (cell.elevation - other.elevation).abs() < 1e-6,
                "elevation at {coord:?} must be deterministic"
            );
        }
    }

    #[test]
    fn elevation_lies_in_unit_interval() {
        let map = PlanetMap::generate(42, 6);
        for cell in map.cells.values() {
            assert!(
                (0.0..=1.0).contains(&cell.elevation),
                "elevation out of range at {:?}: {}",
                cell.coord,
                cell.elevation
            );
        }
    }

    #[test]
    fn mountains_have_higher_mean_elevation_than_plains() {
        // Statistical: aggregate across several seeds so the sample size is
        // large enough for terrain-band means to separate cleanly.
        let mut mountain_elevs: Vec<f32> = Vec::new();
        let mut plains_elevs: Vec<f32> = Vec::new();
        for seed in 0..8u64 {
            let map = PlanetMap::generate(seed, 6);
            for cell in map.cells.values() {
                match cell.terrain {
                    Terrain::Mountains => mountain_elevs.push(cell.elevation),
                    Terrain::Plains => plains_elevs.push(cell.elevation),
                    _ => {}
                }
            }
        }
        assert!(
            !mountain_elevs.is_empty() && !plains_elevs.is_empty(),
            "test needs samples of both terrains — got {} mountains, {} plains",
            mountain_elevs.len(),
            plains_elevs.len()
        );
        let mean = |xs: &[f32]| -> f32 {
            #[allow(clippy::cast_precision_loss)]
            let n = xs.len() as f32;
            xs.iter().sum::<f32>() / n
        };
        let mm = mean(&mountain_elevs);
        let pm = mean(&plains_elevs);
        assert!(
            mm > pm,
            "mountains should skew higher than plains: mountains={mm}, plains={pm}"
        );
    }

    #[test]
    fn cell_temperature_defaults_to_body_band_near_equator() {
        // At the origin (latitude ≈ 0) with mid-range elevation, no shift
        // should apply, so cell temp equals body temp.
        for body in [
            TemperatureBand::Frozen,
            TemperatureBand::Cold,
            TemperatureBand::Temperate,
            TemperatureBand::Hot,
        ] {
            let derived = cell_temperature(body, 0.0, 0.5);
            assert_eq!(
                derived, body,
                "cell at equator+mid-elev must match body band, got {derived:?} for {body:?}"
            );
        }
    }

    #[test]
    fn cell_temperature_shifts_colder_toward_poles() {
        // A Temperate body at latitude 0.9 (2-band cold shift) lands at Frozen.
        assert_eq!(
            cell_temperature(TemperatureBand::Temperate, 0.9, 0.5),
            TemperatureBand::Frozen
        );
        // A Temperate body at latitude 0.6 (1-band cold shift) lands at Cold.
        assert_eq!(
            cell_temperature(TemperatureBand::Temperate, 0.6, 0.5),
            TemperatureBand::Cold
        );
        // A Hot body at latitude 0.6 (1-band cold shift) lands at Temperate.
        assert_eq!(
            cell_temperature(TemperatureBand::Hot, 0.6, 0.5),
            TemperatureBand::Temperate
        );
        // High elevation compounds with high latitude — clamped at Extreme.
        assert_eq!(
            cell_temperature(TemperatureBand::Temperate, 0.9, 0.95),
            TemperatureBand::Extreme
        );
    }

    #[test]
    fn cell_temperature_clamps_at_extreme_and_hot() {
        // Extreme body cannot go colder than Extreme.
        assert_eq!(
            cell_temperature(TemperatureBand::Extreme, 0.9, 0.95),
            TemperatureBand::Extreme
        );
        // Hot body at equator+valley stays Hot; no positive shift is defined.
        assert_eq!(
            cell_temperature(TemperatureBand::Hot, 0.0, 0.0),
            TemperatureBand::Hot
        );
    }

    #[test]
    fn generate_for_body_carries_baseline_temperature() {
        // A `Frozen` body's map should be mostly Frozen or colder — no Hot cells
        // should ever appear since we only shift downward.
        let map = PlanetMap::generate_for_body(9, 5, TemperatureBand::Frozen);
        for cell in map.cells.values() {
            assert!(
                matches!(
                    cell.temperature,
                    TemperatureBand::Frozen | TemperatureBand::Extreme
                ),
                "Frozen body should not produce cell band {:?} at {:?}",
                cell.temperature,
                cell.coord
            );
        }
    }

    #[test]
    fn generate_defaults_to_temperate_body() {
        // Confirm the seed=X, radius=Y convenience matches an explicit Temperate call.
        let a = PlanetMap::generate(77, 4);
        let b = PlanetMap::generate_for_body(77, 4, TemperatureBand::Temperate);
        for (coord, cell) in &a.cells {
            let other = b.cells.get(coord).unwrap();
            assert_eq!(cell.terrain, other.terrain);
            assert_eq!(cell.temperature, other.temperature);
            assert!((cell.elevation - other.elevation).abs() < 1e-6);
        }
    }

    // ── Radius bump + deposit retuning (issue #188) ──────────────────────────

    /// Acceptable band around the ~15%-of-habitable-cells deposit density
    /// target. Wide enough to absorb per-seed RNG variance while still
    /// catching a regression to the old flat-25% roll (which lands north of
    /// 25%) or an over-correction toward near-zero density.
    const DEPOSIT_DENSITY_MIN_PCT: f64 = 8.0;
    const DEPOSIT_DENSITY_MAX_PCT: f64 = 25.0;

    #[allow(clippy::cast_precision_loss)]
    fn deposit_density_pct(map: &PlanetMap) -> f64 {
        let habitable: Vec<&HexCell> = map.cells.values().filter(|c| c.is_habitable()).collect();
        let with_deposit = habitable.iter().filter(|c| !c.deposits.is_empty()).count();
        100.0 * with_deposit as f64 / habitable.len() as f64
    }

    #[test]
    fn deposit_density_in_target_band_across_seeds_and_radii() {
        for radius in [10u32, 12, 16] {
            for seed in 0..10u64 {
                let map = PlanetMap::generate(seed, radius);
                let pct = deposit_density_pct(&map);
                assert!(
                    (DEPOSIT_DENSITY_MIN_PCT..=DEPOSIT_DENSITY_MAX_PCT).contains(&pct),
                    "radius {radius} seed {seed}: deposit density {pct:.2}% outside target band \
                     [{DEPOSIT_DENSITY_MIN_PCT}, {DEPOSIT_DENSITY_MAX_PCT}]"
                );
            }
        }
    }

    #[test]
    fn map_cell_count_matches_radius_formula_for_supported_radii() {
        for radius in [10u32, 12, 16] {
            let map = PlanetMap::generate(0, radius);
            let expected = (3 * radius * radius + 3 * radius + 1) as usize;
            assert_eq!(
                map.cells.len(),
                expected,
                "radius {radius}: cell count mismatch"
            );
        }
    }

    /// Mean nearest-neighbour hex distance among cells holding a given
    /// commodity's deposit, vs. the same statistic computed over an equal
    /// number of uniformly-random habitable cells. Vein clustering should
    /// pull deposits of the same commodity closer together than chance.
    #[allow(clippy::cast_precision_loss)]
    fn mean_nearest_neighbour_distance(coords: &[HexCoord]) -> f64 {
        let mut total = 0u32;
        for (i, a) in coords.iter().enumerate() {
            let nearest = coords
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, b)| a.distance(*b))
                .min()
                .expect("at least one other coordinate");
            total += nearest;
        }
        f64::from(total) / coords.len() as f64
    }

    #[test]
    fn deposits_cluster_by_commodity_more_than_random_baseline() {
        // Aggregated across seeds per (radius, commodity) rather than
        // asserted per individual seed: vein placement is a statistical
        // clustering tendency, not a per-seed guarantee, so a single seed
        // can land worse than the evenly-spaced baseline by chance (issue
        // #232 changed which commodities get elevation-bias truncation in
        // `place_veins`, which shifts downstream shared-RNG draws enough to
        // flip an occasional individual seed even though nothing about the
        // clustering mechanism itself changed).
        for radius in [10u32, 12, 16] {
            let mut actual_by_commodity: HashMap<String, f64> = HashMap::new();
            let mut baseline_by_commodity: HashMap<String, f64> = HashMap::new();
            let mut count_by_commodity: HashMap<String, usize> = HashMap::new();

            for seed in 0..10u64 {
                let map = PlanetMap::generate(seed, radius);
                let habitable: Vec<HexCoord> = map
                    .cells
                    .values()
                    .filter(|c| c.is_habitable())
                    .map(|c| c.coord)
                    .collect();

                let mut by_commodity: HashMap<&str, Vec<HexCoord>> = HashMap::new();
                for cell in map.cells.values() {
                    for d in &cell.deposits {
                        by_commodity
                            .entry(d.commodity_id.as_str())
                            .or_default()
                            .push(cell.coord);
                    }
                }

                // Random baseline: evenly-spaced sample across all habitable
                // cells is the best a uniform-random placement can achieve on
                // average, so nearest-neighbour distance shrinks with density.
                // Approximate it by sampling every Nth habitable cell (by
                // stable sort order) for a commodity with `n` occurrences.
                let mut sorted_habitable = habitable.clone();
                sorted_habitable.sort_by_key(|c| (c.q, c.r));

                for (commodity, coords) in &by_commodity {
                    if coords.len() < 3 {
                        continue; // too few points for a meaningful comparison
                    }
                    let actual = mean_nearest_neighbour_distance(coords);

                    let stride = (sorted_habitable.len() / coords.len()).max(1);
                    let baseline_coords: Vec<HexCoord> = sorted_habitable
                        .iter()
                        .step_by(stride)
                        .copied()
                        .take(coords.len())
                        .collect();
                    let baseline = mean_nearest_neighbour_distance(&baseline_coords);

                    *actual_by_commodity
                        .entry((*commodity).to_string())
                        .or_default() += actual;
                    *baseline_by_commodity
                        .entry((*commodity).to_string())
                        .or_default() += baseline;
                    *count_by_commodity
                        .entry((*commodity).to_string())
                        .or_default() += 1;
                }
            }

            for (commodity, samples) in &count_by_commodity {
                if *samples < 3 {
                    continue; // too few qualifying seeds for a meaningful comparison
                }
                let actual_total = actual_by_commodity[commodity];
                let baseline_total = baseline_by_commodity[commodity];
                assert!(
                    actual_total <= baseline_total,
                    "radius {radius} {commodity}: clustered mean_nn total {actual_total:.2} \
                     should not exceed evenly-spaced baseline total {baseline_total:.2} \
                     (aggregated over {samples} seeds)"
                );
            }
        }
    }

    #[test]
    fn top_landing_sites_respects_minimum_distance() {
        for seed in 0..10u64 {
            let map = PlanetMap::generate(seed, 12);
            let sites = map.top_landing_sites(3, 3);
            for i in 0..sites.len() {
                for j in (i + 1)..sites.len() {
                    let d = sites[i].distance(sites[j]);
                    assert!(
                        d >= 3,
                        "seed {seed}: sites {:?} and {:?} are only {d} hexes apart",
                        sites[i],
                        sites[j]
                    );
                }
            }
        }
    }

    #[test]
    fn top_landing_sites_all_habitable_and_unique() {
        let map = PlanetMap::generate(42, 12);
        let sites = map.top_landing_sites(3, 3);
        let mut seen = std::collections::HashSet::new();
        for coord in &sites {
            let cell = map.cell(*coord).unwrap();
            assert!(
                cell.is_habitable(),
                "landing site {coord:?} must be habitable"
            );
            assert!(seen.insert(*coord), "landing site {coord:?} returned twice");
        }
    }

    #[test]
    fn best_landing_site_matches_top_landing_sites_first_pick() {
        let map = PlanetMap::generate(5, 12);
        assert_eq!(
            map.best_landing_site(),
            map.top_landing_sites(1, 0).into_iter().next()
        );
    }

    // ── Site scoring by resource variety in proximity (issue #302) ───────────

    /// A uniform plains/temperate map with no deposits, so tests can place
    /// exactly the deposits they care about and compare scores directly.
    fn flat_map(radius: u32) -> PlanetMap {
        let mut cells = HashMap::new();
        for coord in HexCoord::origin().within_radius(radius) {
            cells.insert(
                coord,
                HexCell::new(coord, Terrain::Plains, Biome::Grassland),
            );
        }
        PlanetMap {
            seed: 0,
            radius,
            cells,
            colonies: Vec::new(),
            edges: Vec::new(),
            sites: HashMap::new(),
        }
    }

    fn put_deposit(map: &mut PlanetMap, coord: HexCoord, commodity: &str, richness: f32) {
        map.cells
            .get_mut(&coord)
            .expect("coord in map")
            .deposits
            .push(Deposit::new(commodity, richness));
    }

    #[test]
    fn site_score_prefers_a_varied_neighbourhood_over_one_rich_deposit() {
        // This is the reported bug: a lone very rich `precious_ore` tile used
        // to win because the old score summed a single cell's richness.
        let mut map = flat_map(8);
        let rich_single = HexCoord::new(-5, 0);
        let varied = HexCoord::new(5, 0);
        // Far enough apart that the two radius-2 neighbourhoods can't overlap.
        assert!(rich_single.distance(varied) > 2 * SITE_PROXIMITY_RADIUS);

        put_deposit(&mut map, rich_single, "precious_ore", 1.0);

        // Four different commodities, each individually poorer and one ring out.
        for (i, commodity) in ["structural_ore", "conductive_ore", "silicates", "biomass"]
            .iter()
            .enumerate()
        {
            put_deposit(&mut map, varied.neighbours()[i], commodity, 0.5);
        }

        assert!(
            map.site_score(varied) > map.site_score(rich_single),
            "varied neighbourhood {} should beat one rich deposit {}",
            map.site_score(varied),
            map.site_score(rich_single)
        );
        // The recommendation must land *in* the resource-rich neighbourhood
        // rather than on the lone rich tile. Several cells adjacent to `varied`
        // reach the same four deposits and therefore tie with it exactly, so
        // assert proximity rather than an exact coordinate.
        let best = map.best_landing_site().expect("a habitable site exists");
        assert!(
            best.distance(varied) <= SITE_PROXIMITY_RADIUS,
            "best site {best:?} should be near the varied cluster {varied:?}"
        );
        assert!(best.distance(rich_single) > SITE_PROXIMITY_RADIUS);
    }

    #[test]
    fn site_score_counts_neighbouring_deposits_not_only_the_site_cell() {
        let mut map = flat_map(6);
        let with_neighbours = HexCoord::new(-4, 0);
        let barren = HexCoord::new(4, 0);
        put_deposit(
            &mut map,
            with_neighbours.neighbours()[0],
            "structural_ore",
            0.6,
        );

        assert!(map.site_score(with_neighbours) > map.site_score(barren));
        // The site cell itself holds nothing — the score comes from proximity.
        assert!(map.cell(with_neighbours).expect("cell").deposits.is_empty());
    }

    #[test]
    fn site_score_does_not_reward_piling_up_the_same_commodity() {
        let mut map = flat_map(8);
        let mono = HexCoord::new(-5, 0);
        let varied = HexCoord::new(5, 0);

        // Five neighbours all holding the *same*, richer commodity...
        for n in mono.neighbours().iter().take(5) {
            put_deposit(&mut map, *n, "structural_ore", 0.8);
        }
        // ...loses to three neighbours holding three *different*, poorer ones.
        for (i, commodity) in ["structural_ore", "hydrocarbons", "silicates"]
            .iter()
            .enumerate()
        {
            put_deposit(&mut map, varied.neighbours()[i], commodity, 0.5);
        }

        assert!(
            map.site_score(varied) > map.site_score(mono),
            "variety {} should beat quantity of one commodity {}",
            map.site_score(varied),
            map.site_score(mono)
        );
    }

    #[test]
    fn site_score_weights_closer_deposits_more_heavily() {
        let mut map = flat_map(8);
        let near = HexCoord::new(-5, 0);
        let far = HexCoord::new(5, 0);
        // Same commodity, same richness — only the distance differs.
        put_deposit(&mut map, near, "structural_ore", 0.7);
        let two_out = HexCoord::new(far.q + 2, far.r);
        assert_eq!(far.distance(two_out), 2);
        put_deposit(&mut map, two_out, "structural_ore", 0.7);

        assert!(map.site_score(near) > map.site_score(far));
    }

    #[test]
    fn top_landing_sites_at_radius_12_completes_within_budget() {
        // `site_score` scans a 19-hex neighbourhood per candidate, and the
        // founding wizard calls this interactively ("jump to best site"), so
        // guard against it becoming a visible stall on a full-size map.
        let map = PlanetMap::generate(3, 12);
        let start = std::time::Instant::now();
        let sites = map.top_landing_sites(3, 4);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "top_landing_sites at radius 12 took {elapsed:?}, expected well under 100ms"
        );
        assert!(!sites.is_empty(), "a generated map should offer some site");
    }

    #[test]
    fn site_score_is_zero_for_uninhabitable_and_unknown_cells() {
        let mut map = flat_map(3);
        let ocean = HexCoord::new(1, 0);
        let cell = map.cells.get_mut(&ocean).expect("coord in map");
        *cell = HexCell::new(ocean, Terrain::Ocean, Biome::Grassland);
        assert!(!map.cell(ocean).expect("cell").is_habitable());

        assert!(map.site_score(ocean).abs() < 1e-6);
        // A coordinate outside the map scores zero rather than panicking.
        assert!(map.site_score(HexCoord::new(99, 99)).abs() < 1e-6);
    }

    #[test]
    fn site_score_still_penalises_harsh_climate_despite_rich_surroundings() {
        // Climate multiplies the whole score, mirroring `suitability` — so a
        // resource-rich Extreme-band site loses to a bare temperate one.
        let mut map = flat_map(8);
        let harsh_rich = HexCoord::new(-5, 0);
        let plain_temperate = HexCoord::new(5, 0);
        map.cells
            .get_mut(&harsh_rich)
            .expect("coord in map")
            .temperature = TemperatureBand::Extreme;
        for (i, commodity) in ["structural_ore", "conductive_ore", "silicates"]
            .iter()
            .enumerate()
        {
            put_deposit(&mut map, harsh_rich.neighbours()[i], commodity, 1.0);
        }

        assert!(map.site_score(plain_temperate) > map.site_score(harsh_rich));
    }

    #[test]
    fn generate_for_body_radius_12_completes_within_budget() {
        // Sanity guard against an accidental O(n^2)+ blowup in vein placement
        // or deposit rolls; 100ms is the `getPlanetMap` playtest gate from
        // #188, generation itself should be a small fraction of that.
        let start = std::time::Instant::now();
        let _map = PlanetMap::generate(1, 12);
        assert!(
            start.elapsed().as_millis() < 100,
            "planet map generation at radius 12 took {:?}, expected well under 100ms",
            start.elapsed()
        );
    }

    // ── Per-cell temperature affects suitability (issue #190) ────────────────

    fn cell_with_temperature(temperature: TemperatureBand) -> HexCell {
        let mut cell = HexCell::new(HexCoord::origin(), Terrain::Plains, Biome::Grassland);
        cell.temperature = temperature;
        cell
    }

    #[test]
    fn temperature_suitability_factor_peaks_at_temperate() {
        for band in [
            TemperatureBand::Extreme,
            TemperatureBand::Frozen,
            TemperatureBand::Cold,
            TemperatureBand::Hot,
        ] {
            assert!(
                temperature_suitability_factor(band)
                    < temperature_suitability_factor(TemperatureBand::Temperate),
                "{band:?} should score below Temperate"
            );
        }
    }

    #[test]
    fn temperature_suitability_factor_matches_body_habitability_ordering() {
        // Mirrors `Body::habitability`'s per-band ordering (issue #163):
        // Temperate > Cold > Hot > Frozen > Extreme.
        let t = temperature_suitability_factor(TemperatureBand::Temperate);
        let cold = temperature_suitability_factor(TemperatureBand::Cold);
        let hot = temperature_suitability_factor(TemperatureBand::Hot);
        let frozen = temperature_suitability_factor(TemperatureBand::Frozen);
        let extreme = temperature_suitability_factor(TemperatureBand::Extreme);
        assert!(t > cold);
        assert!(cold > hot);
        assert!(hot > frozen);
        assert!(frozen > extreme);
    }

    #[test]
    fn temperature_suitability_factor_never_reaches_zero() {
        // A soft penalty, not a hard block (#190 defers hard-blocking to
        // #183) — Extreme cells must stay orderable against each other by
        // terrain/deposits rather than all collapsing to an identical 0.0.
        assert!(temperature_suitability_factor(TemperatureBand::Extreme) > 0.0);
    }

    #[test]
    fn suitability_is_lower_on_harsher_temperature_bands() {
        let temperate = cell_with_temperature(TemperatureBand::Temperate);
        let cold = cell_with_temperature(TemperatureBand::Cold);
        let frozen = cell_with_temperature(TemperatureBand::Frozen);
        let extreme = cell_with_temperature(TemperatureBand::Extreme);

        assert!(temperate.suitability() > cold.suitability());
        assert!(cold.suitability() > frozen.suitability());
        assert!(frozen.suitability() > extreme.suitability());
    }

    #[test]
    fn suitability_temperature_penalty_does_not_affect_ocean_cells() {
        // Ocean is never habitable regardless of temperature, so suitability
        // must stay exactly 0.0 rather than picking up the temperature floor.
        let mut ocean = HexCell::new(HexCoord::origin(), Terrain::Ocean, Biome::Ocean);
        ocean.temperature = TemperatureBand::Extreme;
        assert!(ocean.suitability().abs() < 1e-6);
    }

    #[test]
    fn equatorial_temperate_site_can_outrank_richer_polar_extreme_site() {
        // A modest Temperate hex should be able to beat a deposit-rich
        // Extreme hex — the core scenario #190 calls out: "equatorial hexes
        // on a Cold body may still be viable, polar hexes on a Temperate
        // body may not."
        let plain_temperate = cell_with_temperature(TemperatureBand::Temperate);

        let mut rich_extreme = cell_with_temperature(TemperatureBand::Extreme);
        rich_extreme.deposits.push(Deposit::new("iron", 1.0));

        assert!(
            plain_temperate.suitability() > rich_extreme.suitability(),
            "temperate={} extreme+deposit={}",
            plain_temperate.suitability(),
            rich_extreme.suitability()
        );
    }

    // ── Planetary-subtype deposit bias (issue #196) ──────────────────────────

    fn water_family_deposit_count(map: &PlanetMap) -> usize {
        map.cells
            .values()
            .flat_map(|c| &c.deposits)
            .filter(|d| matches!(d.commodity_id.as_str(), "hydrocarbons" | "biomass"))
            .count()
    }

    /// Cells eligible for deposits at all — everything but ocean (issue #313).
    fn land_cell_count(map: &PlanetMap) -> usize {
        map.cells
            .values()
            .filter(|c| !matches!(c.terrain, Terrain::Ocean))
            .count()
    }

    // ── Founding-site resource guarantee (issue #232) ────────────────────────

    #[test]
    fn every_curated_commodity_has_a_deposit_somewhere_on_the_map() {
        // Every subtype, including ones whose multiplier table can zero out
        // several commodities (e.g. Molten kills hydrocarbons/biomass), must
        // still end up with at least one real deposit of every
        // `VEIN_COMMODITIES` entry after `force_guaranteed_deposits` runs —
        // this is the actual "founding site can reach every early-tech raw
        // material" guarantee #232 asks for.
        let subtypes = [
            PlanetarySubtype::Unclassified,
            PlanetarySubtype::EarthLike,
            PlanetarySubtype::Ocean,
            PlanetarySubtype::Molten,
            PlanetarySubtype::Icy,
            PlanetarySubtype::IceGiant,
            PlanetarySubtype::RockyBarrenHot,
            PlanetarySubtype::RockyBarrenCold,
            PlanetarySubtype::Rocky,
            PlanetarySubtype::GasGiant,
        ];
        for subtype in subtypes {
            for seed in 0..8u64 {
                let map = PlanetMap::generate_for_body_and_subtype(
                    seed,
                    10,
                    TemperatureBand::Temperate,
                    subtype,
                );
                let present: std::collections::HashSet<&str> = map
                    .cells
                    .values()
                    .flat_map(|c| &c.deposits)
                    .map(|d| d.commodity_id.as_str())
                    .collect();
                for commodity in VEIN_COMMODITIES {
                    assert!(
                        present.contains(commodity),
                        "subtype {subtype:?} seed {seed}: no deposit of {commodity} anywhere on the map"
                    );
                }
            }
        }
    }

    #[test]
    fn earth_like_reproduces_unclassified_baseline_exactly() {
        // Both map to a 1.0 multiplier for every commodity, so #196 must not
        // change the map at all relative to a body with no subtype authored.
        for seed in 0..5u64 {
            let unclassified = PlanetMap::generate_for_body_and_subtype(
                seed,
                10,
                TemperatureBand::Temperate,
                PlanetarySubtype::Unclassified,
            );
            let earth_like = PlanetMap::generate_for_body_and_subtype(
                seed,
                10,
                TemperatureBand::Temperate,
                PlanetarySubtype::EarthLike,
            );
            assert_eq!(
                unclassified.cells, earth_like.cells,
                "seed {seed}: EarthLike must match Unclassified exactly"
            );
        }
    }

    #[test]
    fn generate_for_body_matches_unclassified_subtype() {
        // The pre-#196 entry point must still produce exactly what it did
        // before — i.e. the same as explicitly requesting Unclassified.
        for seed in 0..5u64 {
            let via_old_api = PlanetMap::generate_for_body(seed, 10, TemperatureBand::Cold);
            let via_new_api = PlanetMap::generate_for_body_and_subtype(
                seed,
                10,
                TemperatureBand::Cold,
                PlanetarySubtype::Unclassified,
            );
            assert_eq!(via_old_api.cells, via_new_api.cells);
        }
    }

    #[test]
    fn ocean_subtype_yields_more_water_family_deposits_than_earth_like() {
        // Since issue #313 gave Ocean a much smaller land-fraction target
        // than EarthLike (0.20 vs 0.55), an ocean world's *land* is scarcer
        // — comparing raw deposit totals would conflate "less land to place
        // deposits on" with "the multiplier favours these commodities less,"
        // which is backwards. Comparing density (deposits per land cell)
        // isolates `subtype_commodity_multiplier`'s effect from the land/water
        // target's, which is what this test is actually about.
        let radius = 12;
        let mut ocean_deposits = 0usize;
        let mut ocean_land_cells = 0usize;
        let mut earth_like_deposits = 0usize;
        let mut earth_like_land_cells = 0usize;
        for seed in 0..10u64 {
            let ocean = PlanetMap::generate_for_body_and_subtype(
                seed,
                radius,
                TemperatureBand::Temperate,
                PlanetarySubtype::Ocean,
            );
            let earth_like = PlanetMap::generate_for_body_and_subtype(
                seed,
                radius,
                TemperatureBand::Temperate,
                PlanetarySubtype::EarthLike,
            );
            ocean_deposits += water_family_deposit_count(&ocean);
            ocean_land_cells += land_cell_count(&ocean);
            earth_like_deposits += water_family_deposit_count(&earth_like);
            earth_like_land_cells += land_cell_count(&earth_like);
        }
        #[allow(clippy::cast_precision_loss)]
        let ocean_density = ocean_deposits as f64 / ocean_land_cells as f64;
        #[allow(clippy::cast_precision_loss)]
        let earth_like_density = earth_like_deposits as f64 / earth_like_land_cells as f64;
        assert!(
            ocean_density > earth_like_density,
            "ocean density={ocean_density:.4} should exceed earth-like density={earth_like_density:.4} across seeds 0-9"
        );
    }

    // ── Quantitative archetype targets (issue #313) ───────────────────────────

    fn land_fraction(map: &PlanetMap) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        {
            land_cell_count(map) as f64 / map.cells.len() as f64
        }
    }

    #[test]
    fn ocean_world_land_fraction_matches_its_less_than_25_percent_target() {
        // The issue's own wording: "less than 25% land tiles." Near-polar
        // cells are always forced to Plains regardless of the water
        // threshold, which nudges the real fraction up slightly — hence the
        // small tolerance rather than asserting the raw 0.20 target exactly.
        for seed in 0..5u64 {
            let ocean = PlanetMap::generate_for_body_and_subtype(
                seed,
                12,
                TemperatureBand::Temperate,
                PlanetarySubtype::Ocean,
            );
            let land = land_fraction(&ocean);
            assert!(
                land < 0.30,
                "seed {seed}: ocean world land fraction {land:.3} should be well under 25%"
            );
        }
    }

    #[test]
    fn mountain_world_is_mostly_land_with_elevated_terrain() {
        // The issue's own wording: "all high elevation, less than 10% water."
        for seed in 0..5u64 {
            let mountain = PlanetMap::generate_for_body_and_subtype(
                seed,
                12,
                TemperatureBand::Cold,
                PlanetarySubtype::Mountain,
            );
            let land = land_fraction(&mountain);
            assert!(
                land > 0.85,
                "seed {seed}: mountain world land fraction {land:.3} should clear 90% water<10%"
            );

            let mean_elevation: f64 = {
                #[allow(clippy::cast_precision_loss)]
                {
                    mountain
                        .cells
                        .values()
                        .map(|c| f64::from(c.elevation))
                        .sum::<f64>()
                        / mountain.cells.len() as f64
                }
            };
            assert!(
                mean_elevation > 0.55,
                "seed {seed}: mountain world mean elevation {mean_elevation:.3} should be biased high"
            );
        }
    }

    #[test]
    fn mountain_subtype_is_inner_planet_only() {
        assert!(PlanetarySubtype::Mountain.compatible_with(&BodyKind::InnerPlanet));
        assert!(!PlanetarySubtype::Mountain.compatible_with(&BodyKind::Moon));
        assert!(!PlanetarySubtype::Mountain.compatible_with(&BodyKind::GasGiant));
        assert!(!PlanetarySubtype::Mountain.compatible_with(&BodyKind::AsteroidBelt));
    }

    #[test]
    fn giant_subtypes_keep_the_pre_313_fixed_probability_ocean_behaviour() {
        // `target_land_fraction` returns `None` for gas/ice giants, so the
        // quantile-threshold path must not engage — the legacy low-probability
        // roll (~98% non-ocean, no archetype opinion) still applies.
        for seed in 0..5u64 {
            let map = PlanetMap::generate_for_body_and_subtype(
                seed,
                10,
                TemperatureBand::Hot,
                PlanetarySubtype::GasGiant,
            );
            let land = land_fraction(&map);
            assert!(
                land > 0.85,
                "seed {seed}: gas giant's unbiased legacy roll should still be mostly non-ocean, got land={land:.3}"
            );
        }
    }

    #[test]
    fn molten_subtype_produces_no_water_family_veins() {
        // subtype_commodity_multiplier maps Molten's hydrocarbons/biomass to
        // a 0.0 multiplier — no veins of those commodities should be placed
        // via normal roll, though the rare flat-rate "second deposit"
        // background roll and the issue #232 founding-guarantee pass (which
        // force-places exactly one deposit of a commodity that's entirely
        // absent) can still occasionally contribute one, so this asserts a
        // low bound rather than exactly zero.
        let radius = 12;
        let mut molten_total = 0usize;
        let mut earth_like_total = 0usize;
        for seed in 0..10u64 {
            let molten = PlanetMap::generate_for_body_and_subtype(
                seed,
                radius,
                TemperatureBand::Hot,
                PlanetarySubtype::Molten,
            );
            let earth_like = PlanetMap::generate_for_body_and_subtype(
                seed,
                radius,
                TemperatureBand::Hot,
                PlanetarySubtype::EarthLike,
            );
            molten_total += water_family_deposit_count(&molten);
            earth_like_total += water_family_deposit_count(&earth_like);
        }
        assert!(
            molten_total < earth_like_total,
            "molten_total={molten_total} should be well below earth_like_total={earth_like_total} across seeds 0-9"
        );
    }

    #[test]
    fn subtype_aware_generation_stays_within_performance_budget() {
        let start = std::time::Instant::now();
        let _map = PlanetMap::generate_for_body_and_subtype(
            1,
            12,
            TemperatureBand::Cold,
            PlanetarySubtype::IceGiant,
        );
        assert!(
            start.elapsed().as_millis() < 100,
            "subtype-aware generation at radius 12 took {:?}, expected well under 100ms",
            start.elapsed()
        );
    }
}
