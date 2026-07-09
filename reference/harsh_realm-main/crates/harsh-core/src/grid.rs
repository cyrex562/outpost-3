//! Grid abstraction for Harsh Realm.
//!
//! Ported from `src/harsh_realm/models/grid.py`. A single topology is supported
//! — [`SquareGrid`] with 8-way (king-move) adjacency and Chebyshev distance —
//! used for all maps (world, towns, dungeons, interiors).

use std::fmt;

use rand::seq::SliceRandom;
use rand::RngCore;
use serde::{Deserialize, Serialize};

/// Supported grid topologies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GridType {
    /// 8-way square grid.
    Square,
}

impl GridType {
    /// Parse a grid type from its string name (e.g. `"square"`).
    pub fn parse(name: &str) -> Result<Self, GridError> {
        match name {
            "square" => Ok(GridType::Square),
            other => Err(GridError::UnknownGridType(other.to_string())),
        }
    }
}

/// Errors raised by grid operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridError {
    /// A direction name that is not one of the eight square directions.
    InvalidDirection(String),
    /// A grid-type name that has no implementation.
    UnknownGridType(String),
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GridError::InvalidDirection(d) => write!(f, "Invalid square direction: {d:?}"),
            GridError::UnknownGridType(t) => write!(f, "Unknown grid type: {t:?}"),
        }
    }
}

impl std::error::Error for GridError {}

/// A grid coordinate where `q` is column and `r` is row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridCoord {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
}

impl GridCoord {
    /// Create a coordinate.
    pub fn new(q: i32, r: i32) -> Self {
        GridCoord { q, r }
    }
}

/// The eight square directions, in canonical order, with their `(dq, dr)` offset.
const SQUARE_OFFSETS: [(&str, (i32, i32)); 8] = [
    ("north", (0, -1)),
    ("northeast", (1, -1)),
    ("east", (1, 0)),
    ("southeast", (1, 1)),
    ("south", (0, 1)),
    ("southwest", (-1, 1)),
    ("west", (-1, 0)),
    ("northwest", (-1, -1)),
];

/// The two cardinal directions adjacent to each diagonal.
const DIAGONAL_CARDINALS: [(&str, (&str, &str)); 4] = [
    ("northeast", ("north", "east")),
    ("southeast", ("south", "east")),
    ("southwest", ("south", "west")),
    ("northwest", ("north", "west")),
];

/// Behaviour every grid implementation provides.
pub trait Grid {
    /// The grid topology.
    fn grid_type(&self) -> GridType;
    /// Canonical direction names.
    fn directions(&self) -> Vec<&'static str>;
    /// The neighbouring coordinate in `direction`.
    fn neighbor(&self, coord: GridCoord, direction: &str) -> Result<GridCoord, GridError>;
    /// All neighbouring coordinates.
    fn neighbors(&self, coord: GridCoord) -> Vec<GridCoord>;
    /// Distance between two coordinates.
    fn distance(&self, a: GridCoord, b: GridCoord) -> i32;
    /// Whether `coord` lies within a `width` x `height` grid.
    fn is_valid(&self, coord: GridCoord, width: i32, height: i32) -> bool;
    /// A uniformly random neighbour of `coord`.
    fn random_neighbor(&self, coord: GridCoord, rng: &mut dyn RngCore) -> GridCoord;
}

/// 8-direction square grid using king-move adjacency.
#[derive(Debug, Clone, Copy, Default)]
pub struct SquareGrid;

impl SquareGrid {
    /// Create a square grid.
    pub fn new() -> Self {
        SquareGrid
    }

    /// Offset for a direction name, if valid.
    fn offset(direction: &str) -> Option<(i32, i32)> {
        SQUARE_OFFSETS
            .iter()
            .find(|(name, _)| *name == direction)
            .map(|(_, delta)| *delta)
    }

    /// The two cardinal directions adjacent to a diagonal, or `None` for a
    /// cardinal/unknown direction.
    pub fn diagonal_cardinals(direction: &str) -> Option<(&'static str, &'static str)> {
        DIAGONAL_CARDINALS
            .iter()
            .find(|(name, _)| *name == direction)
            .map(|(_, pair)| *pair)
    }
}

impl Grid for SquareGrid {
    fn grid_type(&self) -> GridType {
        GridType::Square
    }

