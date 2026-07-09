//! Targeting + grid range (R5-P5).
//!
//! Range is wired to the grid from the start: distance is Chebyshev (the
//! `SquareGrid`'s 8-way metric), and range *bands* are world data mapping a band
//! name to a maximum cell distance. The compiler validates band references; an
//! unknown band is an unimplemented primitive.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What an action targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TargetShape {
    /// The acting entity itself.
    #[serde(rename = "self")]
    SelfTarget,
    /// One other entity.
    Single,
    /// All entities within range of a point.
    Area,
    /// Entities along a line.
    Line,
}

/// An action's targeting spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Targeting {
    pub shape: TargetShape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_band: Option<String>,
    /// Extra cells of reach beyond the band's base (e.g. a polearm). Default 0.
    #[serde(default)]
    pub reach: i64,
    #[serde(default)]
    pub requires_los: bool,
}

/// Chebyshev (8-way) grid distance between two cells — the `SquareGrid` metric.
pub fn chebyshev_distance(q1: i64, r1: i64, q2: i64, r2: i64) -> i64 {
    (q2 - q1).abs().max((r2 - r1).abs())
}

/// Whether a target at grid distance `dist` is within `band` (+ `reach`), given
/// the world's band → max-cell-distance table. Returns `None` if the band is not
/// defined (the compiler surfaces that as an unimplemented primitive).
pub fn in_range(
    band: &str,
    reach: i64,
    dist: i64,
    band_cells: &BTreeMap<String, i64>,
) -> Option<bool> {
    band_cells.get(band).map(|&max| dist <= max + reach)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chebyshev_is_eight_way() {
        assert_eq!(chebyshev_distance(0, 0, 0, 0), 0);
        assert_eq!(chebyshev_distance(0, 0, 1, 0), 1);
        assert_eq!(chebyshev_distance(0, 0, 1, 1), 1); // diagonal = 1 (8-way)
        assert_eq!(chebyshev_distance(0, 0, 3, 1), 3);
        assert_eq!(chebyshev_distance(2, 5, -1, 1), 4);
    }

    #[test]
    fn range_band_membership() {
        let mut bands = BTreeMap::new();
        bands.insert("melee".to_string(), 1);
        bands.insert("short".to_string(), 6);
        assert_eq!(in_range("melee", 0, 1, &bands), Some(true));
        assert_eq!(in_range("melee", 0, 2, &bands), Some(false));
        // reach extends the band.
        assert_eq!(in_range("melee", 1, 2, &bands), Some(true));
        assert_eq!(in_range("short", 0, 6, &bands), Some(true));
        assert_eq!(in_range("short", 0, 7, &bands), Some(false));
        // unknown band → None (unimplemented primitive at compile time).
        assert_eq!(in_range("orbital", 0, 1, &bands), None);
    }

    #[test]
    fn self_target_serializes_as_self() {
        assert_eq!(
            serde_json::to_value(TargetShape::SelfTarget).unwrap(),
            serde_json::json!("self")
        );
    }
}
