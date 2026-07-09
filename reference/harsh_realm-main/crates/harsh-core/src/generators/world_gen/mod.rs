//! Procedural world generator.
//!
//! Ported from `src/harsh_realm/generators/world_gen.py` plus its three mixins
//! (`_world_region_mixin`, `_world_features_mixin`, `_world_settlements_mixin`).
//! The Python class composed the mixins; here a single [`WorldGenerator`] struct
//! carries the region logic (this file) with feature placement and settlement
//! enhancement in sibling submodules.

mod features;
mod settlements;

use std::collections::HashMap;

use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use crate::cell::TerrainRegistry;
use crate::generation::{TerrainCountMap, WorldGenerationSummary};
use crate::generators::biome::{classify_forest_biome, FOREST_BIOME_KEY};
use crate::generators::water::{classify_water_kind, WATER_KIND_KEY};
use crate::generators::world_support::load_terrain_weights;
use crate::grid::{Grid, GridCoord, SquareGrid};
use crate::runtime::{JsonObject, JsonValue};

/// A terrain assignment grid: present key = filled, value = terrain id.
type TerrainGrid = HashMap<(i32, i32), String>;
/// Per-cell feature lists, mutated during feature placement.
type CellFeatures = HashMap<(i32, i32), Vec<String>>;
/// Per-cell feature data payloads, mutated during feature placement.
type HexData = HashMap<(i32, i32), JsonObject>;

const IMPASSABLE_CHOICES: [&str; 2] = ["water", "mountains"];

/// Generates a procedural grid world region and persists it to the database.
pub struct WorldGenerator<'a> {
    db: &'a crate::db::WorldDatabase,
    terrain_registry: TerrainRegistry,
    data_dir: Option<std::path::PathBuf>,
    grid: SquareGrid,
}

impl<'a> WorldGenerator<'a> {
    /// Create a world generator over a database and terrain registry.
    ///
    /// When `data_dir` is set, settlement cells are enhanced with a full
    /// [`crate::generators::SettlementGenerator`] pass after terrain/features.
    pub fn new(
        db: &'a crate::db::WorldDatabase,
        terrain_registry: TerrainRegistry,
        data_dir: Option<std::path::PathBuf>,
    ) -> Self {
        WorldGenerator {
            db,
            terrain_registry,
            data_dir,
            grid: SquareGrid::new(),
        }
    }

    fn passable_ids(&self) -> Vec<String> {
        self.terrain_registry
            .passable_types()
            .into_iter()
            .map(|t| t.id.clone())
            .collect()
    }

    fn get_neighbors_coords(&self, q: i32, r: i32, width: i32, height: i32) -> Vec<(i32, i32)> {
        self.grid
            .neighbors(GridCoord::new(q, r))
            .into_iter()
            .filter(|nb| self.grid.is_valid(*nb, width, height))
            .map(|nb| (nb.q, nb.r))
            .collect()
    }

