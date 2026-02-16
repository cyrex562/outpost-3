use serde::{Deserialize, Serialize};
use std::fmt;

use crate::domain::{ColonyId, RouteId, SegmentId, StationId};

/// Route waypoint - either a station stop or rail segment transit point
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteWaypoint {
    /// Stop at a station for cargo operations
    Station {
        station_id: StationId,
        load: bool,   // true if loading, false if unloading only
        unload: bool, // true if unloading, false if loading only
    },
    /// Transit through a rail segment (part of the path, not a stop)
    Segment { segment_id: SegmentId },
}

/// A train route defines a path through the rail network with stops for cargo operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainRoute {
    pub id: RouteId,
    pub colony_id: ColonyId,
    pub origin: StationId,
    pub destination: StationId,
    /// Ordered list of waypoints (stations and segments)
    pub waypoints: Vec<RouteWaypoint>,
    /// Current waypoint index for trains following this route
    pub current_waypoint: usize,
    /// Total segments in route (for progress tracking)
    pub total_segments: usize,
    /// Is this route currently active?
    pub active: bool,
    /// Priority for route conflict resolution (higher = more priority)
    pub priority: u8,
}

impl TrainRoute {
    /// Create a new route from origin to destination with waypoints
    pub fn new(
        id: RouteId,
        colony_id: ColonyId,
        origin: StationId,
        destination: StationId,
        waypoints: Vec<RouteWaypoint>,
        priority: u8,
    ) -> Self {
        let total_segments = waypoints
            .iter()
            .filter(|w| matches!(w, RouteWaypoint::Segment { .. }))
            .count();

        Self {
            id,
            colony_id,
            origin,
            destination,
            waypoints,
            current_waypoint: 0,
            total_segments,
            active: true,
            priority,
        }
    }

    /// Get current waypoint without advancing
    pub fn current(&self) -> Option<&RouteWaypoint> {
        self.waypoints.get(self.current_waypoint)
    }

    /// Advance to next waypoint
    pub fn advance_waypoint(&mut self) -> bool {
        if self.current_waypoint < self.waypoints.len() - 1 {
            self.current_waypoint += 1;
            true
        } else {
            false
        }
    }

    /// Check if route is complete (reached destination)
    pub fn is_complete(&self) -> bool {
        self.current_waypoint >= self.waypoints.len()
    }

    /// Reset route to beginning
    pub fn reset(&mut self) {
        self.current_waypoint = 0;
    }

    /// Get remaining distance in segments (including current if on a segment)
    pub fn remaining_segments(&self) -> usize {
        self.waypoints
            .iter()
            .skip(self.current_waypoint)
            .filter(|w| matches!(w, RouteWaypoint::Segment { .. }))
            .count()
    }

    /// Get progress as percentage (0.0-100.0)
    /// Progress increases as we move through waypoints
    pub fn progress_percent(&self) -> f32 {
        if self.waypoints.is_empty() {
            return 100.0;
        }

        // Progress is (waypoints traversed) / (total waypoints - 1)
        // -1 because we don't count staying at the final destination as progress
        let max_waypoint = self.waypoints.len().saturating_sub(1);
        if max_waypoint == 0 {
            return 100.0;
        }

        (self.current_waypoint as f32 / max_waypoint as f32) * 100.0
    }
}

/// Route validation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    /// Route has no waypoints
    EmptyRoute,
    /// Origin and destination must be different
    SameOriginDestination,
    /// Route doesn't start at origin station
    InvalidOrigin,
    /// Route doesn't end at destination station
    InvalidDestination,
    /// Waypoint sequence is invalid (e.g., disconnected segments)
    DisconnectedPath,
    /// Route already exists
    RouteExists,
    /// Invalid priority value
    InvalidPriority,
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RouteError::EmptyRoute => write!(f, "Route has no waypoints"),
            RouteError::SameOriginDestination => {
                write!(f, "Origin and destination must be different")
            }
            RouteError::InvalidOrigin => write!(f, "Route doesn't start at origin station"),
            RouteError::InvalidDestination => write!(f, "Route doesn't end at destination station"),
            RouteError::DisconnectedPath => write!(f, "Waypoint sequence is invalid"),
            RouteError::RouteExists => write!(f, "Route already exists"),
            RouteError::InvalidPriority => write!(f, "Invalid priority value"),
        }
    }
}

/// Route manager for a colony
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteNetwork {
    routes: Vec<TrainRoute>,
    next_route_id: u64,
}

