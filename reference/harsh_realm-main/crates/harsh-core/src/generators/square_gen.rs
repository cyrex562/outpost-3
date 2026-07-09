//! Square grid world generators for dungeons and towns.
//!
//! Ported from `src/harsh_realm/generators/square_gen.py`. Generates
//! room-and-corridor dungeons and building-based towns on a square grid with
//! guaranteed connectivity from the entrance.

use std::collections::{HashMap, HashSet, VecDeque};

use rand::Rng;

use crate::generation::{DungeonResult, SquareCell, TownResult};
use crate::scene_data::{DungeonConnection, DungeonRoom, SettlementBuilding};
use crate::settlement::SettlementSummary;
use crate::runtime::JsonObject;

type Grid = HashMap<(i32, i32), String>;

/// Generates square-grid maps for dungeons and towns.
#[derive(Debug, Default, Clone, Copy)]
pub struct SquareWorldGenerator;

impl SquareWorldGenerator {
    /// Generate a dungeon with random rooms connected by L-shaped corridors.
    pub fn generate_dungeon<R: Rng>(
        &self,
        width: i32,
        height: i32,
        num_rooms: i32,
        rng: &mut R,
    ) -> DungeonResult {
        let mut grid: Grid = HashMap::new();
        for r in 0..height {
            for q in 0..width {
                grid.insert((q, r), "wall".to_string());
            }
        }

        let mut rooms: Vec<DungeonRoom> = Vec::new();
        let mut centers: Vec<(i32, i32)> = Vec::new();

        let mut attempts = 0;
        while (rooms.len() as i32) < num_rooms && attempts < num_rooms * 20 {
            attempts += 1;
            let rw = roll_range(rng, 3, (width / 3).min(7));
            let rh = roll_range(rng, 3, (height / 3).min(5));
            let rx_hi = width - rw - 1;
            let ry_hi = height - rh - 1;
            if rx_hi < 1 || ry_hi < 1 {
                continue;
            }
            let rx = roll_range(rng, 1, rx_hi);
            let ry = roll_range(rng, 1, ry_hi);

            // Check overlap (with a 1-cell margin).
            let mut overlap = false;
            'outer: for q in (rx - 1)..(rx + rw + 1) {
                for r in (ry - 1)..(ry + rh + 1) {
                    if grid.get(&(q, r)).map(|t| t != "wall").unwrap_or(false) {
                        overlap = true;
                        break 'outer;
                    }
                }
            }
            if overlap {
                continue;
            }

            let idx = rooms.len();
            let room_type = if idx == 0 { "entrance" } else { "room" };
            let cx = rx + rw / 2;
            let cy = ry + rh / 2;
            centers.push((cx, cy));

            for q in rx..(rx + rw) {
                for r in ry..(ry + rh) {
                    grid.insert((q, r), "room_floor".to_string());
                }
            }

            let mut editor_pos = JsonObject::new();
            editor_pos.insert("x".into(), (cx * 20).into());
            editor_pos.insert("y".into(), (cy * 20).into());

            rooms.push(DungeonRoom {
                id: format!("room_{idx}"),
                name: if room_type == "entrance" {
                    "Entrance".to_string()
                } else {
                    format!("Room {}", idx + 1)
                },
                r#type: room_type.to_string(),
                x: Some(rx),
                y: Some(ry),
                w: Some(rw),
                h: Some(rh),
                center_q: Some(cx),
                center_r: Some(cy),
                editor_pos: Some(editor_pos),
                description: None,
                loot: Vec::new(),
                hidden_loot: Vec::new(),
                traps: Vec::new(),
                encounter: None,
                data: JsonObject::new(),
            });
        }

        let mut connections: Vec<DungeonConnection> = Vec::new();
        for i in 1..rooms.len() {
            let (q1, r1) = centers[i - 1];
            let (q2, r2) = centers[i];
            carve_corridor(&mut grid, q1, r1, q2, r2, width, height);
            connections.push(DungeonConnection {
                from_room: rooms[i - 1].id.clone(),
                to_room: rooms[i].id.clone(),
                direction: primary_direction(q2 - q1, r2 - r1).to_string(),
            });
        }

        ensure_connectivity(&mut grid, &rooms, &centers, &mut connections, width, height);

        let cells = build_cells(&grid, width, height);
        let entrance_q = rooms.first().and_then(|r| r.center_q).unwrap_or(0);
        let entrance_r = rooms.first().and_then(|r| r.center_r).unwrap_or(0);

