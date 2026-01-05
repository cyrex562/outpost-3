use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::domain::{
    production_chain::ProductionRecipe, ColonyId, RailNetwork, ResourceType, RouteId, RouteNetwork,
    RouteWaypoint, SegmentId, StationId,
};

/// Error types for route generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteGenerationError {
    /// No path exists between stations
    NoPathAvailable,
    /// Station not found in rail network
    StationNotFound,
    /// No suitable production recipe found
    NoRecipeFound,
    /// Invalid production recipe
    InvalidRecipe,
    /// Route already exists
    DuplicateRoute,
}

impl fmt::Display for RouteGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RouteGenerationError::NoPathAvailable => write!(f, "No path available"),
            RouteGenerationError::StationNotFound => write!(f, "Station not found"),
            RouteGenerationError::NoRecipeFound => write!(f, "No recipe found"),
            RouteGenerationError::InvalidRecipe => write!(f, "Invalid recipe"),
            RouteGenerationError::DuplicateRoute => write!(f, "Duplicate route"),
        }
    }
}

/// Production chain configuration for route generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionChainConfig {
    /// Source station (produces input resource)
    pub source_station: StationId,
    /// Destination station (consumes output resource)
    pub destination_station: StationId,
    /// Resource being transported
    pub resource: ResourceType,
    /// Production recipe defining input->output relationship
    pub recipe: ProductionRecipe,
    /// Priority for this route (higher = more urgent)
    pub priority: u8,
}

/// Generated route with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedRoute {
    pub route_id: RouteId,
    pub config: ProductionChainConfig,
    pub waypoints: Vec<RouteWaypoint>,
    pub segments_count: usize,
    pub created_tick: u64,
}

/// Manages automatic route generation from production chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionChainRouteGenerator {
    /// Generated routes, keyed by route ID
    generated_routes: HashMap<RouteId, GeneratedRoute>,
    /// Next route ID to assign
    next_route_id: u64,
    /// Track resource flows for optimization
    resource_flows: HashMap<(StationId, StationId, ResourceType), Vec<RouteId>>,
}

impl ProductionChainRouteGenerator {
    pub fn new() -> Self {
        Self {
            generated_routes: HashMap::new(),
            next_route_id: 1,
            resource_flows: HashMap::new(),
        }
    }

    /// Generate a route from a production chain configuration
    pub fn generate_route(
        &mut self,
        config: ProductionChainConfig,
        _rail_network: &RailNetwork,
        current_tick: u64,
    ) -> Result<RouteId, RouteGenerationError> {
        // For now, create a simple route with just the waypoints
        // In a full implementation, this would use pathfinding
        let waypoints = vec![
            RouteWaypoint::Station {
                station_id: config.source_station,
                load: true,
                unload: false,
            },
            RouteWaypoint::Station {
                station_id: config.destination_station,
                load: false,
                unload: true,
            },
        ];

        let segments_count = 0; // Will be populated by route network

        let route_id = RouteId(self.next_route_id);
        self.next_route_id += 1;

        let generated = GeneratedRoute {
            route_id,
            config: config.clone(),
            waypoints,
            segments_count,
            created_tick: current_tick,
        };

        // Track resource flow
        let flow_key = (
            config.source_station,
            config.destination_station,
            config.resource,
        );
        self.resource_flows
            .entry(flow_key)
            .or_insert_with(Vec::new)
            .push(route_id);

        self.generated_routes.insert(route_id, generated);
        Ok(route_id)
    }

    /// Generate multiple routes from a list of production chains
    pub fn generate_routes_batch(
        &mut self,
        configs: Vec<ProductionChainConfig>,
        rail_network: &RailNetwork,
        current_tick: u64,
    ) -> Vec<Result<RouteId, RouteGenerationError>> {
        configs
            .into_iter()
            .map(|config| self.generate_route(config, rail_network, current_tick))
            .collect()
    }

    /// Find shortest path between two stations using BFS
    fn find_shortest_path(&self, _start: StationId, _end: StationId) -> Vec<SegmentId> {
        // Simplified - just return empty for now
        // Full implementation would need station->segment mapping
        Vec::new()
    }

    /// Convert segment path to waypoints with stations
    fn path_to_waypoints(&self, _path: &[SegmentId]) -> Vec<RouteWaypoint> {
        // Simplified - actual waypoints created in generate_route
        Vec::new()
    }

