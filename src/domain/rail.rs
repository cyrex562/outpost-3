use super::Resources;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a rail segment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SegmentId(pub u64);

/// Rail type with qualitative differences in speed and cost
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RailType {
    /// Standard rail: 1.0x speed, baseline cost
    Standard,
    /// Advanced rail: 1.5x speed, 2x cost
    Advanced,
    /// Monorail: 2.0x speed, 4x cost
    Monorail,
    /// Maglev: 2.5x speed, 8x cost
    Maglev,
}

impl RailType {
    /// Speed multiplier relative to baseline
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            RailType::Standard => 1.0,
            RailType::Advanced => 1.5,
            RailType::Monorail => 2.0,
            RailType::Maglev => 2.5,
        }
    }

    /// Cost multiplier relative to baseline (Standard costs 100 per km)
    pub fn cost_multiplier(&self) -> f32 {
        match self {
            RailType::Standard => 1.0,
            RailType::Advanced => 2.0,
            RailType::Monorail => 4.0,
            RailType::Maglev => 8.0,
        }
    }

    /// Maintenance cost multiplier per turn
    pub fn maintenance_multiplier(&self) -> f32 {
        match self {
            RailType::Standard => 1.0,
            RailType::Advanced => 1.5,
            RailType::Monorail => 2.5,
            RailType::Maglev => 4.0,
        }
    }

    /// Display name for UI
    pub fn name(&self) -> &'static str {
        match self {
            RailType::Standard => "Standard",
            RailType::Advanced => "Advanced",
            RailType::Monorail => "Monorail",
            RailType::Maglev => "Maglev",
        }
    }

    /// Description for tooltips
    pub fn description(&self) -> &'static str {
        match self {
            RailType::Standard => "Standard rail: 1.0x speed, baseline cost",
            RailType::Advanced => "Advanced rail: 1.5x speed, 2x cost",
            RailType::Monorail => "Monorail: 2.0x speed, 4x cost",
            RailType::Maglev => "Maglev: 2.5x speed, 8x cost",
        }
    }
}

/// Calculate construction cost for a rail segment
pub fn calculate_construction_cost(rail_type: RailType, distance_tiles: f32) -> Resources {
    // Base cost: 100 credits per tile per Standard rail
    let base_cost = (distance_tiles * 100.0) as i64;
    let total_cost = (base_cost as f32 * rail_type.cost_multiplier()) as i64;

    let mut costs = Resources::new();
    costs.set(crate::domain::ResourceType::Credits, total_cost);

    // Steel requirement scales with type
    let steel_per_tile = 5.0;
    let steel_cost = (distance_tiles * steel_per_tile * rail_type.cost_multiplier()) as i64;
    costs.set(crate::domain::ResourceType::Steel, steel_cost);

    costs
}

/// Calculate maintenance cost per turn
pub fn calculate_maintenance_cost(rail_type: RailType) -> Resources {
    let mut costs = Resources::new();
    // Base maintenance: 5 credits per turn per Standard rail
    let maintenance = (5.0 * rail_type.maintenance_multiplier()) as i64;
    costs.set(crate::domain::ResourceType::Credits, maintenance);
    costs
}

/// Hexagonal coordinate for grid-based map
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

impl HexCoord {
    pub fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// Get s-coordinate (derived: s = -q - r)
    pub fn s(&self) -> i32 {
        -self.q - self.r
    }

    /// Distance between two hexagons (in hex grid distance)
    pub fn distance(&self, other: &HexCoord) -> u32 {
        ((self.q.abs_diff(other.q) as u32
            + self.r.abs_diff(other.r) as u32
            + self.s().abs_diff(other.s()) as u32)
            / 2) as u32
    }

    /// Check if hexagons are adjacent
    pub fn is_adjacent(&self, other: &HexCoord) -> bool {
        self.distance(other) == 1
    }
}

/// A rail segment connecting two adjacent hexagons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RailSegment {
    pub id: SegmentId,
    pub from_tile: HexCoord,
    pub to_tile: HexCoord,
    pub rail_type: RailType,
    /// Number of parallel tracks (1-8)
    pub track_count: u8,
}