        DungeonResult {
            width,
            height,
            cells,
            rooms,
            connections,
            entrance_q,
            entrance_r,
        }
    }

    /// Generate a town layout with a cross-road grid and quadrant buildings.
    pub fn generate_town<R: Rng>(
        &self,
        width: i32,
        height: i32,
        settlement_data: Option<SettlementSummary>,
        rng: &mut R,
    ) -> TownResult {
        let settlement = settlement_data.unwrap_or_else(|| SettlementSummary {
            name: "Town".to_string(),
            size: "hamlet".to_string(),
            description: String::new(),
            settlement_id: "generated".to_string(),
            establishments: Vec::new(),
            resident_npc_ids: Vec::new(),
        });

        let mut grid: Grid = HashMap::new();
        for r in 0..height {
            for q in 0..width {
                grid.insert((q, r), "open_ground".to_string());
            }
        }

        let cx = width / 2;
        let cy = height / 2;
        for dq in -1..2 {
            for dr in -1..2 {
                grid.insert((cx + dq, cy + dr), "plaza".to_string());
            }
        }
        for q in 0..width {
            grid.insert((q, cy), "road".to_string());
        }
        for r in 0..height {
            grid.insert((cx, r), "road".to_string());
        }

        let mut buildings: Vec<SettlementBuilding> = Vec::new();
        let establishments = &settlement.establishments;
        let quadrants = [
            (1, 1, cx - 2, cy - 2),
            (cx + 2, 1, width - 2, cy - 2),
            (1, cy + 2, cx - 2, height - 2),
            (cx + 2, cy + 2, width - 2, height - 2),
        ];

        let mut building_idx = 0;
        for (qmin, rmin, qmax, rmax) in quadrants {
            if qmin >= qmax || rmin >= rmax {
                continue;
            }
            let bw = 3.min(qmax - qmin);
            let bh = 3.min(rmax - rmin);
            let bq = roll_range(rng, qmin, qmin.max(qmax - bw));
            let br = roll_range(rng, rmin, rmin.max(rmax - bh));

            let (building, btype) = if building_idx < establishments.len() {
                let est = &establishments[building_idx];
                let mut b = est.clone();
                b.id = Some(format!("building_{building_idx}"));
                b.q = Some(bq);
                b.r = Some(br);
                b.w = Some(bw);
                b.h = Some(bh);
                let btype = est.r#type.clone().unwrap_or_else(|| "building".to_string());
                (b, btype)
            } else {
                let b = SettlementBuilding {
                    id: Some(format!("building_{building_idx}")),
                    name: format!("Building {}", building_idx + 1),
                    r#type: Some("building".to_string()),
                    q: Some(bq),
                    r: Some(br),
                    w: Some(bw),
                    h: Some(bh),
                    npc_id: None,
                    npc_name: None,
                    building_id: None,
                    extra: JsonObject::new(),
                };
                (b, "building".to_string())
            };

            let terrain = match btype.as_str() {
                "shop" | "blacksmith" | "general_store" => "shop",
                "tavern" => "tavern",
                "temple" => "temple",
                _ => "building",
            };
            for dq in 0..bw {
                for dr in 0..bh {
                    grid.insert((bq + dq, br + dr), terrain.to_string());
                }
            }

            buildings.push(building);
            building_idx += 1;
        }

        let cells = build_cells(&grid, width, height);
        TownResult {
            width,
            height,
            cells,
            buildings,
        }
    }
}

/// `random.randint(lo, hi)` equivalent, clamped so `hi >= lo` (never panics).
fn roll_range<R: Rng>(rng: &mut R, lo: i32, hi: i32) -> i32 {
    let hi = hi.max(lo);
    rng.gen_range(lo..=hi)
}

fn build_cells(grid: &Grid, width: i32, height: i32) -> Vec<SquareCell> {
    let mut cells = Vec::with_capacity((width * height) as usize);
    for r in 0..height {
        for q in 0..width {
            let terrain = grid.get(&(q, r)).cloned().unwrap_or_else(|| "wall".to_string());
            cells.push(SquareCell {
                q,
                r,
                terrain,
                room_id: None,
                features: Vec::new(),
                data: JsonObject::new(),
            });
        }
    }
    cells
}