impl RouteNetwork {
    /// Create a new route network
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            next_route_id: 1,
        }
    }

    /// Create and add a new route
    pub fn create_route(
        &mut self,
        colony_id: ColonyId,
        origin: StationId,
        destination: StationId,
        waypoints: Vec<RouteWaypoint>,
        priority: u8,
    ) -> Result<RouteId, RouteError> {
        // Validation
        if waypoints.is_empty() {
            return Err(RouteError::EmptyRoute);
        }
        if origin == destination {
            return Err(RouteError::SameOriginDestination);
        }
        if priority > 10 {
            return Err(RouteError::InvalidPriority);
        }

        // Check origin is first waypoint
        if !matches!(&waypoints[0], RouteWaypoint::Station { station_id, .. } if *station_id == origin)
        {
            return Err(RouteError::InvalidOrigin);
        }

        // Check destination is last waypoint
        if !matches!(&waypoints[waypoints.len() - 1], RouteWaypoint::Station { station_id, .. } if *station_id == destination)
        {
            return Err(RouteError::InvalidDestination);
        }

        // Check for duplicate route
        if self.routes.iter().any(|r| {
            r.origin == origin
                && r.destination == destination
                && r.colony_id == colony_id
                && r.active
        }) {
            return Err(RouteError::RouteExists);
        }

        let route_id = RouteId(self.next_route_id);
        self.next_route_id += 1;

        let route = TrainRoute::new(
            route_id,
            colony_id,
            origin,
            destination,
            waypoints,
            priority,
        );
        self.routes.push(route);

        Ok(route_id)
    }

    /// Get route by ID
    pub fn get_route(&self, route_id: RouteId) -> Option<&TrainRoute> {
        self.routes.iter().find(|r| r.id == route_id)
    }

    /// Get mutable route by ID
    pub fn get_route_mut(&mut self, route_id: RouteId) -> Option<&mut TrainRoute> {
        self.routes.iter_mut().find(|r| r.id == route_id)
    }

    /// Get all active routes for a colony
    pub fn get_routes_for_colony(&self, colony_id: ColonyId) -> Vec<&TrainRoute> {
        self.routes
            .iter()
            .filter(|r| r.colony_id == colony_id && r.active)
            .collect()
    }

    /// Get all routes from origin to destination
    pub fn find_routes(&self, origin: StationId, destination: StationId) -> Vec<&TrainRoute> {
        self.routes
            .iter()
            .filter(|r| r.origin == origin && r.destination == destination && r.active)
            .collect()
    }

    /// Get highest priority route between two stations
    pub fn get_best_route(&self, origin: StationId, destination: StationId) -> Option<&TrainRoute> {
        self.find_routes(origin, destination)
            .into_iter()
            .max_by_key(|r| r.priority)
    }

    /// Deactivate a route
    pub fn deactivate_route(&mut self, route_id: RouteId) -> Result<(), RouteError> {
        if let Some(route) = self.get_route_mut(route_id) {
            route.active = false;
            Ok(())
        } else {
            Err(RouteError::RouteExists)
        }
    }

    /// Get total number of routes
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Get number of active routes
    pub fn active_route_count(&self) -> usize {
        self.routes.iter().filter(|r| r.active).count()
    }
}

impl Default for RouteNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_route() -> TrainRoute {
        let waypoints = vec![
            RouteWaypoint::Station {
                station_id: StationId(1),
                load: true,
                unload: false,
            },
            RouteWaypoint::Segment {
                segment_id: SegmentId(1),
            },
            RouteWaypoint::Segment {
                segment_id: SegmentId(2),
            },
            RouteWaypoint::Station {
                station_id: StationId(2),
                load: false,
                unload: true,
            },
        ];

