//! Forest biome classification for map graphics.
//!
//! Ported from `src/harsh_realm/generators/biome.py`. Forest is one terrain, but
//! the map renders boreal/temperate/tropical tree art chosen deterministically
//! from a cell's latitude (row) and neighbouring terrain.

/// Key under `cell.data` carrying the classified biome.
pub const FOREST_BIOME_KEY: &str = "forest_biome";

const NORTH_BAND: f64 = 0.34;
const SOUTH_BAND: f64 = 0.66;
const THRESHOLD: f64 = 1.0;
const LATITUDE_WEIGHT: f64 = 1.5;

fn boreal_neighbor(terrain: &str) -> f64 {
    match terrain {
        "mountains" => 1.0,
        "hills" => 0.4,
        _ => 0.0,
    }
}

fn tropical_neighbor(terrain: &str) -> f64 {
    match terrain {
        "swamp" | "water" => 1.0,
        "desert" => 0.3,
        _ => 0.0,
    }
}

/// Classify a forest cell's biome from its row and neighbouring terrain.
///
/// Returns `"temperate"`, `"boreal"`, or `"tropical"` (never fails).
pub fn classify_forest_biome(r: i32, height: i32, neighbor_terrains: &[String]) -> &'static str {
    let latitude = r as f64 / (height - 1).max(1) as f64; // 0.0 north .. 1.0 south

    let mut boreal = 0.0;
    let mut tropical = 0.0;
    if latitude <= NORTH_BAND {
        boreal += LATITUDE_WEIGHT;
    } else if latitude >= SOUTH_BAND {
        tropical += LATITUDE_WEIGHT;
    }

    for terrain in neighbor_terrains {
        boreal += boreal_neighbor(terrain);
        tropical += tropical_neighbor(terrain);
    }

    if boreal >= tropical && boreal >= THRESHOLD {
        "boreal"
    } else if tropical > boreal && tropical >= THRESHOLD {
        "tropical"
    } else {
        "temperate"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn northern_forest_is_boreal() {
        assert_eq!(classify_forest_biome(0, 100, &[]), "boreal");
    }

    #[test]
    fn southern_forest_is_tropical() {
        assert_eq!(classify_forest_biome(99, 100, &[]), "tropical");
    }

    #[test]
    fn mid_latitude_plain_is_temperate() {
        assert_eq!(classify_forest_biome(50, 100, &[]), "temperate");
    }

    #[test]
    fn neighbor_terrain_shifts_biome() {
        // Mid-latitude but mountain-adjacent -> boreal.
        let neighbors = vec!["mountains".to_string()];
        assert_eq!(classify_forest_biome(50, 100, &neighbors), "boreal");
        // Mid-latitude but swamp-adjacent -> tropical.
        let wet = vec!["swamp".to_string()];
        assert_eq!(classify_forest_biome(50, 100, &wet), "tropical");
    }
}
