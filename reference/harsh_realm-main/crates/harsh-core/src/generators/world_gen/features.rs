//! World generator: feature placement (settlements, ruins, landmarks, lairs, camps).
//!
//! Ported from `src/harsh_realm/generators/_world_features_mixin.py`.

use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use crate::generators::content_tables::load_table_results;
use crate::generators::world_support::load_name_list;
use crate::grid::{Grid, GridCoord};
use crate::loot_source;
use crate::packs::paths::default_content_dir;
use crate::runtime::{JsonObject, JsonValue};
use crate::table_engine::TableEngine;

use super::{CellFeatures, HexData, TerrainGrid, WorldGenerator};

/// Flavor items a placed container (HR-786) may hold.
const CONTAINER_ITEMS: &[&str] = &[
    "Rusted Blade",
    "Pretech Fragment",
    "Faded Star Chart",
    "Cracked Vial",
    "Tarnished Locket",
];

/// Build a searchable, unsearched container loot source for a placed feature.
/// The player must `search` the cell (skill/luck check vs `difficulty`) to
/// reveal it, then `take` its contents.
fn make_container(
    rng: &mut SmallRng,
    pos: (i32, i32),
    name: &str,
    difficulty: i32,
) -> loot_source::LootSource {
    let mut item = JsonObject::new();
    item.insert(
        "name".into(),
        JsonValue::from(CONTAINER_ITEMS[rng.gen_range(0..CONTAINER_ITEMS.len())]),
    );
    item.insert("type".into(), JsonValue::from("misc"));
    item.insert("value".into(), JsonValue::from(rng.gen_range(30..=180)));
    loot_source::LootSource {
        id: format!("container_{}_{}", pos.0, pos.1),
        kind: loot_source::KIND_CONTAINER.to_string(),
        name: name.to_string(),
        contents: vec![item],
        gold: rng.gen_range(5..=40),
        difficulty,
        searched: false,
        empty: false,
    }
}

fn names_or_default(rel_path: &str, default: &[&str]) -> Vec<String> {
    let names = load_name_list(rel_path).map(|d| d.names).unwrap_or_default();
    if names.is_empty() {
        default.iter().map(|s| s.to_string()).collect()
    } else {
        names
    }
}

