//! Travel goal model and A* pathfinder for right-click map travel.
//!
//! Ported from `src/harsh_realm/gm/travel.py`.

use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::cmp::Reverse;

use serde::{Deserialize, Serialize};

use crate::grid::{Grid, GridCoord, SquareGrid};

/// `gm_state` key under which the active travel goal is persisted.
pub const TRAVEL_GOAL_KEY: &str = "travel_goal";

/// Arrival action when a travel journey completes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TravelAction {
    Move,
    Look,
    Enter,
    Skill,
}

impl Default for TravelAction {
    fn default() -> Self {
        TravelAction::Move
    }
}

/// Status of the travel journey.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TravelStatus {
    Active,
    Interrupted,
}

impl Default for TravelStatus {
    fn default() -> Self {
        TravelStatus::Active
    }
}

/// A pending journey toward a target cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravelGoal {
    /// Target column.
    pub target_q: i32,
    /// Target row.
    pub target_r: i32,
    /// Arrival action.
    #[serde(default)]
    pub action: TravelAction,
    /// Skill id for `action == TravelAction::Skill`.
    #[serde(default)]
    pub skill: Option<String>,
    /// Journey status.
    #[serde(default)]
    pub status: TravelStatus,
}

impl TravelGoal {
    /// Serialise to JSON string for `gm_state` persistence.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialise from a `gm_state` JSON string.
    pub fn from_json(raw: &str) -> Option<Self> {
        serde_json::from_str(raw).ok()
    }
}

/// Return the shortest path from `start` to `goal` (inclusive) over passable cells,
/// or `None` if no route exists.
///
/// `passable` returns `true` when a coordinate may be stepped on.  `max_nodes`
/// caps the search to avoid runaway paths on very large maps.
pub fn find_travel_path(
    start: GridCoord,
    goal: GridCoord,
    passable: &dyn Fn(GridCoord) -> bool,
    max_nodes: usize,
) -> Option<Vec<GridCoord>> {
    let grid = SquareGrid::default();
    if start == goal {
        return Some(vec![start]);
    }
    if !passable(goal) {
        return None;
    }

    // Min-heap entries: (f_score, g_score, coord)
    let mut heap: BinaryHeap<Reverse<(i32, i32, i32, i32)>> = BinaryHeap::new();
    let h = chebyshev(start, goal);
    heap.push(Reverse((h, 0, start.q, start.r)));

    let mut came_from: BTreeMap<(i32, i32), (i32, i32)> = BTreeMap::new();
    let mut g_score: BTreeMap<(i32, i32), i32> = BTreeMap::new();
    g_score.insert((start.q, start.r), 0);
    let mut visited: BTreeSet<(i32, i32)> = BTreeSet::new();

    while let Some(Reverse((_f, g, cq, cr))) = heap.pop() {
        if visited.len() >= max_nodes {
            break;
        }
        let curr = (cq, cr);
        if curr == (goal.q, goal.r) {
            return Some(reconstruct(&came_from, goal, start));
        }
        if visited.contains(&curr) {
            continue;
        }
        visited.insert(curr);

        for nb in grid.neighbors(GridCoord { q: cq, r: cr }) {
            let nk = (nb.q, nb.r);
            if nb != goal && !passable(nb) {
                continue;
            }
            let tentative_g = g + 1;
            if tentative_g < *g_score.get(&nk).unwrap_or(&i32::MAX) {
                came_from.insert(nk, curr);
                g_score.insert(nk, tentative_g);
                let f = tentative_g + chebyshev(nb, goal);
                heap.push(Reverse((f, tentative_g, nb.q, nb.r)));
            }
        }
    }
    None
}

fn chebyshev(a: GridCoord, b: GridCoord) -> i32 {
    (a.q - b.q).abs().max((a.r - b.r).abs())
}

fn reconstruct(
    came_from: &BTreeMap<(i32, i32), (i32, i32)>,
    goal: GridCoord,
    start: GridCoord,
) -> Vec<GridCoord> {
    let mut path = vec![];
    let mut curr = (goal.q, goal.r);
    let start_k = (start.q, start.r);
    loop {
        path.push(GridCoord { q: curr.0, r: curr.1 });
        if curr == start_k {
            break;
        }
        match came_from.get(&curr) {
            Some(&prev) => curr = prev,
            None => break,
        }
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn travel_goal_json_roundtrip() {
        let goal = TravelGoal {
            target_q: 3,
            target_r: 5,
            action: TravelAction::Look,
            skill: None,
            status: TravelStatus::Active,
        };
        let json = goal.to_json();
        let restored = TravelGoal::from_json(&json).unwrap();
        assert_eq!(goal, restored);
    }

    #[test]
    fn travel_goal_from_bad_json_returns_none() {
        assert!(TravelGoal::from_json("not valid json").is_none());
    }

    #[test]
    fn find_path_same_start_goal() {
        let coord = GridCoord { q: 0, r: 0 };
        let path = find_travel_path(coord, coord, &|_| true, 1000).unwrap();
        assert_eq!(path, vec![coord]);
    }

    #[test]
    fn find_path_adjacent() {
        let start = GridCoord { q: 0, r: 0 };
        let goal = GridCoord { q: 1, r: 0 };
        let path = find_travel_path(start, goal, &|_| true, 1000).unwrap();
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&goal));
    }

    #[test]
    fn find_path_impassable_goal_returns_none() {
        let start = GridCoord { q: 0, r: 0 };
        let goal = GridCoord { q: 5, r: 5 };
        let result = find_travel_path(start, goal, &|_| false, 1000);
        assert!(result.is_none());
    }

    #[test]
    fn find_path_around_wall() {
        let start = GridCoord { q: 0, r: 0 };
        let goal = GridCoord { q: 2, r: 0 };
        // Wall at (1,0) — must route via diagonal
        let path = find_travel_path(start, goal, &|c| !(c.q == 1 && c.r == 0), 1000).unwrap();
        assert_eq!(*path.first().unwrap(), start);
        assert_eq!(*path.last().unwrap(), goal);
        // Path must not contain the wall cell
        assert!(!path.contains(&GridCoord { q: 1, r: 0 }));
    }
}
