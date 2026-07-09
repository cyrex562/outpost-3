//! Map data models for the world map API endpoint.
//!
//! Ported from `src/harsh_realm/models/map.py`.

use serde::{Deserialize, Serialize};

/// A single cell in the world map grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapCell {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Terrain id.
    pub terrain: String,
    /// Whether the cell is passable.
    pub passable: bool,
    /// Feature id, if any.
    pub feature: Option<String>,
    /// Feature display name, if any.
    pub feature_name: Option<String>,
    /// Whether the cell has been explored.
    pub explored: bool,
}

/// The full world map grid returned by the map API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapGrid {
    /// Grid width.
    pub width: i32,
    /// Grid height.
    pub height: i32,
    /// Grid topology name.
    pub grid_type: String,
    /// Cells.
    pub cells: Vec<MapCell>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_grid_round_trips() {
        let g = MapGrid {
            width: 2,
            height: 1,
            grid_type: "square".to_string(),
            cells: vec![MapCell {
                q: 0,
                r: 0,
                terrain: "plains".to_string(),
                passable: true,
                feature: None,
                feature_name: None,
                explored: true,
            }],
        };
        let json = serde_json::to_string(&g).unwrap();
        assert_eq!(serde_json::from_str::<MapGrid>(&json).unwrap(), g);
    }
}