impl WorldGenerator<'_> {
    /// Place settlements, ruins, landmarks, lairs, and camps after terrain gen.
    pub(super) fn place_features(
        &self,
        width: i32,
        height: i32,
        rng: &mut SmallRng,
        terrain_grid: &TerrainGrid,
        cell_features: &mut CellFeatures,
        hex_data: &mut HexData,
        starting: Option<(i32, i32)>,
    ) {
        let mut settlement_names = names_or_default(
            "tables/names/settlement_names.yaml",
            &["Millhaven", "Ashford", "Greywick"],
        );
        let mut ruin_names = names_or_default("tables/names/ruin_names.yaml", &["Old Ruin"]);
        let mut landmark_names =
            names_or_default("tables/names/landmark_names.yaml", &["Standing Stone"]);
        settlement_names.shuffle(rng);
        ruin_names.shuffle(rng);
        landmark_names.shuffle(rng);

        let passable_ids = self.passable_ids();
        let is_passable = |pos: &(i32, i32)| {
            terrain_grid.get(pos).map(|t| passable_ids.contains(t)).unwrap_or(false)
        };

        // Unoccupied passable cells, preferred terrain first.
        let passable_hexes = |cell_features: &CellFeatures, preferred: &[&str]| -> Vec<(i32, i32)> {
            let all: Vec<(i32, i32)> = (0..width)
                .flat_map(|q| (0..height).map(move |r| (q, r)))
                .filter(|pos| is_passable(pos) && cell_features.get(pos).map(|f| f.is_empty()).unwrap_or(true))
                .collect();
            if preferred.is_empty() {
                return all;
            }
            let in_pref = |pos: &(i32, i32)| {
                terrain_grid.get(pos).map(|t| preferred.contains(&t.as_str())).unwrap_or(false)
            };
            let mut ordered: Vec<(i32, i32)> = all.iter().copied().filter(in_pref).collect();
            ordered.extend(all.iter().copied().filter(|p| !in_pref(p)));
            ordered
        };

        let min_dist_to_settlements = |cell_features: &CellFeatures, pos: (i32, i32)| -> i32 {
            let coord = GridCoord::new(pos.0, pos.1);
            cell_features
                .iter()
                .filter(|(_, feats)| feats.iter().any(|f| f == "settlement"))
                .map(|(&(sq, sr), _)| self.grid.distance(coord, GridCoord::new(sq, sr)))
                .min()
                .unwrap_or(9999)
        };

        // --- Settlements ---
        let mut name_idx = 0;
        if let Some(pos) = starting {
            let sname = settlement_names[name_idx % settlement_names.len()].clone();
            name_idx += 1;
            if let Some(JsonValue::Object(s)) =
                hex_data.entry(pos).or_default().get_mut("settlement")
            {
                s.insert("name".into(), JsonValue::from(sname));
            }
        }

        let target_settlements = rng.gen_range(3..=5);
        let mut current_count = if starting.is_some() { 1 } else { 0 };
        let mut attempts = 0;
        while current_count < target_settlements && attempts < width * height {
            attempts += 1;
            let candidates = passable_hexes(cell_features, &["plains", "hills"]);
            let spaced: Vec<(i32, i32)> = candidates
                .into_iter()
                .filter(|&c| min_dist_to_settlements(cell_features, c) >= 3)
                .collect();
            if spaced.is_empty() {
                break;
            }
            let top: Vec<(i32, i32)> = spaced.into_iter().take(20).collect();
            let pos = *top.choose(rng).expect("nonempty");
            let sname = settlement_names[name_idx % settlement_names.len()].clone();
            name_idx += 1;
            let sizes = load_table_results(
                "tables/settlement_sizes.yaml",
                &["hamlet".to_string(), "village".to_string(), "town".to_string()],
            );
            let size = sizes.choose(rng).cloned().unwrap_or_else(|| "hamlet".to_string());
            cell_features.entry(pos).or_default().push("settlement".to_string());
            let mut s = JsonObject::new();
            s.insert("name".into(), JsonValue::from(sname));
            s.insert("size".into(), JsonValue::from(size));
            hex_data.entry(pos).or_default().insert("settlement".into(), JsonValue::Object(s));
            current_count += 1;
        }

        // --- Ruins ---
        let num_ruins = rng.gen_range(4..=8);
        let mut placed = 0;
        let mut ruin_idx = 0;
        let mut ruin_candidates = passable_hexes(cell_features, &["wasteland", "plains", "hills", "desert"]);
        ruin_candidates.shuffle(rng);
        for pos in ruin_candidates {
            if placed >= num_ruins {
                break;
            }
            if cell_features.get(&pos).map(|f| !f.is_empty()).unwrap_or(false) {
                continue;
            }
            let rname = ruin_names[ruin_idx % ruin_names.len()].clone();
            ruin_idx += 1;
            cell_features.entry(pos).or_default().push("ruins".to_string());
            let mut data = JsonObject::new();
            data.insert("name".into(), JsonValue::from(rname));
            hex_data.entry(pos).or_default().insert("ruins".into(), JsonValue::Object(data));
            // HR-786: ~40% of ruins hide a searchable container (notice/int check).
            if rng.gen::<f64>() < 0.4 {
                let src = make_container(rng, pos, "weathered crate", 8);
                loot_source::push_source(hex_data.entry(pos).or_default(), src);
            }
            placed += 1;
        }

        // Table engine for landmark/lair rolls.
        let mut table_engine = TableEngine::new(self.db, SmallRng::seed_from_u64(rng.gen()));
        let _ = table_engine.load_tables(&default_content_dir());

        // --- Landmarks ---
        let num_landmarks = rng.gen_range(3..=6);
        let mut placed_landmarks = 0;
        let mut landmark_candidates = passable_hexes(cell_features, &[]);
        landmark_candidates.shuffle(rng);
        for pos in landmark_candidates {
            if placed_landmarks >= num_landmarks {
                break;
            }
            if cell_features.get(&pos).map(|f| !f.is_empty()).unwrap_or(false) {
                continue;
            }
            let landmark_data = match table_engine.roll_on("landmarks", None) {
                Ok(res) => res.result,
                Err(_) => {
                    let lname = landmark_names[placed_landmarks as usize % landmark_names.len()].clone();
                    let mut d = JsonObject::new();
                    d.insert("name".into(), JsonValue::from(lname));
                    d.insert("type".into(), JsonValue::from("generic"));
                    d
                }
            };
            cell_features.entry(pos).or_default().push("landmark".to_string());
            hex_data.entry(pos).or_default().insert("landmark".into(), JsonValue::Object(landmark_data));
            placed_landmarks += 1;
        }

        // --- Lairs ---
        let num_lairs = rng.gen_range(2..=4);
        let mut placed_lairs = 0;
        let mut lair_candidates = passable_hexes(cell_features, &["forest", "hills", "swamp", "mountains"]);
        lair_candidates.shuffle(rng);
        for pos in lair_candidates {
            if placed_lairs >= num_lairs {
                break;
            }
            if cell_features.get(&pos).map(|f| !f.is_empty()).unwrap_or(false) {
                continue;
            }
            let lair_data = match table_engine.roll_on("lairs", None) {
                Ok(res) => res.result,
                Err(_) => {
                    let mut d = JsonObject::new();
                    d.insert("name".into(), JsonValue::from("Monster Lair"));
                    d.insert("type".into(), JsonValue::from("generic"));
                    d
                }
            };
            cell_features.entry(pos).or_default().push("lair".to_string());
            hex_data.entry(pos).or_default().insert("lair".into(), JsonValue::Object(lair_data));
            // HR-786: ~35% of lairs hold a searchable supply cache (survive/wis).
            if rng.gen::<f64>() < 0.35 {
                let src = make_container(rng, pos, "supply cache", 6);
                loot_source::push_source(hex_data.entry(pos).or_default(), src);
            }
            placed_lairs += 1;
        }

        // --- Camps ---
        let num_camps = rng.gen_range(0..=2);
        let mut placed_camps = 0;
        let mut camp_candidates = passable_hexes(cell_features, &["plains", "wasteland", "hills"]);
        camp_candidates.shuffle(rng);
        for pos in camp_candidates {
            if placed_camps >= num_camps {
                break;
            }
            if cell_features.get(&pos).map(|f| !f.is_empty()).unwrap_or(false) {
                continue;
            }
            cell_features.entry(pos).or_default().push("camp".to_string());
            let mut d = JsonObject::new();
            d.insert("type".into(), JsonValue::from("bandit_camp"));
            hex_data.entry(pos).or_default().insert("camp".into(), JsonValue::Object(d));
            placed_camps += 1;
        }
    }
}
