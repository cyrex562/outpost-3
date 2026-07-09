//! World generator: settlement enhancement.
//!
//! Ported from `src/harsh_realm/generators/_world_settlements_mixin.py`. Runs a
//! full [`SettlementGenerator`] pass over every cell tagged `"settlement"`.

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::generators::npc_gen::NPCGenerator;
use crate::generators::settlement_gen::SettlementGenerator;
use crate::repositories::cell::CellRepository;
use crate::table_engine::TableEngine;

use super::{CellFeatures, TerrainGrid, WorldGenerator};

impl WorldGenerator<'_> {
    /// Generate detailed settlement data for every cell with the `settlement` feature.
    ///
    /// Per-settlement errors are skipped so they don't abort world generation.
    pub(super) fn enhance_settlements(
        &self,
        terrain_grid: &TerrainGrid,
        cell_features: &CellFeatures,
        rng: &mut SmallRng,
    ) {
        let Some(data_dir) = self.data_dir.as_ref() else {
            return;
        };

        let mut settlement_engine = TableEngine::new(self.db, SmallRng::seed_from_u64(rng.gen()));
        if settlement_engine.load_tables(data_dir).is_err() {
            return;
        }
        let mut npc_engine = TableEngine::new(self.db, SmallRng::seed_from_u64(rng.gen()));
        let _ = npc_engine.load_tables(data_dir);
        let npc_gen = NPCGenerator::new(npc_engine);
        let mut settlement_gen =
            SettlementGenerator::new(settlement_engine, npc_gen, SmallRng::seed_from_u64(rng.gen()));

        let settlement_cells: Vec<(i32, i32)> = cell_features
            .iter()
            .filter(|(_, feats)| feats.iter().any(|f| f == "settlement"))
            .map(|(&pos, _)| pos)
            .collect();

        let cell_repo = CellRepository::new(self.db);
        for (q, r) in settlement_cells {
            let terrain = terrain_grid.get(&(q, r)).cloned().unwrap_or_else(|| "plains".to_string());
            let size = cell_repo
                .load_cell_data(q as i64, r as i64)
                .ok()
                .flatten()
                .and_then(|data| {
                    data.get("settlement")
                        .and_then(|s| s.get("size"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| "village".to_string());
            // Errors are intentionally ignored per-settlement.
            let _ = settlement_gen.generate_settlement((q, r), &size, &terrain, self.db);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::TerrainRegistry;
    use crate::db::WorldDatabase;
    use std::collections::HashMap;

    #[test]
    fn enhance_without_data_dir_is_noop() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let plains = serde_json::from_value::<crate::cell::TerrainType>(serde_json::json!({
            "id": "plains", "name": "Plains", "passable": true, "blocks_vision": false
        }))
        .unwrap();
        let registry = TerrainRegistry::from_types(vec![plains]);
        let gen = WorldGenerator::new(&db, registry, None);
        let mut rng = SmallRng::seed_from_u64(1);
        // No data_dir -> returns immediately, no panic, no settlement entities.
        gen.enhance_settlements(&HashMap::new(), &HashMap::new(), &mut rng);
        let count = db
            .fetch_one("SELECT COUNT(*) AS n FROM entities WHERE entity_type = 'settlement'", &[])
            .unwrap()
            .unwrap();
        assert_eq!(count.get("n").unwrap().as_i64(), Some(0));
    }
}
