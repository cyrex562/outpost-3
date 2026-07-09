//! Water-body classification for map graphics.
//!
//! Ported from `src/harsh_realm/generators/water.py`. A water cell is a *lake*
//! cell if it belongs to any fully-water 2x2 block (the water is at least two
//! cells wide there), otherwise it is a *river* cell.

use std::collections::HashSet;

/// Key under `cell.data` carrying the classified water kind.
pub const WATER_KIND_KEY: &str = "water_kind";

/// Classify a water cell as `"lake"` (open water) or `"river"` (a 1-wide channel).
pub fn classify_water_kind(q: i32, r: i32, water: &HashSet<(i32, i32)>) -> &'static str {
    // The four 2x2 squares that contain (q, r), keyed by top-left anchor.
    for anchor_x in [q - 1, q] {
        for anchor_y in [r - 1, r] {
            let block = [
                (anchor_x, anchor_y),
                (anchor_x + 1, anchor_y),
                (anchor_x, anchor_y + 1),
                (anchor_x + 1, anchor_y + 1),
            ];
            if block.iter().all(|cell| water.contains(cell)) {
                return "lake";
            }
        }
    }
    "river"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solitary_water_is_river() {
        let water: HashSet<(i32, i32)> = [(5, 5)].into_iter().collect();
        assert_eq!(classify_water_kind(5, 5, &water), "river");
    }

    #[test]
    fn two_by_two_block_is_lake() {
        let water: HashSet<(i32, i32)> =
            [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().collect();
        for &(q, r) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
            assert_eq!(classify_water_kind(q, r, &water), "lake");
        }
    }

    #[test]
    fn diagonal_line_is_river() {
        let water: HashSet<(i32, i32)> = [(0, 0), (1, 1), (2, 2)].into_iter().collect();
        assert_eq!(classify_water_kind(1, 1, &water), "river");
    }
}