    /// Get generated route by ID
    pub fn get_route(&self, route_id: RouteId) -> Option<&GeneratedRoute> {
        self.generated_routes.get(&route_id)
    }

    /// Get all generated routes
    pub fn get_all_routes(&self) -> Vec<&GeneratedRoute> {
        self.generated_routes.values().collect()
    }

    /// Get routes for a specific resource flow
    pub fn get_routes_for_resource_flow(
        &self,
        source: StationId,
        destination: StationId,
        resource: ResourceType,
    ) -> Vec<&GeneratedRoute> {
        let flow_key = (source, destination, resource);
        self.resource_flows
            .get(&flow_key)
            .map(|ids| ids.iter().filter_map(|id| self.get_route(*id)).collect())
            .unwrap_or_default()
    }

    /// Get routes by destination station
    pub fn get_routes_to_station(&self, station: StationId) -> Vec<&GeneratedRoute> {
        self.generated_routes
            .values()
            .filter(|r| r.config.destination_station == station)
            .collect()
    }

    /// Get routes by source station
    pub fn get_routes_from_station(&self, station: StationId) -> Vec<&GeneratedRoute> {
        self.generated_routes
            .values()
            .filter(|r| r.config.source_station == station)
            .collect()
    }

    /// Remove a generated route
    pub fn remove_route(&mut self, route_id: RouteId) -> Option<GeneratedRoute> {
        if let Some(route) = self.generated_routes.remove(&route_id) {
            let flow_key = (
                route.config.source_station,
                route.config.destination_station,
                route.config.resource,
            );
            if let Some(ids) = self.resource_flows.get_mut(&flow_key) {
                ids.retain(|id| *id != route_id);
            }
            Some(route)
        } else {
            None
        }
    }

    /// Get total generated routes count
    pub fn route_count(&self) -> usize {
        self.generated_routes.len()
    }

    /// Get resource flows count
    pub fn resource_flow_count(&self) -> usize {
        self.resource_flows.len()
    }
}

