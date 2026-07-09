//! Battle-grid position assignment for the encounter window (HR-755).
//!
//! Combat in Harsh Realm is mechanically band-driven (`melee` / `near` /
//! `ranged`). This module maps those bands onto a fixed 9×9 square grid so the
//! frontend can render a top-down encounter view. No tactical-movement logic
//! lives here; positions *mirror* the range band and are recomputed whenever
//! the band changes.
//!
//! All values are named constants (no magic numbers).

use crate::combat_runtime::CombatState;
use crate::payloads::notices_combat::{
    CombatPositionsNotice, PositionsCell, PositionsEntity, PositionsLoot,
};

// ---------------------------------------------------------------------------
// Grid constants
// ---------------------------------------------------------------------------

/// Battle grid width in cells.
pub const GRID_W: i32 = 9;
/// Battle grid height in cells.
pub const GRID_H: i32 = 9;

/// Fixed (q, r) centre point for the enemy cluster (near the top of the grid).
pub const ENEMY_CENTER: (i32, i32) = (4, 2);

/// Chebyshev distance from enemy centre for band `"melee"`.
pub const BAND_DISTANCE_MELEE: i32 = 1;
/// Chebyshev distance from enemy centre for band `"near"`.
pub const BAND_DISTANCE_NEAR: i32 = 3;
/// Chebyshev distance from enemy centre for band `"ranged"`.
pub const BAND_DISTANCE_RANGED: i32 = 5;

/// Maximum number of world-cell features stamped onto corner cells.
pub const MAX_FEATURE_STAMPS: usize = 4;

/// Fixed corner cells that receive world-feature stamps (one per feature, in
/// order). Index 0 → top-left, 1 → top-right, 2 → bottom-left, 3 → bottom-right.
pub const CORNER_CELLS: [(i32, i32); 4] = [(0, 0), (8, 0), (0, 8), (8, 8)];

/// Deterministic spiral of (dq, dr) offsets around `ENEMY_CENTER`. Alive enemies
/// are assigned positions by working through this list in roster order, skipping
/// occupied or out-of-bounds cells. The list is long enough to place 13 enemies.
pub const ENEMY_OFFSETS: [(i32, i32); 13] = [
    (0, 0),
    (-1, 0),
    (1, 0),
    (0, -1),
    (-1, -1),
    (1, -1),
    (-2, 0),
    (2, 0),
    (0, -2),
    (-2, -1),
    (2, -1),
    (-1, -2),
    (1, -2),
];

// ---------------------------------------------------------------------------
// Band → distance
// ---------------------------------------------------------------------------

/// Map a range-band string to its integer Chebyshev distance from
/// `ENEMY_CENTER`. Falls back to `BAND_DISTANCE_NEAR` for unknown bands.
pub fn band_distance(band: &str) -> i32 {
    match band {
        "melee" => BAND_DISTANCE_MELEE,
        "near" => BAND_DISTANCE_NEAR,
        "ranged" => BAND_DISTANCE_RANGED,
        _ => BAND_DISTANCE_NEAR,
    }
}

// ---------------------------------------------------------------------------
// Position assignment
// ---------------------------------------------------------------------------