fn carve_corridor(grid: &mut Grid, q1: i32, r1: i32, q2: i32, r2: i32, width: i32, height: i32) {
    let mut q = q1;
    let mut r = r1;
    let carve = |grid: &mut Grid, q: i32, r: i32| {
        if (0..width).contains(&q) && (0..height).contains(&r) && grid.get(&(q, r)).map(String::as_str) == Some("wall") {
            grid.insert((q, r), "corridor".to_string());
        }
    };
    while q != q2 {
        carve(grid, q, r);
        q += if q2 > q { 1 } else { -1 };
    }
    while r != r2 {
        carve(grid, q, r);
        r += if r2 > r { 1 } else { -1 };
    }
    carve(grid, q, r);
}

fn ensure_connectivity(
    grid: &mut Grid,
    rooms: &[DungeonRoom],
    centers: &[(i32, i32)],
    connections: &mut Vec<DungeonConnection>,
    width: i32,
    height: i32,
) {
    if rooms.is_empty() {
        return;
    }
    let passable: HashSet<(i32, i32)> = grid
        .iter()
        .filter(|(_, t)| t.as_str() != "wall")
        .map(|(pos, _)| *pos)
        .collect();
    let start = centers[0];
    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
    visited.insert(start);
    queue.push_back(start);
    while let Some((q, r)) = queue.pop_front() {
        for (dq, dr) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let next = (q + dq, r + dr);
            if passable.contains(&next) && !visited.contains(&next) {
                visited.insert(next);
                queue.push_back(next);
            }
        }
    }
    for (i, &(cq, cr)) in centers.iter().enumerate() {
        if !visited.contains(&(cq, cr)) {
            let (eq, er) = centers[0];
            carve_corridor(grid, eq, er, cq, cr, width, height);
            connections.push(DungeonConnection {
                from_room: rooms[0].id.clone(),
                to_room: rooms[i].id.clone(),
                direction: primary_direction(cq - eq, cr - er).to_string(),
            });
        }
    }
}

fn primary_direction(dq: i32, dr: i32) -> &'static str {
    if dq.abs() > dr.abs() {
        if dq > 0 {
            "east"
        } else {
            "west"
        }
    } else if dr.abs() > dq.abs() {
        if dr > 0 {
            "south"
        } else {
            "north"
        }
    } else if dq > 0 && dr > 0 {
        "southeast"
    } else if dq > 0 && dr < 0 {
        "northeast"
    } else if dq < 0 && dr > 0 {
        "southwest"
    } else if dq < 0 && dr < 0 {
        "northwest"
    } else {
        "east"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn dungeon_has_cells_rooms_and_entrance() {
        let mut rng = SmallRng::seed_from_u64(1);
        let result = SquareWorldGenerator.generate_dungeon(30, 20, 8, &mut rng);
        assert_eq!(result.cells.len(), 30 * 20);
        assert!(!result.rooms.is_empty());
        assert_eq!(result.rooms[0].r#type, "entrance");
        // Entrance coordinates come from the first room center.
        assert_eq!(result.entrance_q, result.rooms[0].center_q.unwrap());
    }

    #[test]
    fn all_rooms_reachable_from_entrance() {
        let mut rng = SmallRng::seed_from_u64(7);
        let result = SquareWorldGenerator.generate_dungeon(30, 20, 8, &mut rng);
        let passable: HashSet<(i32, i32)> = result
            .cells
            .iter()
            .filter(|c| c.terrain != "wall")
            .map(|c| (c.q, c.r))
            .collect();
        let start = (result.entrance_q, result.entrance_r);
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([start]);
        visited.insert(start);
        while let Some((q, r)) = queue.pop_front() {
            for (dq, dr) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let n = (q + dq, r + dr);
                if passable.contains(&n) && visited.insert(n) {
                    queue.push_back(n);
                }
            }
        }
        for room in &result.rooms {
            let center = (room.center_q.unwrap(), room.center_r.unwrap());
            assert!(visited.contains(&center), "room {} unreachable", room.id);
        }
    }

    #[test]
    fn town_has_roads_plaza_and_buildings() {
        let mut rng = SmallRng::seed_from_u64(3);
        let result = SquareWorldGenerator.generate_town(20, 20, None, &mut rng);
        assert_eq!(result.cells.len(), 20 * 20);
        assert!(result.cells.iter().any(|c| c.terrain == "road"));
        assert!(result.cells.iter().any(|c| c.terrain == "plaza"));
        assert!(!result.buildings.is_empty());
    }

    #[test]
    fn primary_direction_cases() {
        assert_eq!(primary_direction(5, 1), "east");
        assert_eq!(primary_direction(-5, 1), "west");
        assert_eq!(primary_direction(1, 5), "south");
        assert_eq!(primary_direction(1, -5), "north");
        assert_eq!(primary_direction(3, 3), "southeast");
    }
}