    /// Generate a complete grid world region and persist it to the database.
    pub fn generate_region(
        &self,
        width: i32,
        height: i32,
        seed: Option<u64>,
    ) -> Result<WorldGenerationSummary, String> {
        let mut rng = match seed {
            Some(s) => SmallRng::seed_from_u64(s),
            None => SmallRng::from_entropy(),
        };

        let weights = load_terrain_weights().unwrap_or_default();
        let base_weights = weights.base_weights.clone();
        let adjacency = weights.adjacency_modifiers.clone();

        // Choose 1-3 bounded edges; the remaining sides are open.
        let all_sides = ["N", "S", "E", "W"];
        let num_bounded = rng.gen_range(1..=3);
        let bounded_sides: Vec<&str> = all_sides
            .choose_multiple(&mut rng, num_bounded)
            .copied()
            .collect();
        let open_sides: Vec<&str> = all_sides
            .iter()
            .copied()
            .filter(|s| !bounded_sides.contains(s))
            .collect();
        let open_edge_side = open_sides
            .choose(&mut rng)
            .copied()
            .or_else(|| all_sides.choose(&mut rng).copied())
            .unwrap_or("N")
            .to_string();

        let on_bounded_edge = |q: i32, r: i32| -> bool {
            let side = border_side(q, r, width, height);
            side.map(|s| bounded_sides.contains(&s)).unwrap_or(false)
        };

        // Bounded edge cells get impassable terrain.
        let mut grid: TerrainGrid = HashMap::new();
        for q in 0..width {
            for r in 0..height {
                if on_bounded_edge(q, r) {
                    let choice = *IMPASSABLE_CHOICES.choose(&mut rng).expect("nonempty");
                    grid.insert((q, r), choice.to_string());
                }
            }
        }

        let open_edge_cells: Vec<(i32, i32)> = (0..width)
            .flat_map(|q| (0..height).map(move |r| (q, r)))
            .filter(|&(q, r)| is_border(q, r, width, height) && !on_bounded_edge(q, r))
            .collect();

        let passable_ids = self.passable_ids();

        // Fill terrain, retrying until at least 3 passable terrain types appear.
        for attempt in 0..=3 {
            self.fill_terrain(
                &mut grid,
                width,
                height,
                &base_weights,
                &adjacency,
                &passable_ids,
                &on_bounded_edge,
                &mut rng,
            );
            let distinct: std::collections::HashSet<&String> = grid
                .values()
                .filter(|t| passable_ids.contains(t))
                .collect();
            if distinct.len() >= 3 || attempt == 3 {
                break;
            }
        }

        // Place starting settlement on a passable open-edge cell (or nearest).
        let starting = self.choose_starting_settlement(&grid, &open_edge_cells, &passable_ids, &mut rng);

        let mut cell_features: CellFeatures = HashMap::new();
        let mut hex_data: HexData = HashMap::new();
        if let Some((sq, sr)) = starting {
            cell_features.insert((sq, sr), vec!["settlement".to_string()]);
            let mut settlement = JsonObject::new();
            settlement.insert("name".into(), JsonValue::from("Millhaven"));
            settlement.insert("size".into(), JsonValue::from("village"));
            settlement.insert("starting".into(), JsonValue::from(true));
            let mut data = JsonObject::new();
            data.insert("settlement".into(), JsonValue::Object(settlement));
            hex_data.insert((sq, sr), data);
        }

        self.place_features(
            width,
            height,
            &mut rng,
            &grid,
            &mut cell_features,
            &mut hex_data,
            starting,
        );

        let water_coords: std::collections::HashSet<(i32, i32)> = grid
            .iter()
            .filter(|(_, t)| t.as_str() == "water")
            .map(|(pos, _)| *pos)
            .collect();

        // Write all cells.
        for q in 0..width {
            for r in 0..height {
                let terrain = grid.get(&(q, r)).cloned().unwrap_or_else(|| "plains".to_string());
                let features = cell_features.get(&(q, r)).cloned().unwrap_or_default();
                let mut data = hex_data.get(&(q, r)).cloned().unwrap_or_default();
                if terrain == "water" {
                    data.insert(
                        WATER_KIND_KEY.into(),
                        JsonValue::from(classify_water_kind(q, r, &water_coords)),
                    );
                }
                if terrain == "forest" {
                    let neighbors: Vec<String> = self
                        .get_neighbors_coords(q, r, width, height)
                        .into_iter()
                        .filter_map(|(nq, nr)| grid.get(&(nq, nr)).cloned())
                        .collect();
                    data.insert(
                        FOREST_BIOME_KEY.into(),
                        JsonValue::from(classify_forest_biome(r, height, &neighbors)),
                    );
                }
                let features_json = serde_json::to_string(&features).map_err(|e| e.to_string())?;
                let data_json = serde_json::to_string(&data).map_err(|e| e.to_string())?;
                let none: Option<String> = None;
                self.db.execute(
                    "INSERT OR REPLACE INTO cells \
                     (q, r, terrain, features, explored, description, faction_id, data) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    &[
                        &q,
                        &r,
                        &terrain,
                        &features_json,
                        &0_i64,
                        &none,
                        &none,
                        &data_json,
                    ],
                )?;
            }
        }

        if self.data_dir.is_some() {
            self.enhance_settlements(&grid, &cell_features, &mut rng);
        }

        let mut terrain_counts: TerrainCountMap = TerrainCountMap::new();
        for t in grid.values() {
            *terrain_counts.entry(t.clone()).or_insert(0) += 1;
        }