impl Default for ProductionChainRouteGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{HexCoord, RailSegment, RailType};

    fn create_test_rail_network() -> RailNetwork {
        let mut network = RailNetwork::new();

        // Create a path: Station 1 -> Segment 1 -> Station 2 -> Segment 2 -> Station 3
        network
            .add_segment(
                SegmentId(1),
                HexCoord::new(0, 0),
                HexCoord::new(1, 0),
                RailType::Standard,
            )
            .ok();

        network
            .add_segment(
                SegmentId(2),
                HexCoord::new(1, 0),
                HexCoord::new(2, 0),
                RailType::Standard,
            )
            .ok();

        network
    }

    fn create_test_config() -> ProductionChainConfig {
        ProductionChainConfig {
            source_station: StationId(1),
            destination_station: StationId(2),
            resource: ResourceType::Steel,
            recipe: ProductionRecipe::new(
                vec![(ResourceType::IronOre, 2)],
                vec![(ResourceType::Steel, 1)],
                1,
            ),
            priority: 5,
        }
    }

    #[test]
    fn test_generator_creation() {
        let generator = ProductionChainRouteGenerator::new();
        assert_eq!(generator.route_count(), 0);
        assert_eq!(generator.resource_flow_count(), 0);
    }

    #[test]
    fn test_generate_route() {
        let mut generator = ProductionChainRouteGenerator::new();
        let network = create_test_rail_network();
        let config = create_test_config();

        let result = generator.generate_route(config, &network, 0);
        assert!(result.is_ok());
        assert_eq!(generator.route_count(), 1);
    }

    #[test]
    fn test_route_id_incrementing() {
        let mut generator = ProductionChainRouteGenerator::new();
        let network = create_test_rail_network();
        let config1 = create_test_config();
        let mut config2 = create_test_config();
        config2.resource = ResourceType::Food;

        let id1 = generator.generate_route(config1, &network, 0).unwrap();
        let id2 = generator.generate_route(config2, &network, 0).unwrap();

        assert_ne!(id1, id2);
        assert_eq!(id1, RouteId(1));
        assert_eq!(id2, RouteId(2));
    }

    #[test]
    fn test_get_route() {
        let mut generator = ProductionChainRouteGenerator::new();
        let network = create_test_rail_network();
        let config = create_test_config();

        let route_id = generator.generate_route(config, &network, 0).unwrap();
        let route = generator.get_route(route_id);

        assert!(route.is_some());
        assert_eq!(route.unwrap().route_id, route_id);
    }

    #[test]
    fn test_get_routes_from_station() {
        let mut generator = ProductionChainRouteGenerator::new();
        let network = create_test_rail_network();

        let config1 = create_test_config();
        generator.generate_route(config1, &network, 0).ok();

        let routes = generator.get_routes_from_station(StationId(1));
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].config.source_station, StationId(1));
    }

    #[test]
    fn test_get_routes_to_station() {
        let mut generator = ProductionChainRouteGenerator::new();
        let network = create_test_rail_network();

        let config1 = create_test_config();
        generator.generate_route(config1, &network, 0).ok();

        let routes = generator.get_routes_to_station(StationId(2));
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].config.destination_station, StationId(2));
    }

    #[test]
    fn test_remove_route() {
        let mut generator = ProductionChainRouteGenerator::new();
        let network = create_test_rail_network();
        let config = create_test_config();

        let route_id = generator.generate_route(config, &network, 0).unwrap();
        assert_eq!(generator.route_count(), 1);

        let removed = generator.remove_route(route_id);
        assert!(removed.is_some());
        assert_eq!(generator.route_count(), 0);
    }

    #[test]
    fn test_resource_flow_tracking() {
        let mut generator = ProductionChainRouteGenerator::new();
        let network = create_test_rail_network();

        let config1 = ProductionChainConfig {
            source_station: StationId(1),
            destination_station: StationId(2),
            resource: ResourceType::Steel,
            recipe: ProductionRecipe::new(
                vec![(ResourceType::IronOre, 2)],
                vec![(ResourceType::Steel, 1)],
                1,
            ),
            priority: 5,
        };

        let config2 = ProductionChainConfig {
            source_station: StationId(1),
            destination_station: StationId(2),
            resource: ResourceType::Steel,
            recipe: ProductionRecipe::new(
                vec![(ResourceType::IronOre, 2)],
                vec![(ResourceType::Steel, 1)],
                1,
            ),
            priority: 7,
        };

        generator.generate_route(config1, &network, 0).ok();
        generator.generate_route(config2, &network, 0).ok();

        let routes =
            generator.get_routes_for_resource_flow(StationId(1), StationId(2), ResourceType::Steel);
        assert_eq!(routes.len(), 2);
    }

    #[test]
    fn test_generate_batch() {
        let mut generator = ProductionChainRouteGenerator::new();
        let network = RailNetwork::new();

        let configs = vec![
            create_test_config(),
            ProductionChainConfig {
                source_station: StationId(1),
                destination_station: StationId(2),
                resource: ResourceType::Food,
                recipe: ProductionRecipe::new(
                    vec![(ResourceType::IronOre, 3)],
                    vec![(ResourceType::Food, 2)],
                    1,
                ),
                priority: 3,
            },
        ];

        let results = generator.generate_routes_batch(configs, &network, 0);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
        assert_eq!(generator.route_count(), 2);
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            RouteGenerationError::NoPathAvailable.to_string(),
            "No path available"
        );
        assert_eq!(
            RouteGenerationError::StationNotFound.to_string(),
            "Station not found"
        );
        assert_eq!(
            RouteGenerationError::DuplicateRoute.to_string(),
            "Duplicate route"
        );
    }

    #[test]
    fn test_generated_route_creation() {
        let config = create_test_config();
        let waypoints = vec![RouteWaypoint::Segment {
            segment_id: SegmentId(1),
        }];

        let route = GeneratedRoute {
            route_id: RouteId(1),
            config: config.clone(),
            waypoints,
            segments_count: 1,
            created_tick: 0,
        };

        assert_eq!(route.route_id, RouteId(1));
        assert_eq!(route.config.resource, ResourceType::Steel);
        assert_eq!(route.segments_count, 1);
    }

    #[test]
    fn test_production_chain_config_creation() {
        let config = create_test_config();
        assert_eq!(config.source_station, StationId(1));
        assert_eq!(config.destination_station, StationId(2));
        assert_eq!(config.resource, ResourceType::Steel);
        assert_eq!(config.priority, 5);
    }
}