impl RailSegment {
    pub fn new(id: SegmentId, from_tile: HexCoord, to_tile: HexCoord, rail_type: RailType) -> Self {
        Self {
            id,
            from_tile,
            to_tile,
            rail_type,
            track_count: 1,
        }
    }

    /// Get construction cost for this segment
    pub fn construction_cost(&self) -> Resources {
        let distance = self.from_tile.distance(&self.to_tile) as f32;
        calculate_construction_cost(self.rail_type, distance)
    }

    /// Get maintenance cost per turn
    pub fn maintenance_cost(&self) -> Resources {
        calculate_maintenance_cost(self.rail_type)
    }

    /// Check if segment can be upgraded to a new type
    pub fn can_upgrade(&self, new_type: RailType) -> bool {
        // Can upgrade to more advanced types
        let upgrade_order = [
            RailType::Standard,
            RailType::Advanced,
            RailType::Monorail,
            RailType::Maglev,
        ];

        let current_idx = upgrade_order
            .iter()
            .position(|&t| t == self.rail_type)
            .unwrap_or(0);
        let new_idx = upgrade_order
            .iter()
            .position(|&t| t == new_type)
            .unwrap_or(0);

        new_idx > current_idx
    }

    /// Get the speed factor for this segment
    pub fn speed_factor(&self) -> f32 {
        self.rail_type.speed_multiplier()
    }

    /// Get maximum trains that can occupy this segment simultaneously
    pub fn max_trains_per_segment(&self) -> u32 {
        // More tracks = more trains can move through simultaneously
        self.track_count as u32
    }

    /// Add a parallel track (up to 8 maximum)
    pub fn add_track(&mut self) -> bool {
        if self.track_count < 8 {
            self.track_count += 1;
            true
        } else {
            false
        }
    }
}

/// Rail network as a directed graph of segments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RailNetwork {
    /// All segments indexed by ID
    pub segments: HashMap<SegmentId, RailSegment>,
    /// Adjacency list: hex -> list of segment IDs connected to that hex
    pub adjacency: HashMap<HexCoord, Vec<SegmentId>>,
}

impl RailNetwork {
    pub fn new() -> Self {
        Self {
            segments: HashMap::new(),
            adjacency: HashMap::new(),
        }
    }

    /// Add a new segment to the network
    /// Returns error if hexagons are not adjacent
    pub fn add_segment(
        &mut self,
        segment_id: SegmentId,
        from: HexCoord,
        to: HexCoord,
        rail_type: RailType,
    ) -> Result<(), String> {
        if !from.is_adjacent(&to) {
            return Err("Hexagons must be adjacent to connect by rail".to_string());
        }

        let segment = RailSegment::new(segment_id, from, to, rail_type);
        self.segments.insert(segment_id, segment);

        // Add to adjacency list in both directions (bidirectional graph)
        self.adjacency
            .entry(from)
            .or_insert_with(Vec::new)
            .push(segment_id);
        self.adjacency
            .entry(to)
            .or_insert_with(Vec::new)
            .push(segment_id);

        Ok(())
    }

    /// Remove a segment from the network
    pub fn remove_segment(&mut self, segment_id: SegmentId) -> Result<(), String> {
        let segment = self
            .segments
            .remove(&segment_id)
            .ok_or("Segment not found".to_string())?;

        // Remove from adjacency list
        if let Some(adjacent_segments) = self.adjacency.get_mut(&segment.from_tile) {
            adjacent_segments.retain(|&id| id != segment_id);
        }
        if let Some(adjacent_segments) = self.adjacency.get_mut(&segment.to_tile) {
            adjacent_segments.retain(|&id| id != segment_id);
        }

        Ok(())
    }