        Ok(WorldGenerationSummary {
            width,
            height,
            cell_count: width * height,
            terrain_counts,
            open_edge: open_edge_side,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn fill_terrain(
        &self,
        grid: &mut TerrainGrid,
        width: i32,
        height: i32,
        base_weights: &crate::generation::TerrainWeightMap,
        adjacency: &crate::generation::AdjacencyWeightMap,
        passable_ids: &[String],
        on_bounded_edge: &impl Fn(i32, i32) -> bool,
        rng: &mut SmallRng,
    ) {
        // Reset non-bounded cells.
        grid.retain(|&(q, r), _| on_bounded_edge(q, r));

        // Seed 5-10 interior cells with passable terrain.
        let interior: Vec<(i32, i32)> = (1..width - 1)
            .flat_map(|q| (1..height - 1).map(move |r| (q, r)))
            .filter(|&(q, r)| !on_bounded_edge(q, r))
            .collect();
        if !interior.is_empty() && !passable_ids.is_empty() {
            let num_seeds = rng.gen_range(5..=10.min(interior.len()));
            for &(sq, sr) in interior.choose_multiple(rng, num_seeds) {
                let terrain = passable_ids.choose(rng).expect("nonempty").clone();
                grid.insert((sq, sr), terrain);
            }
        }

        // Fill remaining cells in scan order.
        for q in 0..width {
            for r in 0..height {
                if grid.contains_key(&(q, r)) {
                    continue;
                }
                let filled_neighbors: Vec<String> = self
                    .get_neighbors_coords(q, r, width, height)
                    .into_iter()
                    .filter_map(|(nq, nr)| grid.get(&(nq, nr)).cloned())
                    .collect();
                let chosen = weighted_terrain_choice(rng, &filled_neighbors, base_weights, adjacency);
                grid.insert((q, r), chosen);
            }
        }
    }

    fn choose_starting_settlement(
        &self,
        grid: &TerrainGrid,
        open_edge_cells: &[(i32, i32)],
        passable_ids: &[String],
        rng: &mut SmallRng,
    ) -> Option<(i32, i32)> {
        let is_passable = |pos: &(i32, i32)| grid.get(pos).map(|t| passable_ids.contains(t)).unwrap_or(false);
        let passable_open: Vec<(i32, i32)> = open_edge_cells.iter().copied().filter(is_passable).collect();
        if !passable_open.is_empty() {
            return passable_open.choose(rng).copied();
        }
        if open_edge_cells.is_empty() {
            return None;
        }
        // Fall back to the passable cell closest to any open-edge cell.
        let mut all_passable: Vec<(i32, i32)> = grid
            .iter()
            .filter(|(_, t)| passable_ids.contains(t))
            .map(|(pos, _)| *pos)
            .collect();
        if all_passable.is_empty() {
            return None;
        }
        let min_dist = |pos: &(i32, i32)| -> i32 {
            open_edge_cells
                .iter()
                .map(|&(oq, or)| self.grid.distance(GridCoord::new(pos.0, pos.1), GridCoord::new(oq, or)))
                .min()
                .unwrap_or(i32::MAX)
        };
        all_passable.sort_by_key(min_dist);
        all_passable.first().copied()
    }
}

fn is_border(q: i32, r: i32, width: i32, height: i32) -> bool {
    q == 0 || q == width - 1 || r == 0 || r == height - 1
}

fn border_side(q: i32, r: i32, width: i32, height: i32) -> Option<&'static str> {
    if r == 0 {
        Some("N")
    } else if r == height - 1 {
        Some("S")
    } else if q == 0 {
        Some("W")
    } else if q == width - 1 {
        Some("E")
    } else {
        None
    }
}

/// Select a terrain via weighted random choice with neighbour adjacency modifiers.
fn weighted_terrain_choice(
    rng: &mut SmallRng,
    neighbor_terrains: &[String],
    base_weights: &crate::generation::TerrainWeightMap,
    adjacency: &crate::generation::AdjacencyWeightMap,
) -> String {
    let mut weights = base_weights.clone();
    for neighbor in neighbor_terrains {
        if let Some(modifiers) = adjacency.get(neighbor) {
            for (terrain_id, multiplier) in modifiers {
                if let Some(w) = weights.get_mut(terrain_id) {
                    *w *= *multiplier;
                }
            }
        }
    }
    let items: Vec<(&String, f64)> = weights.iter().map(|(k, v)| (k, *v)).collect();
    if items.is_empty() {
        return "plains".to_string();
    }
    let total: f64 = items.iter().map(|(_, w)| w.max(0.0)).sum();
    if total <= 0.0 {
        return items[0].0.clone();
    }
    let mut roll = rng.gen_range(0.0..total);
    for (id, w) in &items {
        roll -= w.max(0.0);
        if roll < 0.0 {
            return (*id).clone();
        }
    }
    items.last().expect("nonempty").0.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;

    fn registry() -> TerrainRegistry {
        let t = |id: &str, passable: bool| {
            serde_json::from_value::<crate::cell::TerrainType>(serde_json::json!({
                "id": id, "name": id, "passable": passable, "blocks_vision": false
            }))
            .unwrap()
        };
        TerrainRegistry::from_types(vec![
            t("plains", true),
            t("forest", true),
            t("hills", true),
            t("swamp", true),
            t("water", false),
            t("mountains", false),
        ])
    }

    #[test]
    fn generates_region_and_writes_cells() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let gen = WorldGenerator::new(&db, registry(), None);
        let summary = gen.generate_region(12, 10, Some(42)).unwrap();
        assert_eq!(summary.width, 12);
        assert_eq!(summary.cell_count, 120);
        assert!(["N", "S", "E", "W"].contains(&summary.open_edge.as_str()));
        // All cells were written.
        let count = db
            .fetch_one("SELECT COUNT(*) AS n FROM cells", &[])
            .unwrap()
            .unwrap();
        assert_eq!(count.get("n").unwrap().as_i64(), Some(120));
    }