/// Assign `(q, r)` grid positions to every combatant in `state`.
///
/// Rules (from the design doc §"Battle grid & position algorithm"):
/// - **PC:** `(ENEMY_CENTER.0, ENEMY_CENTER.1 + band_distance(player.range_band))`
///   — the player token slides toward/away from the enemy cluster as the band
///   changes (`advance` → melee → closest; `withdraw` → near → farther).
/// - **Alive enemies:** laid out around `ENEMY_CENTER` via `ENEMY_OFFSETS` in
///   roster order, skipping occupied or out-of-bounds cells.
/// - Dead enemies retain whatever coordinates they had (they are omitted from
///   `positions_notice` anyway).
///
/// This is a pure, deterministic function of the current combatant list; no RNG
/// is involved. Call it at combat start and after any band change.
pub fn assign_positions(state: &mut CombatState) {
    // -- PC ------------------------------------------------------------------
    let player_band = state
        .combatants
        .iter()
        .find(|c| c.is_player)
        .map(|c| c.range_band.clone())
        .unwrap_or_else(|| "melee".to_string());

    let pc_q = ENEMY_CENTER.0;
    let pc_r = ENEMY_CENTER.1 + band_distance(&player_band);

    for c in state.combatants.iter_mut() {
        if c.is_player {
            c.q = pc_q;
            c.r = pc_r;
        }
    }

    // -- Alive enemies -------------------------------------------------------
    let mut occupied = std::collections::HashSet::<(i32, i32)>::new();
    occupied.insert((pc_q, pc_r));

    // Defeated enemies keep their last position (they are shown as corpses), so
    // reserve those cells too — a live enemy must not be laid on top of a corpse
    // when positions are recomputed after a kill.
    for c in state.combatants.iter() {
        if !c.is_player && !c.alive {
            occupied.insert((c.q, c.r));
        }
    }

    let mut offset_idx: usize = 0;

    for c in state.combatants.iter_mut() {
        if c.is_player || !c.alive {
            continue;
        }
        let mut placed = false;
        while offset_idx < ENEMY_OFFSETS.len() {
            let (dq, dr) = ENEMY_OFFSETS[offset_idx];
            offset_idx += 1;
            let q = ENEMY_CENTER.0 + dq;
            let r = ENEMY_CENTER.1 + dr;
            if (0..GRID_W).contains(&q) && (0..GRID_H).contains(&r) && !occupied.contains(&(q, r)) {
                c.q = q;
                c.r = r;
                occupied.insert((q, r));
                placed = true;
                break;
            }
        }
        if !placed {
            // Overflow: leave at (0, 0) as a safe default.
            c.q = 0;
            c.r = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Positions notice builder
// ---------------------------------------------------------------------------

/// Build a `CombatPositionsNotice` snapshot from `state`.
///
/// - All 81 cells are emitted in row-major order (`r` outer, `q` inner).
/// - Every cell's `terrain` is `state.terrain`.
/// - The i-th world-cell feature (up to `MAX_FEATURE_STAMPS`) is stamped onto
///   the i-th corner cell (its `features = [feature_id]`); all other cells have
///   `features = []`.
/// - `entities` contains the PC and every enemy; defeated enemies are kept
///   (`alive: false`) so the frontend can show them as dimmed corpses.
pub fn positions_notice(state: &CombatState) -> CombatPositionsNotice {
    // Build the 81-cell grid (row-major: r outer, q inner).
    let mut cells = Vec::with_capacity((GRID_W * GRID_H) as usize);
    for r in 0..GRID_H {
        for q in 0..GRID_W {
            // Check whether this cell is a feature-stamped corner.
            let features = CORNER_CELLS
                .iter()
                .enumerate()
                .find(|(_, &(cq, cr))| q == cq && r == cr)
                .and_then(|(i, _)| {
                    if i < MAX_FEATURE_STAMPS {
                        state.features.get(i).map(|f| vec![f.clone()])
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            cells.push(PositionsCell {
                q,
                r,
                terrain: state.terrain.clone(),
                features,
            });
        }
    }

    // Build the entity list: PC + every enemy (defeated enemies are kept so the
    // grid shows them as dimmed corpses; the frontend renders `alive: false`
    // tokens at reduced opacity). Keeping them here means a re-emit on
    // advance/withdraw stays consistent with the `combat.enemy_defeated` path.
    let entities: Vec<PositionsEntity> = state
        .combatants
        .iter()
        .map(|c| PositionsEntity {
            entity_id: c.entity_id.clone(),
            kind: if c.is_player {
                "pc".to_string()
            } else {
                "monster".to_string()
            },
            name: c.display_name.clone(),
            q: c.q,
            r: c.r,
            hp: c.hp,
            max_hp: c.max_hp,
            alive: c.alive,
        })
        .collect();

    // HR-787: dropped loot piles, one per defeated enemy that dropped anything.
    // `value` is the pile's representative worth (highest single item value or
    // currency) so the frontend can pick a rarity tier.
    let loot: Vec<PositionsLoot> = state
        .ground_loot
        .iter()
        .filter_map(|g| {
            let mut best = if g.currency > 0 { g.currency } else { 0 };
            for item in &g.items {
                if let Some(v) = item.get("value").and_then(|v| v.as_i64()) {
                    best = best.max(v as i32);
                }
            }
            let item_count = g.items.len() as i32 + i32::from(g.currency > 0);
            if item_count == 0 {
                return None; // empty roll — tracked in state, but nothing to show
            }
            Some(PositionsLoot {
                q: g.q,
                r: g.r,
                value: best,
                item_count,
            })
        })
        .collect();

    CombatPositionsNotice {
        width: GRID_W,
        height: GRID_H,
        grid_type: "square".to_string(),
        cells,
        entities,
        loot,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_runtime::{Combatant, CombatState};

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn base_combatant(id: &str, is_player: bool, alive: bool, band: &str) -> Combatant {
        serde_json::from_value(serde_json::json!({
            "entity_id": id,
            "name": id,
            "display_name": id,
            "is_player": is_player,
            "initiative": 0,
            "hp": if alive { 5 } else { 0 },
            "max_hp": 5,
            "ac": 10,
            "attack_bonus": 0,
            "damage_expr": "1d4",
            "attack_description": "hits",
            "behavior": "melee",
            "num_attacks": 1,
            "alive": alive,
            "range_band": band,
        }))
        .expect("combatant json")
    }

    fn make_state_with(combatants: Vec<Combatant>) -> CombatState {
        let order: Vec<String> = combatants.iter().map(|c| c.entity_id.clone()).collect();
        serde_json::from_value(serde_json::json!({
            "combatants": serde_json::to_value(&combatants).unwrap(),
            "initiative_order": order,
        }))
        .expect("combat state json")
    }

    // ------------------------------------------------------------------
    // assign_positions — deterministic layout
    // ------------------------------------------------------------------

    #[test]
    fn assign_positions_pc_melee_at_4_3() {
        let pc = base_combatant("pc", true, true, "melee");
        let e0 = base_combatant("e0", false, true, "melee");
        let e1 = base_combatant("e1", false, true, "melee");
        let e2 = base_combatant("e2", false, true, "melee");
        let mut state = make_state_with(vec![pc, e0, e1, e2]);

        assign_positions(&mut state);

        // PC lands at (4, ENEMY_CENTER.1 + BAND_DISTANCE_MELEE) = (4, 3).
        let pc = state.combatants.iter().find(|c| c.is_player).unwrap();
        assert_eq!((pc.q, pc.r), (4, 3), "PC at (4,3) for melee band");

        // First enemy takes offset (0,0) → ENEMY_CENTER = (4, 2).
        let e0 = state.combatants.iter().find(|c| c.entity_id == "e0").unwrap();
        assert_eq!((e0.q, e0.r), (4, 2));

        // Second enemy: offset (-1,0) → (3, 2).
        let e1 = state.combatants.iter().find(|c| c.entity_id == "e1").unwrap();
        assert_eq!((e1.q, e1.r), (3, 2));

        // Third enemy: offset (1,0) → (5, 2).
        let e2 = state.combatants.iter().find(|c| c.entity_id == "e2").unwrap();
        assert_eq!((e2.q, e2.r), (5, 2));
    }

    // ------------------------------------------------------------------
    // Band → distance + emission: switching band moves the PC token and the
    // positions_notice reflects the new position (simulates advance handler).
    // ------------------------------------------------------------------

    #[test]
    fn band_switch_melee_to_near_and_back() {
        let pc = base_combatant("pc", true, true, "melee");
        let e0 = base_combatant("e0", false, true, "melee");
        let mut state = make_state_with(vec![pc, e0]);

        // Initial: melee → PC at (4, 3); enemy at ENEMY_CENTER (4, 2).
        assign_positions(&mut state);
        let notice = positions_notice(&state);
        let pc_ent = notice.entities.iter().find(|e| e.kind == "pc").unwrap();
        assert_eq!((pc_ent.q, pc_ent.r), (4, 3), "melee places PC at (4,3)");

        // Chebyshev distance from enemy centre equals BAND_DISTANCE_MELEE.
        let dist = (pc_ent.q - ENEMY_CENTER.0).abs().max((pc_ent.r - ENEMY_CENTER.1).abs());
        assert_eq!(dist, BAND_DISTANCE_MELEE);

        // Simulate withdraw → near band → PC moves to (4, 5).
        for c in state.combatants.iter_mut() {
            if c.is_player {
                c.range_band = "near".to_string();
            }
        }
        assign_positions(&mut state);
        let notice = positions_notice(&state);
        let pc_ent = notice.entities.iter().find(|e| e.kind == "pc").unwrap();
        assert_eq!((pc_ent.q, pc_ent.r), (4, 5), "near band places PC at (4,5)");

        let dist = (pc_ent.q - ENEMY_CENTER.0).abs().max((pc_ent.r - ENEMY_CENTER.1).abs());
        assert_eq!(dist, BAND_DISTANCE_NEAR);
    }

    // ------------------------------------------------------------------
    // positions_notice — 81 cells, terrain, feature stamps, entity kinds,
    // defeated enemy kept as a corpse.
    // ------------------------------------------------------------------

    #[test]
    fn positions_notice_full() {
        let pc = base_combatant("player_1", true, true, "melee");
        let alive_enemy = base_combatant("goblin_0", false, true, "melee");
        let dead_enemy = base_combatant("e_dead", false, false, "melee");
        let mut state = make_state_with(vec![pc, alive_enemy, dead_enemy]);
        state.terrain = "ruins".to_string();
        state.features = vec!["rubble".to_string(), "pillar".to_string()];
        // Mark the dead enemy explicitly (base_combatant already sets hp=0/alive=false
        // for alive=false, but be explicit).
        for c in state.combatants.iter_mut() {
            if c.entity_id == "e_dead" {
                c.hp = 0;
                c.alive = false;
            }
        }
        assign_positions(&mut state);

        let notice = positions_notice(&state);

        // 81 cells in row-major order.
        assert_eq!(notice.cells.len(), 81, "must emit exactly 81 cells");
        assert_eq!(notice.width, 9);
        assert_eq!(notice.height, 9);
        assert_eq!(notice.grid_type, "square");
        let expected_coords: Vec<(i32, i32)> =
            (0..9_i32).flat_map(|r| (0..9_i32).map(move |q| (q, r))).collect();
        let got_coords: Vec<(i32, i32)> =
            notice.cells.iter().map(|c| (c.q, c.r)).collect();
        assert_eq!(got_coords, expected_coords, "cells must be row-major");

        // Every cell carries the encounter terrain.
        assert!(notice.cells.iter().all(|c| c.terrain == "ruins"));

        // Corner stamps: (0,0) → "rubble", (8,0) → "pillar", (0,8) → no feature.
        let c00 = notice.cells.iter().find(|c| c.q == 0 && c.r == 0).unwrap();
        assert_eq!(c00.features, vec!["rubble"]);
        let c80 = notice.cells.iter().find(|c| c.q == 8 && c.r == 0).unwrap();
        assert_eq!(c80.features, vec!["pillar"]);
        let c08 = notice.cells.iter().find(|c| c.q == 0 && c.r == 8).unwrap();
        assert!(c08.features.is_empty());

        // Entity kinds: PC → "pc", alive monster → "monster".
        let pc_ent = notice.entities.iter().find(|e| e.entity_id == "player_1").unwrap();
        assert_eq!(pc_ent.kind, "pc");
        let mon_ent = notice.entities.iter().find(|e| e.entity_id == "goblin_0").unwrap();
        assert_eq!(mon_ent.kind, "monster");

        // Defeated enemy is kept as a corpse (alive: false) so the grid stays
        // consistent across re-emits: PC + 1 alive + 1 dead = 3 entities.
        assert_eq!(notice.entities.len(), 3, "dead enemy must be kept as a corpse");
        let dead_ent = notice
            .entities
            .iter()
            .find(|e| e.entity_id == "e_dead")
            .expect("dead enemy must be present");
        assert_eq!(dead_ent.kind, "monster");
        assert!(!dead_ent.alive, "dead enemy must be marked not alive");
    }

    #[test]
    fn assign_positions_keeps_live_enemy_off_a_corpse_cell() {
        // A corpse occupies its last cell; a re-lay after the kill must not put a
        // surviving enemy on top of it.
        let pc = base_combatant("player_1", true, true, "melee");
        let e_dead = base_combatant("e_dead", false, false, "melee");
        let e_live = base_combatant("e_live", false, true, "melee");
        let mut state = make_state_with(vec![pc, e_dead, e_live]);
        // Pin the corpse onto the first enemy slot (ENEMY_CENTER).
        for c in state.combatants.iter_mut() {
            if c.entity_id == "e_dead" {
                c.q = ENEMY_CENTER.0;
                c.r = ENEMY_CENTER.1;
            }
        }
        assign_positions(&mut state);
        let live = state
            .combatants
            .iter()
            .find(|c| c.entity_id == "e_live")
            .unwrap();
        assert_ne!(
            (live.q, live.r),
            (ENEMY_CENTER.0, ENEMY_CENTER.1),
            "live enemy must not be placed on the corpse cell"
        );
    }

    #[test]
    fn positions_notice_maps_ground_loot_to_tiered_piles() {
        // HR-787: dropped piles become PositionsLoot with a representative value
        // (top item value or currency); empty piles are excluded.
        use crate::combat_runtime::GroundLoot;
        use crate::runtime::JsonObject;

        let item = |name: &str, value: i64| -> JsonObject {
            serde_json::from_value(
                serde_json::json!({ "name": name, "type": "misc", "value": value }),
            )
            .unwrap()
        };
        let mut state = make_state_with(vec![
            base_combatant("player_1", true, true, "melee"),
            base_combatant("goblin_0", false, false, "melee"),
        ]);
        state.ground_loot = vec![
            GroundLoot {
                entity_id: "goblin_0".into(),
                q: 2,
                r: 3,
                items: vec![item("Dagger", 40), item("Data Chip", 150)],
                currency: 0,
            },
            GroundLoot {
                entity_id: "rat_1".into(),
                q: 4,
                r: 5,
                items: vec![],
                currency: 12,
            },
            GroundLoot {
                entity_id: "void_2".into(),
                q: 0,
                r: 0,
                items: vec![],
                currency: 0,
            },
        ];

        let notice = positions_notice(&state);

        // Empty pile excluded; two visible piles.
        assert_eq!(notice.loot.len(), 2);
        let gem = notice.loot.iter().find(|l| l.q == 2 && l.r == 3).unwrap();
        assert_eq!(gem.value, 150, "representative value is the top item value");
        assert_eq!(gem.item_count, 2);
        let coins = notice.loot.iter().find(|l| l.q == 4 && l.r == 5).unwrap();
        assert_eq!(coins.value, 12, "currency counts as value");
        assert_eq!(coins.item_count, 1);
    }
}