        TrainRoute::new(
            RouteId(1),
            ColonyId(1),
            StationId(1),
            StationId(2),
            waypoints,
            5,
        )
    }

    #[test]
    fn test_route_creation() {
        let route = create_test_route();
        assert_eq!(route.id, RouteId(1));
        assert_eq!(route.origin, StationId(1));
        assert_eq!(route.destination, StationId(2));
        assert_eq!(route.waypoints.len(), 4);
        assert_eq!(route.total_segments, 2);
        assert!(!route.is_complete());
    }

    #[test]
    fn test_route_waypoint_advancement() {
        let mut route = create_test_route();
        // 4 waypoints total, max_waypoint = 3
        assert_eq!(route.current_waypoint, 0);
        assert_eq!(route.progress_percent(), 0.0); // 0/3 * 100

        route.advance_waypoint();
        assert_eq!(route.current_waypoint, 1);
        // ~33.3% (1/3 * 100)
        assert!(route.progress_percent() > 25.0 && route.progress_percent() < 50.0);

        route.advance_waypoint();
        assert_eq!(route.current_waypoint, 2);
        // ~66.7% (2/3 * 100)
        assert!(route.progress_percent() > 50.0 && route.progress_percent() < 75.0);

        route.advance_waypoint(); // Move to destination
        assert_eq!(route.current_waypoint, 3);
        assert_eq!(route.progress_percent(), 100.0); // 3/3 * 100

        assert!(!route.advance_waypoint()); // Can't advance past end
    }

    #[test]
    fn test_route_remaining_segments() {
        let mut route = create_test_route();
        assert_eq!(route.remaining_segments(), 2);

        route.advance_waypoint(); // Move to first segment
        assert_eq!(route.remaining_segments(), 2);

        route.advance_waypoint(); // Move to second segment
        assert_eq!(route.remaining_segments(), 1);

        route.advance_waypoint(); // Move to destination
        assert_eq!(route.remaining_segments(), 0);
    }

    #[test]
    fn test_route_reset() {
        let mut route = create_test_route();
        route.advance_waypoint();
        route.advance_waypoint();
        assert_eq!(route.current_waypoint, 2);

        route.reset();
        assert_eq!(route.current_waypoint, 0);
    }

    #[test]
    fn test_route_network_create() {
        let mut network = RouteNetwork::new();
        let waypoints = vec![
            RouteWaypoint::Station {
                station_id: StationId(1),
                load: true,
                unload: false,
            },
            RouteWaypoint::Segment {
                segment_id: SegmentId(1),
            },
            RouteWaypoint::Station {
                station_id: StationId(2),
                load: false,
                unload: true,
            },
        ];

        let result = network.create_route(ColonyId(1), StationId(1), StationId(2), waypoints, 5);
        assert!(result.is_ok());
        assert_eq!(network.active_route_count(), 1);
    }

    #[test]
    fn test_route_network_empty_route() {
        let mut network = RouteNetwork::new();
        let result = network.create_route(ColonyId(1), StationId(1), StationId(2), vec![], 5);
        assert_eq!(result, Err(RouteError::EmptyRoute));
    }

    #[test]
    fn test_route_network_same_origin_destination() {
        let mut network = RouteNetwork::new();
        let waypoints = vec![RouteWaypoint::Station {
            station_id: StationId(1),
            load: true,
            unload: true,
        }];
        let result = network.create_route(ColonyId(1), StationId(1), StationId(1), waypoints, 5);
        assert_eq!(result, Err(RouteError::SameOriginDestination));
    }

    #[test]
    fn test_route_network_invalid_origin() {
        let mut network = RouteNetwork::new();
        let waypoints = vec![
            RouteWaypoint::Station {
                station_id: StationId(99), // Wrong station
                load: true,
                unload: false,
            },
            RouteWaypoint::Station {
                station_id: StationId(2),
                load: false,
                unload: true,
            },
        ];
        let result = network.create_route(ColonyId(1), StationId(1), StationId(2), waypoints, 5);
        assert_eq!(result, Err(RouteError::InvalidOrigin));
    }

    #[test]
    fn test_route_network_duplicate_route() {
        let mut network = RouteNetwork::new();
        let waypoints = vec![
            RouteWaypoint::Station {
                station_id: StationId(1),
                load: true,
                unload: false,
            },
            RouteWaypoint::Station {
                station_id: StationId(2),
                load: false,
                unload: true,
            },
        ];

        network
            .create_route(
                ColonyId(1),
                StationId(1),
                StationId(2),
                waypoints.clone(),
                5,
            )
            .unwrap();

        let result = network.create_route(ColonyId(1), StationId(1), StationId(2), waypoints, 5);
        assert_eq!(result, Err(RouteError::RouteExists));
    }

    #[test]
    fn test_route_network_get_routes() {
        let mut network = RouteNetwork::new();
        let waypoints1 = vec![
            RouteWaypoint::Station {
                station_id: StationId(1),
                load: true,
                unload: false,
            },
            RouteWaypoint::Station {
                station_id: StationId(2),
                load: false,
                unload: true,
            },
        ];
        let waypoints2 = vec![
            RouteWaypoint::Station {
                station_id: StationId(1),
                load: true,
                unload: false,
            },
            RouteWaypoint::Station {
                station_id: StationId(3),
                load: false,
                unload: true,
            },
        ];

        network
            .create_route(ColonyId(1), StationId(1), StationId(2), waypoints1, 5)
            .unwrap();
        network
            .create_route(ColonyId(1), StationId(1), StationId(3), waypoints2, 7)
            .unwrap();

        let routes = network.find_routes(StationId(1), StationId(2));
        assert_eq!(routes.len(), 1);

        let best = network.get_best_route(StationId(1), StationId(3));
        assert!(best.is_some());
        assert_eq!(best.unwrap().priority, 7);
    }

    #[test]
    fn test_route_network_deactivate() {
        let mut network = RouteNetwork::new();
        let waypoints = vec![
            RouteWaypoint::Station {
                station_id: StationId(1),
                load: true,
                unload: false,
            },
            RouteWaypoint::Station {
                station_id: StationId(2),
                load: false,
                unload: true,
            },
        ];

        let route_id = network
            .create_route(ColonyId(1), StationId(1), StationId(2), waypoints, 5)
            .unwrap();

        assert_eq!(network.active_route_count(), 1);
        network.deactivate_route(route_id).unwrap();
        assert_eq!(network.active_route_count(), 0);
    }

    #[test]
    fn test_route_network_progress() {
        let mut route = create_test_route();
        assert_eq!(route.progress_percent(), 0.0);

        // Station -> Segment1 -> Segment2 -> Station (4 waypoints)
        route.advance_waypoint(); // Now on Segment1 (waypoint 1/3)
        assert!(route.progress_percent() > 0.0);
        assert!(route.progress_percent() < 100.0);

        route.advance_waypoint(); // Now on Segment2 (waypoint 2/3)
        assert!(route.progress_percent() > 0.0);
        assert!(route.progress_percent() < 100.0);

        route.advance_waypoint(); // Now on destination Station (waypoint 3/3)
        assert_eq!(route.progress_percent(), 100.0);
    }
}
