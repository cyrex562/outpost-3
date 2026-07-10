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

use rand::SeedableRng;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::colony::ColonyId;

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
        const DIRECTIONS: [(i32, i32); 6] = [
            (1, 0),
            (1, -1),
            (0, -1),
            (-1, 0),
            (-1, 1),
            (0, 1),
        ];
        DIRECTIONS.map(|(dq, dr)| HexCoord::new(self.q + dq, self.r + dr))
    }

    /// Return all cells within `radius` steps of this cell (inclusive of center).
    #[must_use]
    pub fn within_radius(self, radius: u32) -> Vec<HexCoord> {
        let r = radius as i32;
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

/// A single hex cell in the planet map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HexCell {
    /// Axial coordinate of this cell.
    pub coord: HexCoord,
    /// Surface terrain class.
    pub terrain: Terrain,
    /// Surface biome class.
    pub biome: Biome,
    /// Resource deposits present in this cell (may be empty).
    pub deposits: Vec<Deposit>,
}

impl HexCell {
    /// Construct a cell with no deposits.
    #[must_use]
    pub fn new(coord: HexCoord, terrain: Terrain, biome: Biome) -> Self {
        Self {
            coord,
            terrain,
            biome,
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
    /// Higher is better.  Deposits increase the score; difficult terrain reduces it.
    #[must_use]
    pub fn suitability(&self) -> f32 {
        if !self.is_habitable() {
            return 0.0;
        }
        let base = 10.0 / self.terrain.difficulty();
        let deposit_bonus: f32 = self.deposits.iter().map(|d| d.richness * 5.0).sum();
        base + deposit_bonus
    }
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
    pub cells: HashMap<HexCoord, HexCell>,
    /// Colony nodes placed on this map.
    pub colonies: Vec<ColonyNode>,
    /// Infrastructure edges connecting colony nodes.
    pub edges: Vec<InfraEdge>,
}

impl PlanetMap {
    /// Generate a planet map deterministically from `seed` and `radius`.
    ///
    /// All cells within the hex radius of the origin are generated.
    #[must_use]
    pub fn generate(seed: u64, radius: u32) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let coords = HexCoord::origin().within_radius(radius);
        let mut cells = HashMap::with_capacity(coords.len());

        for coord in &coords {
            let cell = generate_cell(&mut rng, *coord, radius);
            cells.insert(*coord, cell);
        }

        Self {
            seed,
            radius,
            cells,
            colonies: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Return a reference to a cell by coordinate, if it exists.
    #[must_use]
    pub fn cell(&self, coord: HexCoord) -> Option<&HexCell> {
        self.cells.get(&coord)
    }

    /// Return the best landing site (highest suitability) in the map.
    ///
    /// Returns `None` only if the map has no habitable cells (degenerate).
    #[must_use]
    pub fn best_landing_site(&self) -> Option<HexCoord> {
        self.cells
            .values()
            .max_by(|a, b| a.suitability().partial_cmp(&b.suitability()).unwrap_or(std::cmp::Ordering::Equal))
            .filter(|c| c.is_habitable())
            .map(|c| c.coord)
    }

    /// Place a colony node at the given hex coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`MapError::CellNotFound`] if the coordinate is not in this map.
    /// Returns [`MapError::CellNotHabitable`] if the terrain is ocean.
    /// Returns [`MapError::CellOccupied`] if a colony already exists at that coordinate.
    pub fn place_colony(
        &mut self,
        colony_id: ColonyId,
        coord: HexCoord,
    ) -> Result<(), MapError> {
        let cell = self.cells.get(&coord).ok_or(MapError::CellNotFound(coord))?;
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

/// Generate a single hex cell at `coord` using the given RNG.
fn generate_cell(rng: &mut ChaCha8Rng, coord: HexCoord, radius: u32) -> HexCell {
    // Rough distance from centre as a fraction [0, 1].
    let dist_frac = if radius == 0 {
        0.0
    } else {
        HexCoord::origin().distance(coord) as f32 / radius as f32
    };

    // Pick terrain based on position noise.
    let terrain_roll: f32 = rng.gen();
    let terrain = if dist_frac < 0.15 {
        // Near-polar regions favour flat plains.
        Terrain::Plains
    } else if terrain_roll < 0.02 {
        Terrain::Ocean
    } else if terrain_roll < 0.08 {
        Terrain::Volcanic
    } else if terrain_roll < 0.20 {
        Terrain::Mountains
    } else if terrain_roll < 0.35 {
        Terrain::Hills
    } else if terrain_roll < 0.45 {
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

    let mut cell = HexCell::new(coord, terrain, biome);

    // Seed deposits with low probability.
    if !matches!(terrain, Terrain::Ocean) {
        if rng.gen::<f32>() < 0.25 {
            let richness: f32 = rng.gen::<f32>() * 0.9 + 0.1;
            let commodity = pick_deposit_commodity(rng, &biome);
            cell.deposits.push(Deposit::new(commodity, richness));
        }
        // Rare second deposit.
        if rng.gen::<f32>() < 0.06 {
            let richness: f32 = rng.gen::<f32>() * 0.5 + 0.05;
            let commodity = pick_deposit_commodity(rng, &biome);
            cell.deposits.push(Deposit::new(commodity, richness));
        }
    }

    cell
}

/// Choose a deposit commodity influenced by biome.
fn pick_deposit_commodity(rng: &mut ChaCha8Rng, biome: &Biome) -> &'static str {
    let roll: f32 = rng.gen();
    match biome {
        Biome::Desert | Biome::Barren => {
            if roll < 0.4 { "iron" } else if roll < 0.7 { "silicates" } else { "rare_metals" }
        }
        Biome::Tundra | Biome::Polar => {
            if roll < 0.5 { "water_ice" } else if roll < 0.75 { "methane" } else { "iron" }
        }
        Biome::Geothermal => {
            if roll < 0.5 { "sulfur" } else { "geothermal_energy" }
        }
        _ => {
            if roll < 0.35 { "iron" } else if roll < 0.60 { "water" } else if roll < 0.80 { "organics" } else { "rare_metals" }
        }
    }
}

/// Compute the infrastructure construction cost between two hex coordinates.
///
/// Formula: `sum_of_difficulty_along_path × distance × infra_cost_factor`
///
/// Path is approximated as the hex-line from `from` to `to`; cells not in the
/// map contribute a difficulty of 2.0 (unknown / unexplored).
#[must_use]
pub fn edge_cost(
    from: HexCoord,
    to: HexCoord,
    cells: &HashMap<HexCoord, HexCell>,
    infra_type: InfraType,
) -> f32 {
    let path = hex_line(from, to);
    let total_difficulty: f32 = path
        .iter()
        .map(|c| {
            cells
                .get(c)
                .map_or(2.0, |cell| cell.terrain.difficulty())
        })
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
            assert_eq!(actual, expected, "radius {n}: expected {expected} cells, got {actual}");
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
        let oceans1 = map1.cells.values().filter(|c| matches!(c.terrain, Terrain::Ocean)).count();
        let oceans2 = map2.cells.values().filter(|c| matches!(c.terrain, Terrain::Ocean)).count();
        assert_eq!(oceans1, oceans2, "ocean count must be deterministic for same seed");
    }

    #[test]
    fn different_seeds_produce_different_maps() {
        let map_a = PlanetMap::generate(1, 5);
        let map_b = PlanetMap::generate(2, 5);
        let deposits_a: usize = map_a.cells.values().map(|c| c.deposits.len()).sum();
        let deposits_b: usize = map_b.cells.values().map(|c| c.deposits.len()).sum();
        // Very unlikely (but not impossible) to be identical — assert on cell count always.
        assert_eq!(map_a.cells.len(), map_b.cells.len(), "both maps must have same cell count for radius 5");
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
}