    /// Get neighboring hexagons from a given hex
    pub fn get_neighbors(&self, hex: HexCoord) -> Vec<HexCoord> {
        self.adjacency
            .get(&hex)
            .map(|segment_ids| {
                segment_ids
                    .iter()
                    .flat_map(|&segment_id| {
                        self.segments.get(&segment_id).map(|segment| {
                            if segment.from_tile == hex {
                                segment.to_tile
                            } else {
                                segment.from_tile
                            }
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if a path exists between two hexagons
    pub fn path_exists(&self, from: HexCoord, to: HexCoord) -> bool {
        self.find_shortest_path(from, to).is_some()
    }

    /// Find shortest path between two hexagons using BFS
    pub fn find_shortest_path(&self, from: HexCoord, to: HexCoord) -> Option<Vec<HexCoord>> {
        if from == to {
            return Some(vec![from]);
        }

        use std::collections::VecDeque;

        let mut queue = VecDeque::new();
        let mut visited = std::collections::HashSet::new();
        let mut parent = std::collections::HashMap::new();

        queue.push_back(from);
        visited.insert(from);

        while let Some(current) = queue.pop_front() {
            for neighbor in self.get_neighbors(current) {
                if neighbor == to {
                    // Found the target, reconstruct path
                    let mut path = vec![to];
                    let mut node = current;
                    while node != from {
                        path.push(node);
                        node = parent[&node];
                    }
                    path.push(from);
                    path.reverse();
                    return Some(path);
                }

                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }
        }

        None
    }

    /// Get all segment IDs on a given path
    pub fn find_segments_on_path(&self, path: &[HexCoord]) -> Vec<SegmentId> {
        let mut segments = Vec::new();

        for i in 0..path.len() - 1 {
            let from = path[i];
            let to = path[i + 1];

            // Find the segment connecting these two hexagons
            if let Some(segment_ids) = self.adjacency.get(&from) {
                for &segment_id in segment_ids {
                    if let Some(segment) = self.segments.get(&segment_id) {
                        if (segment.from_tile == from && segment.to_tile == to)
                            || (segment.from_tile == to && segment.to_tile == from)
                        {
                            segments.push(segment_id);
                            break;
                        }
                    }
                }
            }
        }

        segments
    }

    /// Get the segment connecting two adjacent hexagons
    pub fn get_segment_between(&self, hex1: HexCoord, hex2: HexCoord) -> Option<SegmentId> {
        if let Some(segment_ids) = self.adjacency.get(&hex1) {
            for &segment_id in segment_ids {
                if let Some(segment) = self.segments.get(&segment_id) {
                    if (segment.from_tile == hex1 && segment.to_tile == hex2)
                        || (segment.from_tile == hex2 && segment.to_tile == hex1)
                    {
                        return Some(segment_id);
                    }
                }
            }
        }
        None
    }

    /// Check if a segment exists between two hexagons
    pub fn segment_exists(&self, hex1: HexCoord, hex2: HexCoord) -> bool {
        self.get_segment_between(hex1, hex2).is_some()
    }

    /// Get a reference to a segment by ID
    pub fn get_segment(&self, segment_id: SegmentId) -> Option<&RailSegment> {
        self.segments.get(&segment_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rail_type_speed_multipliers() {
        assert_eq!(RailType::Standard.speed_multiplier(), 1.0);
        assert_eq!(RailType::Advanced.speed_multiplier(), 1.5);
        assert_eq!(RailType::Monorail.speed_multiplier(), 2.0);
        assert_eq!(RailType::Maglev.speed_multiplier(), 2.5);
    }

    #[test]
    fn test_rail_type_cost_multipliers() {
        assert_eq!(RailType::Standard.cost_multiplier(), 1.0);
        assert_eq!(RailType::Advanced.cost_multiplier(), 2.0);
        assert_eq!(RailType::Monorail.cost_multiplier(), 4.0);
        assert_eq!(RailType::Maglev.cost_multiplier(), 8.0);
    }

    #[test]
    fn test_rail_type_maintenance_multipliers() {
        assert_eq!(RailType::Standard.maintenance_multiplier(), 1.0);
        assert_eq!(RailType::Advanced.maintenance_multiplier(), 1.5);
        assert_eq!(RailType::Monorail.maintenance_multiplier(), 2.5);
        assert_eq!(RailType::Maglev.maintenance_multiplier(), 4.0);
    }

    #[test]
    fn test_rail_types_ordered_by_speed() {
        let types = vec![
            RailType::Standard,
            RailType::Advanced,
            RailType::Monorail,
            RailType::Maglev,
        ];

        for i in 0..types.len() - 1 {
            assert!(
                types[i].speed_multiplier() < types[i + 1].speed_multiplier(),
                "{} should be slower than {}",
                types[i].name(),
                types[i + 1].name()
            );
        }
    }

    #[test]
    fn test_construction_cost_scales_with_distance() {
        let cost_1_tile = calculate_construction_cost(RailType::Standard, 1.0);
        let cost_2_tiles = calculate_construction_cost(RailType::Standard, 2.0);

        assert!(
            cost_2_tiles.get(crate::domain::ResourceType::Credits)
                > cost_1_tile.get(crate::domain::ResourceType::Credits)
        );
        assert_eq!(
            cost_2_tiles.get(crate::domain::ResourceType::Credits),
            cost_1_tile.get(crate::domain::ResourceType::Credits) * 2
        );
    }

    #[test]
    fn test_construction_cost_scales_with_rail_type() {
        let distance = 1.0;
        let standard_cost = calculate_construction_cost(RailType::Standard, distance);
        let advanced_cost = calculate_construction_cost(RailType::Advanced, distance);
        let monorail_cost = calculate_construction_cost(RailType::Monorail, distance);
        let maglev_cost = calculate_construction_cost(RailType::Maglev, distance);

        let standard_credits = standard_cost.get(crate::domain::ResourceType::Credits);
        let advanced_credits = advanced_cost.get(crate::domain::ResourceType::Credits);
        let monorail_credits = monorail_cost.get(crate::domain::ResourceType::Credits);
        let maglev_credits = maglev_cost.get(crate::domain::ResourceType::Credits);

        // Verify cost increases with rail type
        assert!(advanced_credits > standard_credits);
        assert!(monorail_credits > advanced_credits);
        assert!(maglev_credits > monorail_credits);

        // Verify multipliers
        assert_eq!(advanced_credits, standard_credits * 2);
        assert_eq!(monorail_credits, standard_credits * 4);
        assert_eq!(maglev_credits, standard_credits * 8);
    }

    #[test]
    fn test_maintenance_cost_includes_both_types() {
        let cost = calculate_maintenance_cost(RailType::Standard);
        assert!(cost.get(crate::domain::ResourceType::Credits) > 0);
    }

    #[test]
    fn test_rail_type_names_unique() {
        let types = vec![
            RailType::Standard,
            RailType::Advanced,
            RailType::Monorail,
            RailType::Maglev,
        ];

        let names: Vec<&str> = types.iter().map(|t| t.name()).collect();
        assert_eq!(
            names.len(),
            names.iter().collect::<std::collections::HashSet<_>>().len()
        );
    }

    #[test]
    fn test_construction_cost_includes_steel() {
        let cost = calculate_construction_cost(RailType::Standard, 1.0);
        assert!(cost.get(crate::domain::ResourceType::Steel) > 0);
    }

    #[test]
    fn test_hex_coord_distance() {
        let origin = HexCoord::new(0, 0);
        let adjacent = HexCoord::new(1, 0);
        let two_away = HexCoord::new(2, 0);

        assert_eq!(origin.distance(&origin), 0);
        assert_eq!(origin.distance(&adjacent), 1);
        assert_eq!(origin.distance(&two_away), 2);
    }

    #[test]
    fn test_hex_coord_adjacency() {
        let origin = HexCoord::new(0, 0);
        let adjacent1 = HexCoord::new(1, 0);
        let adjacent2 = HexCoord::new(0, 1);
        let diagonal = HexCoord::new(1, 1);

        assert!(origin.is_adjacent(&adjacent1));
        assert!(origin.is_adjacent(&adjacent2));
        assert!(!origin.is_adjacent(&diagonal));
    }

    #[test]
    fn test_rail_segment_creation() {
        let from = HexCoord::new(0, 0);
        let to = HexCoord::new(1, 0);
        let segment = RailSegment::new(SegmentId(1), from, to, RailType::Standard);

        assert_eq!(segment.id, SegmentId(1));
        assert_eq!(segment.from_tile, from);
        assert_eq!(segment.to_tile, to);
        assert_eq!(segment.rail_type, RailType::Standard);
        assert_eq!(segment.track_count, 1);
    }

    #[test]
    fn test_rail_segment_cost_calculation() {
        let from = HexCoord::new(0, 0);
        let to = HexCoord::new(1, 0);
        let segment = RailSegment::new(SegmentId(1), from, to, RailType::Standard);

        let cost = segment.construction_cost();
        assert!(cost.get(crate::domain::ResourceType::Credits) > 0);
        assert!(cost.get(crate::domain::ResourceType::Steel) > 0);
    }

    #[test]
    fn test_rail_segment_upgrade_path() {
        let from = HexCoord::new(0, 0);
        let to = HexCoord::new(1, 0);
        let mut segment = RailSegment::new(SegmentId(1), from, to, RailType::Standard);

        // Can upgrade to Advanced
        assert!(segment.can_upgrade(RailType::Advanced));
        assert!(segment.can_upgrade(RailType::Monorail));
        assert!(segment.can_upgrade(RailType::Maglev));

        // Cannot upgrade to same or lower
        assert!(!segment.can_upgrade(RailType::Standard));

        // After upgrade, Standard can't be set again
        segment.rail_type = RailType::Advanced;
        assert!(!segment.can_upgrade(RailType::Standard));
        assert!(segment.can_upgrade(RailType::Monorail));
    }

    #[test]
    fn test_rail_segment_track_count() {
        let from = HexCoord::new(0, 0);
        let to = HexCoord::new(1, 0);
        let mut segment = RailSegment::new(SegmentId(1), from, to, RailType::Standard);

        assert_eq!(segment.track_count, 1);
        assert_eq!(segment.max_trains_per_segment(), 1);

        // Add tracks
        for i in 2..=8 {
            assert!(segment.add_track());
            assert_eq!(segment.track_count, i);
            assert_eq!(segment.max_trains_per_segment(), i as u32);
        }

        // Cannot exceed 8
        assert!(!segment.add_track());
        assert_eq!(segment.track_count, 8);
    }

    #[test]
    fn test_rail_segment_speed_factor() {
        let from = HexCoord::new(0, 0);
        let to = HexCoord::new(1, 0);

        let standard = RailSegment::new(SegmentId(1), from, to, RailType::Standard);
        let advanced = RailSegment::new(SegmentId(2), from, to, RailType::Advanced);

        assert_eq!(standard.speed_factor(), 1.0);
        assert_eq!(advanced.speed_factor(), 1.5);
    }

    #[test]
    fn test_rail_segment_maintenance_cost() {
        let from = HexCoord::new(0, 0);
        let to = HexCoord::new(1, 0);
        let segment = RailSegment::new(SegmentId(1), from, to, RailType::Maglev);

        let cost = segment.maintenance_cost();
        assert!(cost.get(crate::domain::ResourceType::Credits) > 0);
    }

    #[test]
    fn test_rail_network_creation() {
        let network = RailNetwork::new();
        assert!(network.segments.is_empty());
        assert!(network.adjacency.is_empty());
    }

    #[test]
    fn test_rail_network_add_segment() {
        let mut network = RailNetwork::new();
        let from = HexCoord::new(0, 0);
        let to = HexCoord::new(1, 0);

        let result = network.add_segment(SegmentId(1), from, to, RailType::Standard);
        assert!(result.is_ok());
        assert_eq!(network.segments.len(), 1);
        assert!(network.adjacency.contains_key(&from));
        assert!(network.adjacency.contains_key(&to));
    }

    #[test]
    fn test_rail_network_add_non_adjacent() {
        let mut network = RailNetwork::new();
        let from = HexCoord::new(0, 0);
        let non_adjacent = HexCoord::new(2, 0);

        let result = network.add_segment(SegmentId(1), from, non_adjacent, RailType::Standard);
        assert!(result.is_err());
        assert_eq!(network.segments.len(), 0);
    }

    #[test]
    fn test_rail_network_get_neighbors() {
        let mut network = RailNetwork::new();
        let origin = HexCoord::new(0, 0);
        let neighbor1 = HexCoord::new(1, 0);
        let neighbor2 = HexCoord::new(0, 1);

        network
            .add_segment(SegmentId(1), origin, neighbor1, RailType::Standard)
            .unwrap();
        network
            .add_segment(SegmentId(2), origin, neighbor2, RailType::Standard)
            .unwrap();

        let neighbors = network.get_neighbors(origin);
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&neighbor1));
        assert!(neighbors.contains(&neighbor2));
    }

    #[test]
    fn test_rail_network_remove_segment() {
        let mut network = RailNetwork::new();
        let from = HexCoord::new(0, 0);
        let to = HexCoord::new(1, 0);

        network
            .add_segment(SegmentId(1), from, to, RailType::Standard)
            .unwrap();
        assert_eq!(network.segments.len(), 1);

        let result = network.remove_segment(SegmentId(1));
        assert!(result.is_ok());
        assert_eq!(network.segments.len(), 0);
    }

    #[test]
    fn test_rail_network_path_exists() {
        let mut network = RailNetwork::new();
        let a = HexCoord::new(0, 0);
        let b = HexCoord::new(1, 0);
        let c = HexCoord::new(2, 0);

        network
            .add_segment(SegmentId(1), a, b, RailType::Standard)
            .unwrap();
        network
            .add_segment(SegmentId(2), b, c, RailType::Standard)
            .unwrap();

        assert!(network.path_exists(a, b));
        assert!(network.path_exists(a, c));
        assert!(!network.path_exists(a, HexCoord::new(10, 10)));
    }

    #[test]
    fn test_rail_network_find_shortest_path() {
        let mut network = RailNetwork::new();
        let a = HexCoord::new(0, 0);
        let b = HexCoord::new(1, 0);
        let c = HexCoord::new(2, 0);
        let d = HexCoord::new(1, 1);

        network
            .add_segment(SegmentId(1), a, b, RailType::Standard)
            .unwrap();
        network
            .add_segment(SegmentId(2), b, c, RailType::Standard)
            .unwrap();
        network
            .add_segment(SegmentId(3), b, d, RailType::Standard)
            .unwrap();

        let path = network.find_shortest_path(a, c);
        assert!(path.is_some());

        let path = path.unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], a);
        assert_eq!(path[1], b);
        assert_eq!(path[2], c);
    }

    #[test]
    fn test_rail_network_find_shortest_path_same_hex() {
        let network = RailNetwork::new();
        let a = HexCoord::new(0, 0);

        let path = network.find_shortest_path(a, a);
        assert!(path.is_some());
        assert_eq!(path.unwrap(), vec![a]);
    }

    #[test]
    fn test_rail_network_find_segments_on_path() {
        let mut network = RailNetwork::new();
        let a = HexCoord::new(0, 0);
        let b = HexCoord::new(1, 0);
        let c = HexCoord::new(2, 0);

        network
            .add_segment(SegmentId(1), a, b, RailType::Standard)
            .unwrap();
        network
            .add_segment(SegmentId(2), b, c, RailType::Standard)
            .unwrap();

        let path = vec![a, b, c];
        let segments = network.find_segments_on_path(&path);

        assert_eq!(segments.len(), 2);
        assert!(segments.contains(&SegmentId(1)));
        assert!(segments.contains(&SegmentId(2)));
    }

    #[test]
    fn test_rail_network_get_segment_between() {
        let mut network = RailNetwork::new();
        let from = HexCoord::new(0, 0);
        let to = HexCoord::new(1, 0);

        network
            .add_segment(SegmentId(1), from, to, RailType::Advanced)
            .unwrap();

        let segment_id = network.get_segment_between(from, to);
        assert!(segment_id.is_some());
        assert_eq!(segment_id.unwrap(), SegmentId(1));

        let segment = network.get_segment(SegmentId(1)).unwrap();
        assert_eq!(segment.rail_type, RailType::Advanced);

        // Test bidirectionality
        let segment_reversed = network.get_segment_between(to, from);
        assert!(segment_reversed.is_some());
        assert_eq!(segment_reversed.unwrap(), SegmentId(1));
    }

    #[test]
    fn test_rail_network_segment_exists() {
        let mut network = RailNetwork::new();
        let a = HexCoord::new(0, 0);
        let b = HexCoord::new(1, 0);
        let c = HexCoord::new(2, 0);

        network
            .add_segment(SegmentId(1), a, b, RailType::Standard)
            .unwrap();

        assert!(network.segment_exists(a, b));
        assert!(network.segment_exists(b, a)); // Bidirectional
        assert!(!network.segment_exists(a, c));
    }
}