    fn directions(&self) -> Vec<&'static str> {
        SQUARE_OFFSETS.iter().map(|(name, _)| *name).collect()
    }

    fn neighbor(&self, coord: GridCoord, direction: &str) -> Result<GridCoord, GridError> {
        let (dq, dr) =
            Self::offset(direction).ok_or_else(|| GridError::InvalidDirection(direction.to_string()))?;
        Ok(GridCoord::new(coord.q + dq, coord.r + dr))
    }

    fn neighbors(&self, coord: GridCoord) -> Vec<GridCoord> {
        SQUARE_OFFSETS
            .iter()
            .map(|(_, (dq, dr))| GridCoord::new(coord.q + dq, coord.r + dr))
            .collect()
    }

    fn distance(&self, a: GridCoord, b: GridCoord) -> i32 {
        (a.q - b.q).abs().max((a.r - b.r).abs())
    }

    fn is_valid(&self, coord: GridCoord, width: i32, height: i32) -> bool {
        0 <= coord.q && coord.q < width && 0 <= coord.r && coord.r < height
    }

    fn random_neighbor(&self, coord: GridCoord, rng: &mut dyn RngCore) -> GridCoord {
        let (_, (dq, dr)) = SQUARE_OFFSETS
            .choose(rng)
            .expect("SQUARE_OFFSETS is non-empty");
        GridCoord::new(coord.q + dq, coord.r + dr)
    }
}

