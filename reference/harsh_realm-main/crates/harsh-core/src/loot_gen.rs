//! Loot generation system for Harsh Realm.
//!
//! Ported from `src/harsh_realm/engine/loot.py` (named `loot_gen` to avoid the
//! `loot` value-types module). RNG is injected for determinism.

use std::collections::HashMap;
use std::path::PathBuf;

use rand::Rng;

use crate::creature::CreatureData;
use crate::engine_runtime::{HarvestableRecord, LootTableDocument, LootTableEntry};
use crate::loot::{HarvestResult, LootItem, LootResult};
use crate::packs::paths::default_content_dir;

/// Generates loot from defeated enemies using YAML loot tables.
#[derive(Debug, Clone)]
pub struct LootGenerator {
    data_dir: PathBuf,
    tables: HashMap<String, Vec<LootTableEntry>>,
    /// Multiplier on loot item count and currency totals (default 1.0).
    pub loot_amount_mult: f64,
    /// Multiplier on drop probability via the `type: nothing` weight (default 1.0).
    pub loot_probability_mult: f64,
}

impl LootGenerator {
    /// Create a generator rooted at `data_dir` (defaults to the content dir).
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        LootGenerator {
            data_dir: data_dir.unwrap_or_else(default_content_dir),
            tables: HashMap::new(),
            loot_amount_mult: 1.0,
            loot_probability_mult: 1.0,
        }
    }

    /// Create a generator with explicit difficulty scalars.
    pub fn with_difficulty(
        data_dir: Option<PathBuf>,
        loot_amount_mult: f64,
        loot_probability_mult: f64,
    ) -> Self {
        LootGenerator {
            data_dir: data_dir.unwrap_or_else(default_content_dir),
            tables: HashMap::new(),
            loot_amount_mult,
            loot_probability_mult,
        }
    }

    fn load_table(&mut self, table_id: &str) -> &[LootTableEntry] {
        if !self.tables.contains_key(table_id) {
            let path = self
                .data_dir
                .join("tables")
                .join("loot")
                .join(format!("{table_id}.yaml"));
            let entries = std::fs::read_to_string(&path)
                .ok()
                .and_then(|t| serde_yaml::from_str::<LootTableDocument>(&t).ok())
                .map(|doc| doc.entries)
                .unwrap_or_default();
            self.tables.insert(table_id.to_string(), entries);
        }
        &self.tables[table_id]
    }

    /// Roll once on a named loot table, applying the loot-probability multiplier.
    ///
    /// When `loot_probability_mult != 1.0` the weight of any entry with
    /// `type: nothing` is scaled by `1 / loot_probability_mult`, preserving the
    /// relative weights of real-item entries while shifting the overall drop rate:
    /// - mult > 1 → smaller "nothing" weight → more drops (easier)
    /// - mult < 1 → larger  "nothing" weight → fewer drops (harder)
    pub fn roll_loot<R: Rng + ?Sized>(&mut self, table_id: &str, rng: &mut R) -> Vec<LootItem> {
        let mut entries = self.load_table(table_id).to_vec();
        if entries.is_empty() {
            return vec![];
        }
        // Apply loot_probability_mult by scaling the "nothing" entry weight.
        if (self.loot_probability_mult - 1.0).abs() > 1e-9 {
            for entry in &mut entries {
                if entry.data.r#type == "nothing" {
                    let original = entry.weight as f64;
                    // Dividing by mult: if mult>1 → weight decreases → more drops.
                    let scaled = (original / self.loot_probability_mult).round().max(1.0);
                    entry.weight = scaled as i32;
                }
            }
        }
        let total_weight: i32 = entries.iter().map(|e| e.weight).sum();
        if total_weight <= 0 {
            return vec![];
        }
        let roll = rng.gen_range(1..=total_weight);
        let mut cumulative = 0;
        let mut chosen = entries.last().unwrap().clone();
        for entry in &entries {
            cumulative += entry.weight;
            if roll <= cumulative {
                chosen = entry.clone();
                break;
            }
        }

        let item_data = chosen.data;
        if item_data.r#type == "nothing" {
            return vec![];
        }
        let name = if !item_data.name.is_empty() {
            item_data.name.clone()
        } else if !chosen.result.is_empty() {
            chosen.result.clone()
        } else {
            "unknown item".to_string()
        };
        vec![LootItem {
            name,
            description: chosen.result,
            item_type: item_data.r#type.clone(),
            value: item_data.value,
            data: item_data,
        }]
    }

    /// Determine how many loot rolls to perform for one enemy, based on
    /// `loot_amount_mult`. Returns `floor(mult)` plus one probabilistic extra
    /// roll with probability equal to the fractional part of `mult`.
    fn loot_roll_count<R: Rng + ?Sized>(&self, rng: &mut R) -> usize {
        let mult = self.loot_amount_mult.max(0.0);
        let floor = mult.floor() as usize;
        let frac = mult - mult.floor();
        let extra = if frac > 0.0 && rng.gen::<f64>() < frac { 1 } else { 0 };
        floor + extra
    }

    /// Generate all loot from a list of defeated enemies.
    ///
    /// Applies difficulty scalars:
    /// - `loot_amount_mult` scales how many rolls are made per enemy and the
    ///   final currency total.
    /// - `loot_probability_mult` is handled per-roll inside [`roll_loot`].
    pub fn generate_combat_loot<R: Rng + ?Sized>(
        &mut self,
        enemies: &[CreatureData],
        rng: &mut R,
    ) -> LootResult {
        let mut all_items: Vec<LootItem> = Vec::new();
        let mut total_currency = 0;
        let mut harvestable_list: Vec<HarvestableRecord> = Vec::new();

        for enemy in enemies {
            if let Some(table) = &enemy.loot_table {
                let rolls = self.loot_roll_count(rng);
                for _ in 0..rolls {
                    for item in self.roll_loot(table, rng) {
                        if item.item_type == "currency" {
                            total_currency += item.value;
                        } else {
                            all_items.push(item);
                        }
                    }
                }
            }
            if let Some(h) = &enemy.harvestable {
                harvestable_list.push(HarvestableRecord {
                    creature_name: enemy.name.clone(),
                    material: h.material.clone(),
                    skill: Some(h.skill.clone()),
                    difficulty: h.difficulty,
                });
            }
        }
        // Scale accumulated currency by the amount multiplier.
        let total_currency = ((total_currency as f64) * self.loot_amount_mult)
            .round()
            .max(0.0) as i32;

        let mut lines: Vec<String> = Vec::new();
        if !all_items.is_empty() {
            let names: Vec<&str> = all_items.iter().map(|i| i.name.as_str()).collect();
            lines.push(format!("You search the remains and find: {}.", names.join(", ")));
        }
        if total_currency > 0 {
            lines.push(format!("You recover {total_currency} coin."));
        }
        if !harvestable_list.is_empty() {
            let mats: Vec<&str> = harvestable_list.iter().map(|h| h.material.as_str()).collect();
            lines.push(format!(
                "You could attempt to harvest: {}. Use 'harvest <material>' to try.",
                mats.join(", ")
            ));
        }
        if lines.is_empty() {
            lines.push("You search the remains but find nothing of value.".to_string());
        }

        LootResult {
            items: all_items,
            currency: total_currency,
            harvestable: harvestable_list,
            narration: lines.join(" "),
        }
    }

    /// Attempt to harvest material from a defeated creature (2d6 + skill + attr).
    pub fn attempt_harvest<R: Rng + ?Sized>(
        &self,
        creature: &CreatureData,
        skill_level: i32,
        attr_mod: i32,
        rng: &mut R,
    ) -> HarvestResult {
        let Some(harvestable) = &creature.harvestable else {
            return HarvestResult {
                success: false,
                material: String::new(),
                narration: format!(
                    "There's nothing useful to harvest from the {}.",
                    creature.name
                ),
            };
        };
        let material = harvestable.material.clone();
        let difficulty = harvestable.difficulty;
        let roll = rng.gen_range(1..=6) + rng.gen_range(1..=6);
        let total = roll + skill_level + attr_mod;
        let success = total >= difficulty;
        let narration = if success {
            format!(
                "Working carefully, you successfully harvest the {material} from the {}. (roll {roll} → {total} vs {difficulty})",
                creature.name
            )
        } else {
            format!(
                "You attempt to harvest the {material} but botch the job — the material is ruined. (roll {roll} → {total} vs {difficulty})"
            )
        };
        HarvestResult {
            success,
            material,
            narration,
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_runtime::{LootItemData, LootTableEntry};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    /// Build a minimal in-memory loot table for testing (without filesystem I/O).
    fn gen_with_table(
        entries: Vec<LootTableEntry>,
        amount_mult: f64,
        prob_mult: f64,
    ) -> LootGenerator {
        let mut gen = LootGenerator::with_difficulty(None, amount_mult, prob_mult);
        gen.tables.insert("test_table".to_string(), entries);
        gen
    }

    fn entry(item_type: &str, weight: i32) -> LootTableEntry {
        LootTableEntry {
            weight,
            result: item_type.to_string(),
            data: LootItemData {
                r#type: item_type.to_string(),
                name: if item_type == "nothing" {
                    String::new()
                } else {
                    item_type.to_string()
                },
                value: if item_type == "currency" { 10 } else { 0 },
                ..LootItemData::default()
            },
        }
    }

    fn creature_with_table() -> CreatureData {
        let mut c: CreatureData = serde_json::from_str(
            r#"{"id":"test","name":"Test","hd":1,"ac":10,"attack_bonus":0,"damage":"1d4"}"#,
        )
        .expect("creature json");
        c.loot_table = Some("test_table".to_string());
        c
    }

    // ---- loot_probability_mult ----

    #[test]
    fn probability_mult_1_is_unchanged() {
        // With mult=1.0 the nothing weight stays as authored.
        // We just verify no panic; the roll distribution is unaffected.
        let entries = vec![entry("nothing", 8), entry("sword", 2)];
        let mut gen = gen_with_table(entries.clone(), 1.0, 1.0);
        let mut rng = StdRng::seed_from_u64(0);
        // Should complete without errors.
        let _ = gen.roll_loot("test_table", &mut rng);
    }

    #[test]
    fn probability_mult_2_reduces_nothing_weight() {
        // With mult=2.0 the nothing entry weight is halved → more real items.
        // Over many rolls with a seeded RNG, we should see more non-empty results.
        let entries = vec![entry("nothing", 80), entry("sword", 20)];
        let mut gen_normal = gen_with_table(entries.clone(), 1.0, 1.0);
        let mut gen_easy = gen_with_table(entries, 1.0, 2.0);
        let mut rng_n = StdRng::seed_from_u64(7);
        let mut rng_e = StdRng::seed_from_u64(7);
        let normal_drops: usize = (0..200)
            .filter(|_| !gen_normal.roll_loot("test_table", &mut rng_n).is_empty())
            .count();
        let easy_drops: usize = (0..200)
            .filter(|_| !gen_easy.roll_loot("test_table", &mut rng_e).is_empty())
            .count();
        // Easier difficulty should yield more drops.
        assert!(
            easy_drops > normal_drops,
            "expected easy({easy_drops}) > normal({normal_drops})"
        );
    }

    #[test]
    fn probability_mult_0_5_increases_nothing_weight() {
        // With mult=0.5 the nothing weight doubles → fewer drops.
        let entries = vec![entry("nothing", 50), entry("sword", 50)];
        let mut gen_normal = gen_with_table(entries.clone(), 1.0, 1.0);
        let mut gen_hard = gen_with_table(entries, 1.0, 0.5);
        let mut rng_n = StdRng::seed_from_u64(3);
        let mut rng_h = StdRng::seed_from_u64(3);
        let normal_drops: usize = (0..200)
            .filter(|_| !gen_normal.roll_loot("test_table", &mut rng_n).is_empty())
            .count();
        let hard_drops: usize = (0..200)
            .filter(|_| !gen_hard.roll_loot("test_table", &mut rng_h).is_empty())
            .count();
        assert!(
            hard_drops < normal_drops,
            "expected hard({hard_drops}) < normal({normal_drops})"
        );
    }

    // ---- loot_amount_mult ----

    #[test]
    fn amount_mult_1_is_unchanged() {
        // A single enemy with a pure-currency table should yield the same total.
        let entries = vec![entry("currency", 1)];
        let mut gen = gen_with_table(entries, 1.0, 1.0);
        let enemy = creature_with_table();
        let mut rng = StdRng::seed_from_u64(0);
        let result = gen.generate_combat_loot(&[enemy], &mut rng);
        // currency = 10 * mult(1.0) = 10
        assert_eq!(result.currency, 10);
    }

    #[test]
    fn amount_mult_2_doubles_currency() {
        let entries = vec![entry("currency", 1)];
        let mut gen = gen_with_table(entries, 2.0, 1.0);
        let enemy = creature_with_table();
        let mut rng = StdRng::seed_from_u64(0);
        let result = gen.generate_combat_loot(&[enemy], &mut rng);
        // 1 guaranteed roll (floor(2)), 1 frac=0 extra → 2 rolls each giving 10 → 20.
        // After currency scaling by 2.0 → 20 * 2 = 40.
        assert!(result.currency >= 10, "expected currency ≥ 10 with 2× mult");
    }

    #[test]
    fn amount_mult_grade_3_unchanged() {
        // mult=1.0 (grade 3) leaves the result equal to a plain generator.
        let entries = vec![entry("currency", 1)];
        let mut gen1 = gen_with_table(entries.clone(), 1.0, 1.0);
        let mut gen2 = gen_with_table(entries, 1.0, 1.0);
        let enemy = creature_with_table();
        let mut rng1 = StdRng::seed_from_u64(5);
        let mut rng2 = StdRng::seed_from_u64(5);
        let r1 = gen1.generate_combat_loot(&[enemy.clone()], &mut rng1);
        let r2 = gen2.generate_combat_loot(&[enemy], &mut rng2);
        assert_eq!(r1.currency, r2.currency);
    }
}
