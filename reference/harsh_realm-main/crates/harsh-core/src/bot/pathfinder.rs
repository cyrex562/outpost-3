//! A* pathfinder over a grid map for bot navigation.
//!
//! Ported from `src/harsh_realm/bot/pathfinder.py`.

use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::cmp::Reverse;

use crate::map::MapGrid;

/// Direction deltas for 8-way square grid movement.
const DIRECTION_DELTAS: &[(i32, i32, &str)] = &[
    (0, -1, "north"),
    (1, -1, "northeast"),
    (1, 0, "east"),
    (1, 1, "southeast"),
    (0, 1, "south"),
    (-1, 1, "southwest"),
    (-1, 0, "west"),
    (-1, -1, "northwest"),
];

fn chebyshev(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

/// A* pathfinder over a MapGrid. Traverses only cells where `passable` is true.
pub struct Pathfinder {
    passable: HashSet<(i32, i32)>,
}

impl Pathfinder {
    /// Build a pathfinder from a MapGrid.
    pub fn new(grid: &MapGrid) -> Self {
        let passable = grid.cells.iter().filter(|c| c.passable).map(|c| (c.q, c.r)).collect();
        Pathfinder { passable }
    }

    fn neighbors(&self, q: i32, r: i32) -> Vec<(i32, i32)> {
        DIRECTION_DELTAS
            .iter()
            .map(|&(dq, dr, _)| (q + dq, r + dr))
            .filter(|nb| self.passable.contains(nb))
            .collect()
    }

    /// Find a path from `start` to `goal`. Returns the list of coordinates
    /// (inclusive of both ends), or `None` if no path exists.
    pub fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
    ) -> Option<Vec<(i32, i32)>> {
        if start == goal {
            return Some(vec![start]);
        }
        if !self.passable.contains(&goal) {
            return None;
        }

        // BinaryHeap is a max-heap; wrap cost in Reverse for min-heap.
        let mut open: BinaryHeap<(Reverse<i32>, (i32, i32))> = BinaryHeap::new();
        let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
        let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();

        g_score.insert(start, 0);
        open.push((Reverse(0), start));

        while let Some((_, current)) = open.pop() {
            if current == goal {
                let mut path = vec![current];
                let mut cur = current;
                while let Some(&prev) = came_from.get(&cur) {
                    path.push(prev);
                    cur = prev;
                }
                path.reverse();
                return Some(path);
            }
            let g_cur = *g_score.get(&current).unwrap_or(&i32::MAX);
            for nb in self.neighbors(current.0, current.1) {
                let tentative = g_cur + 1;
                if tentative < *g_score.get(&nb).unwrap_or(&i32::MAX) {
                    came_from.insert(nb, current);
                    g_score.insert(nb, tentative);
                    let f = tentative + chebyshev(nb, goal);
                    open.push((Reverse(f), nb));
                }
            }
        }
        None
    }

    /// Return the compass direction string for a single-step move.
    pub fn direction_to(&self, current: (i32, i32), next: (i32, i32)) -> Option<&'static str> {
        let dq = next.0 - current.0;
        let dr = next.1 - current.1;
        DIRECTION_DELTAS.iter().find(|&&(ddq, ddr, _)| ddq == dq && ddr == dr).map(|&(_, _, dir)| dir)
    }

    /// BFS from `start`; return all passable cells reachable from it.
    pub fn all_reachable_cells(&self, start: (i32, i32)) -> Vec<(i32, i32)> {
        if !self.passable.contains(&start) {
            return vec![];
        }
        let mut visited: HashSet<(i32, i32)> = HashSet::new();
        let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
        let mut result = vec![start];
        visited.insert(start);
        queue.push_back(start);
        while let Some(current) = queue.pop_front() {
            for nb in self.neighbors(current.0, current.1) {
                if visited.insert(nb) {
                    result.push(nb);
                    queue.push_back(nb);
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{MapCell, MapGrid};

    fn make_cell(q: i32, r: i32, passable: bool) -> MapCell {
        MapCell {
            q,
            r,
            terrain: "plains".to_string(),
            passable,
            feature: None,
            feature_name: None,
            explored: true,
        }
    }

    fn grid_3x3() -> MapGrid {
        // 3x3 fully passable grid
        let cells = (0..3)
            .flat_map(|r| (0..3).map(move |q| make_cell(q, r, true)))
            .collect();
        MapGrid { width: 3, height: 3, grid_type: "square".to_string(), cells }
    }

    #[test]
    fn find_path_same_cell() {
        let pf = Pathfinder::new(&grid_3x3());
        assert_eq!(pf.find_path((0, 0), (0, 0)), Some(vec![(0, 0)]));
    }

    #[test]
    fn find_path_adjacent() {
        let pf = Pathfinder::new(&grid_3x3());
        let path = pf.find_path((0, 0), (1, 0)).unwrap();
        assert_eq!(path.first(), Some(&(0, 0)));
        assert_eq!(path.last(), Some(&(1, 0)));
    }

    #[test]
    fn find_path_diagonal() {
        let pf = Pathfinder::new(&grid_3x3());
        let path = pf.find_path((0, 0), (2, 2)).unwrap();
        assert_eq!(path.first(), Some(&(0, 0)));
        assert_eq!(path.last(), Some(&(2, 2)));
    }

    #[test]
    fn find_path_impassable_goal() {
        let mut grid = grid_3x3();
        grid.cells[4].passable = false; // (1,1) is center
        let pf = Pathfinder::new(&grid);
        assert!(pf.find_path((0, 0), (1, 1)).is_none());
    }

    #[test]
    fn direction_to_east() {
        let pf = Pathfinder::new(&grid_3x3());
        assert_eq!(pf.direction_to((0, 0), (1, 0)), Some("east"));
    }

    #[test]
    fn direction_to_southeast() {
        let pf = Pathfinder::new(&grid_3x3());
        assert_eq!(pf.direction_to((0, 0), (1, 1)), Some("southeast"));
    }

    #[test]
    fn all_reachable_full_grid() {
        let pf = Pathfinder::new(&grid_3x3());
        let reachable = pf.all_reachable_cells((0, 0));
        assert_eq!(reachable.len(), 9);
    }

    #[test]
    fn all_reachable_isolated_start() {
        let pf = Pathfinder::new(&grid_3x3());
        // Start not in passable set (use a wall cell scenario)
        let result = pf.all_reachable_cells((10, 10));
        assert!(result.is_empty());
    }
}