    #[test]
    fn deterministic_for_same_seed() {
        let db1 = WorldDatabase::open_in_memory().unwrap();
        let db2 = WorldDatabase::open_in_memory().unwrap();
        let s1 = WorldGenerator::new(&db1, registry(), None)
            .generate_region(10, 10, Some(7))
            .unwrap();
        let s2 = WorldGenerator::new(&db2, registry(), None)
            .generate_region(10, 10, Some(7))
            .unwrap();
        assert_eq!(s1.terrain_counts, s2.terrain_counts);
        assert_eq!(s1.open_edge, s2.open_edge);
    }

    #[test]
    fn generation_places_searchable_containers() {
        // HR-786: across several seeds, generated regions place unsearched
        // container loot sources (crates/caches) on some ruins/lair cells.
        let mut containers: Vec<crate::loot_source::LootSource> = Vec::new();
        for seed in [1u64, 2, 3, 4, 5] {
            let db = WorldDatabase::open_in_memory().unwrap();
            WorldGenerator::new(&db, registry(), None)
                .generate_region(20, 20, Some(seed))
                .unwrap();
            let rows = db.fetch_all("SELECT data FROM cells", &[]).unwrap();
            for row in &rows {
                if let Some(s) = row.get("data").and_then(|v| v.as_str()) {
                    if let Ok(JsonValue::Object(data)) = serde_json::from_str::<JsonValue>(s) {
                        for src in crate::loot_source::read_sources(&data) {
                            if src.kind == crate::loot_source::KIND_CONTAINER {
                                containers.push(src);
                            }
                        }
                    }
                }
            }
        }
        assert!(
            !containers.is_empty(),
            "generation must place at least one searchable container"
        );
        // A placed container is unsearched and holds loot.
        let c = &containers[0];
        assert!(!c.searched, "placed containers start unsearched");
        assert!(c.has_loot(), "placed containers hold items/gold");
        assert!(c.difficulty > 0, "placed containers require a search check");
    }

    #[test]
    fn weighted_choice_respects_zeroed_weights() {
        let mut rng = SmallRng::seed_from_u64(1);
        let mut base = crate::generation::TerrainWeightMap::new();
        base.insert("plains".into(), 1.0);
        base.insert("forest".into(), 0.0);
        let adjacency = crate::generation::AdjacencyWeightMap::new();
        // forest has zero weight, so only plains can be chosen.
        for _ in 0..20 {
            assert_eq!(weighted_terrain_choice(&mut rng, &[], &base, &adjacency), "plains");
        }
    }
}