/// Create a concrete grid implementation for the given topology.
pub fn create_grid(grid_type: GridType) -> Box<dyn Grid> {
    match grid_type {
        GridType::Square => Box::new(SquareGrid::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::Rng;
    use rand::SeedableRng;

    fn seeded(seed: u64) -> StdRng {
        StdRng::seed_from_u64(seed)
    }

    // --- GridCoord ---

    #[test]
    fn coord_equality() {
        assert_eq!(GridCoord::new(1, 2), GridCoord::new(1, 2));
        assert_ne!(GridCoord::new(1, 2), GridCoord::new(2, 1));
    }

    #[test]
    fn coord_hashable_dedups_in_set() {
        use std::collections::HashSet;
        let s: HashSet<GridCoord> =
            [GridCoord::new(0, 0), GridCoord::new(0, 0), GridCoord::new(1, 1)]
                .into_iter()
                .collect();
        assert_eq!(s.len(), 2);
    }

    // --- SquareGrid ---

    #[test]
    fn grid_type_is_square() {
        assert_eq!(SquareGrid::new().grid_type(), GridType::Square);
    }

    #[test]
    fn directions_are_the_eight_compass_names() {
        let dirs: std::collections::HashSet<&str> =
            SquareGrid::new().directions().into_iter().collect();
        let expected: std::collections::HashSet<&str> = [
            "north", "south", "east", "west", "northeast", "northwest", "southeast", "southwest",
        ]
        .into_iter()
        .collect();
        assert_eq!(dirs.len(), 8);
        assert_eq!(dirs, expected);
    }

    #[test]
    fn cardinal_neighbors() {
        let g = SquareGrid::new();
        let c = GridCoord::new(5, 5);
        assert_eq!(g.neighbor(c, "north").unwrap(), GridCoord::new(5, 4));
        assert_eq!(g.neighbor(c, "south").unwrap(), GridCoord::new(5, 6));
        assert_eq!(g.neighbor(c, "east").unwrap(), GridCoord::new(6, 5));
        assert_eq!(g.neighbor(c, "west").unwrap(), GridCoord::new(4, 5));
    }

    #[test]
    fn diagonal_neighbors() {
        let g = SquareGrid::new();
        let c = GridCoord::new(5, 5);
        assert_eq!(g.neighbor(c, "northeast").unwrap(), GridCoord::new(6, 4));
        assert_eq!(g.neighbor(c, "southeast").unwrap(), GridCoord::new(6, 6));
        assert_eq!(g.neighbor(c, "southwest").unwrap(), GridCoord::new(4, 6));
        assert_eq!(g.neighbor(c, "northwest").unwrap(), GridCoord::new(4, 4));
    }

    #[test]
    fn neighbors_count_is_eight() {
        assert_eq!(SquareGrid::new().neighbors(GridCoord::new(5, 5)).len(), 8);
    }

    #[test]
    fn distance_chebyshev() {
        let g = SquareGrid::new();
        assert_eq!(g.distance(GridCoord::new(3, 4), GridCoord::new(3, 4)), 0);
        assert_eq!(g.distance(GridCoord::new(0, 0), GridCoord::new(3, 4)), 4);
        assert_eq!(g.distance(GridCoord::new(0, 0), GridCoord::new(5, 2)), 5);
    }

    #[test]
    fn distance_to_each_neighbor_is_one() {
        let g = SquareGrid::new();
        let c = GridCoord::new(5, 5);
        for nb in g.neighbors(c) {
            assert_eq!(g.distance(c, nb), 1);
        }
    }

    #[test]
    fn is_valid_bounds() {
        let g = SquareGrid::new();
        assert!(g.is_valid(GridCoord::new(0, 0), 10, 10));
        assert!(g.is_valid(GridCoord::new(9, 9), 10, 10));
        assert!(!g.is_valid(GridCoord::new(-1, 0), 10, 10));
        assert!(!g.is_valid(GridCoord::new(10, 0), 10, 10));
    }

    #[test]
    fn invalid_direction_errors() {
        let g = SquareGrid::new();
        let err = g.neighbor(GridCoord::new(0, 0), "nne").unwrap_err();
        assert_eq!(err, GridError::InvalidDirection("nne".to_string()));
        assert!(err.to_string().contains("Invalid square direction"));
    }

    #[test]
    fn diagonal_cardinals_lookup() {
        assert_eq!(SquareGrid::diagonal_cardinals("northeast"), Some(("north", "east")));
        assert_eq!(SquareGrid::diagonal_cardinals("southwest"), Some(("south", "west")));
        assert_eq!(SquareGrid::diagonal_cardinals("north"), None);
    }

    #[test]
    fn create_grid_from_type_and_name() {
        assert_eq!(create_grid(GridType::Square).grid_type(), GridType::Square);
        assert_eq!(GridType::parse("square").unwrap(), GridType::Square);
        assert!(GridType::parse("triangle").is_err());
    }

    #[test]
    fn coord_round_trips_through_json() {
        let c = GridCoord::new(-7, 13);
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<GridCoord>(&json).unwrap(), c);
    }

    // --- Randomised property-style checks (matches dice.rs conventions) ---

    #[test]
    fn prop_distance_non_negative_and_symmetric() {
        let g = SquareGrid::new();
        let mut rng = seeded(1);
        for _ in 0..5000 {
            let a = GridCoord::new(rng.gen_range(-50..=50), rng.gen_range(-50..=50));
            let b = GridCoord::new(rng.gen_range(-50..=50), rng.gen_range(-50..=50));
            assert!(g.distance(a, b) >= 0);
            assert_eq!(g.distance(a, b), g.distance(b, a));
        }
    }

    #[test]
    fn prop_distance_triangle_inequality() {
        let g = SquareGrid::new();
        let mut rng = seeded(2);
        for _ in 0..5000 {
            let a = GridCoord::new(rng.gen_range(-50..=50), rng.gen_range(-50..=50));
            let b = GridCoord::new(rng.gen_range(-50..=50), rng.gen_range(-50..=50));
            let c = GridCoord::new(rng.gen_range(-50..=50), rng.gen_range(-50..=50));
            assert!(g.distance(a, c) <= g.distance(a, b) + g.distance(b, c));
        }
    }

    #[test]
    fn prop_neighbors_all_distance_one() {
        let g = SquareGrid::new();
        let mut rng = seeded(3);
        for _ in 0..5000 {
            let c = GridCoord::new(rng.gen_range(-50..=50), rng.gen_range(-50..=50));
            for nb in g.neighbors(c) {
                assert_eq!(g.distance(c, nb), 1);
            }
        }
    }

    #[test]
    fn prop_random_neighbor_is_a_neighbor() {
        let g = SquareGrid::new();
        let mut rng = seeded(42);
        for _ in 0..5000 {
            let c = GridCoord::new(rng.gen_range(-50..=50), rng.gen_range(-50..=50));
            let nb = g.random_neighbor(c, &mut rng);
            assert!(g.neighbors(c).contains(&nb));
        }
    }
}
