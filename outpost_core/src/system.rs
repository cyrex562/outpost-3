//! System zoom — world specialization, inter-body logistics, and megaprojects.
//!
//! Covers DESIGN.md §8.3: the system scope's distinct responsibilities:
//! * World-scale specialization: which body plays which system role.
//! * Inter-body logistics: cargo ships with hauler capacity as a managed resource.
//! * Megaprojects: multi-milestone construction pooled from the entire system.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ─── Body & Node Map ─────────────────────────────────────────────────────────

/// Stable identifier for a celestial body (planet, moon, belt, station).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BodyId(pub Uuid);

impl BodyId {
    /// Create a new random [`BodyId`].
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BodyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BodyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Category of a celestial body in the system node map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodyKind {
    /// Rocky inner planet.
    InnerPlanet,
    /// Gas or ice giant.
    GasGiant,
    /// Moon orbiting a planet or giant.
    Moon,
    /// Asteroid belt aggregate node.
    AsteroidBelt,
    /// Orbital station (not a natural body).
    OrbitalStation,
}

/// Assigned system-role for a celestial body.
///
/// The player assigns a role to guide automated resource flow and bonuses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemRole {
    /// Heavy industry, manufacturing, and processing.
    Industry,
    /// Raw resource extraction (mining, harvesting).
    RawExtraction,
    /// Scientific research and development.
    Science,
    /// Volatile and fuel production (e.g. gas giant skimmers).
    FuelProduction,
    /// Population hub and administrative centre.
    PopulationHub,
    /// Unassigned; no specialization bonus applied.
    Unassigned,
}

/// A celestial body in the system node map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body {
    /// Stable identifier.
    pub id: BodyId,
    /// Display name.
    pub name: String,
    /// Kind of body.
    pub kind: BodyKind,
    /// Current system-role assignment.
    pub role: SystemRole,
    /// Distance from the system primary in arbitrary AU units (used for travel time).
    pub distance_au: f32,
}

impl Body {
    /// Create a new unassigned body.
    #[must_use]
    pub fn new(name: impl Into<String>, kind: BodyKind, distance_au: f32) -> Self {
        Self {
            id: BodyId::new(),
            name: name.into(),
            kind,
            role: SystemRole::Unassigned,
            distance_au,
        }
    }
}

/// A directed edge in the system node map representing a shipping route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingRoute {
    /// Stable identifier.
    pub id: Uuid,
    /// Origin body.
    pub from: BodyId,
    /// Destination body.
    pub to: BodyId,
    /// Travel time in strategic months (derived from distance + propulsion tech).
    pub travel_time_months: u32,
}

/// System node map: bodies as nodes, shipping routes as edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemNodeMap {
    /// All celestial bodies in the system.
    pub bodies: HashMap<BodyId, Body>,
    /// Directed shipping routes between bodies.
    pub routes: HashMap<Uuid, ShippingRoute>,
    /// Propulsion technology level (1 = basic, higher = faster travel).
    pub propulsion_level: u32,
}

impl Default for SystemNodeMap {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemNodeMap {
    /// Create an empty system node map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bodies: HashMap::new(),
            routes: HashMap::new(),
            propulsion_level: 1,
        }
    }

    /// Add a body to the map and return its id.
    pub fn add_body(&mut self, body: Body) -> BodyId {
        let id = body.id.clone();
        self.bodies.insert(id.clone(), body);
        id
    }

    /// Compute travel time in strategic months between two bodies based on distance and propulsion.
    ///
    /// `travel_time = ceil(distance_au / propulsion_level)`, minimum 1.
    #[must_use]
    pub fn compute_travel_time(&self, from: &BodyId, to: &BodyId) -> Option<u32> {
        let a = self.bodies.get(from)?;
        let b = self.bodies.get(to)?;
        let dist = (a.distance_au - b.distance_au).abs();
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let months = (dist / self.propulsion_level as f32).ceil() as u32;
        Some(months.max(1))
    }

    /// Add a shipping route between two bodies; computes travel time automatically.
    ///
    /// Returns `None` if either body does not exist.
    pub fn add_route(&mut self, from: BodyId, to: BodyId) -> Option<Uuid> {
        let travel_time = self.compute_travel_time(&from, &to)?;
        let id = Uuid::new_v4();
        self.routes.insert(
            id,
            ShippingRoute {
                id,
                from,
                to,
                travel_time_months: travel_time,
            },
        );
        Some(id)
    }
}

// ─── Hauler Fleet & Capacity ──────────────────────────────────────────────────

/// A cargo ship (hauler) available in the system fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hauler {
    /// Stable identifier.
    pub id: Uuid,
    /// Maximum cargo units this ship can carry per trip.
    pub capacity: f64,
    /// Whether the hauler is currently busy on a shipping mission.
    pub in_transit: bool,
}

/// System-wide hauler fleet: managed resource for inter-body logistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HaulerFleet {
    /// All haulers in the system.
    pub haulers: HashMap<Uuid, Hauler>,
}

impl HaulerFleet {
    /// Create an empty fleet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            haulers: HashMap::new(),
        }
    }

    /// Register a new hauler with the given capacity and return its id.
    pub fn add_hauler(&mut self, capacity: f64) -> Uuid {
        let id = Uuid::new_v4();
        self.haulers.insert(
            id,
            Hauler {
                id,
                capacity,
                in_transit: false,
            },
        );
        id
    }

    /// Total capacity available (free haulers only).
    #[must_use]
    pub fn available_capacity(&self) -> f64 {
        self.haulers
            .values()
            .filter(|h| !h.in_transit)
            .map(|h| h.capacity)
            .sum()
    }

    /// Total fleet capacity (all haulers).
    #[must_use]
    pub fn total_capacity(&self) -> f64 {
        self.haulers.values().map(|h| h.capacity).sum()
    }
}

// ─── Cargo Shipment ───────────────────────────────────────────────────────────

/// A in-transit cargo shipment between two bodies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoShipment {
    /// Stable identifier.
    pub id: Uuid,
    /// Origin body.
    pub from: BodyId,
    /// Destination body.
    pub to: BodyId,
    /// Commodity and quantity being shipped.
    pub cargo: Vec<(String, f64)>,
    /// Hauler assigned to this shipment.
    pub hauler_id: Uuid,
    /// Strategic months remaining until arrival.
    pub turns_remaining: u32,
    /// Colony that should receive this cargo on arrival (if any).
    pub destination_colony: Option<crate::colony::ColonyId>,
}

// ─── Megaproject Framework ────────────────────────────────────────────────────

/// Stable identifier for a megaproject.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MegaprojectId(pub Uuid);

impl MegaprojectId {
    /// Create a new random [`MegaprojectId`].
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MegaprojectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MegaprojectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single milestone within a megaproject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MegaprojectMilestone {
    /// Milestone index (0-based, must be completed in order).
    pub index: u32,
    /// Human-readable label for this milestone.
    pub label: String,
    /// Commodity costs required to complete this milestone.
    pub resource_cost: Vec<(String, f64)>,
    /// Research points required to unlock this milestone.
    pub research_cost: f32,
    /// Resources already contributed toward this milestone.
    pub contributed: Vec<(String, f64)>,
    /// Research already contributed.
    pub research_contributed: f32,
    /// Whether this milestone has been completed.
    pub completed: bool,
}

impl MegaprojectMilestone {
    /// Returns true when all resource and research requirements have been met.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        if self.completed {
            return true;
        }
        // Check research
        if self.research_contributed < self.research_cost {
            return false;
        }
        // Check each commodity requirement
        for (commodity, required) in &self.resource_cost {
            let contributed: f64 = self
                .contributed
                .iter()
                .filter(|(c, _)| c == commodity)
                .map(|(_, q)| *q)
                .sum();
            if contributed < *required {
                return false;
            }
        }
        true
    }
}

/// Category of a megaproject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MegaprojectKind {
    /// The capstone victory condition: launch an interstellar expedition.
    InterstellarExpedition,
    /// Build a wormhole gate connecting to another system.
    WormholeGate,
    /// Deploy a terraforming engine on a body.
    TerraformingEngine,
    /// Construct a system-scale power array.
    SystemPowerArray,
    /// A custom player-defined megaproject.
    Custom(String),
}

/// A megaproject: a multi-milestone construction pooled from the entire system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Megaproject {
    /// Stable identifier.
    pub id: MegaprojectId,
    /// Display name.
    pub name: String,
    /// Category of megaproject.
    pub kind: MegaprojectKind,
    /// Ordered list of milestones (must be completed in sequence).
    pub milestones: Vec<MegaprojectMilestone>,
    /// Whether the entire project has been completed (all milestones done).
    pub completed: bool,
}

impl Megaproject {
    /// Index of the next incomplete milestone, or `None` if all are complete.
    #[must_use]
    pub fn next_milestone_index(&self) -> Option<usize> {
        self.milestones.iter().position(|m| !m.completed)
    }

    /// Number of completed milestones.
    #[must_use]
    pub fn completed_milestones(&self) -> usize {
        self.milestones.iter().filter(|m| m.completed).count()
    }

    /// Returns true when all milestones have been completed.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.milestones.iter().all(|m| m.completed)
    }
}

// ─── Transport Capacity ───────────────────────────────────────────────────────

/// Passenger-transport capacity for migration batches (§6A / §8.3).
///
/// `haulers * colonists_per_hauler` gives the maximum colonists that can move
/// on a single route in one strategic month.  Excess demand is deferred to the
/// next batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportCapacity {
    /// Number of passenger haulers in the migration fleet.
    pub haulers: u32,
    /// Maximum colonists each hauler can carry per strategic month.
    pub colonists_per_hauler: u32,
}

impl TransportCapacity {
    /// Total colonist slots available per route per strategic month.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.haulers.saturating_mul(self.colonists_per_hauler)
    }
}

impl Default for TransportCapacity {
    fn default() -> Self {
        // Default: 2 haulers × 50 colonists each = 100 per route per month.
        Self {
            haulers: 2,
            colonists_per_hauler: 50,
        }
    }
}

// ─── System State ─────────────────────────────────────────────────────────────

/// Top-level state container for the system zoom layer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemState {
    /// Node map of bodies and shipping routes.
    pub node_map: SystemNodeMap,
    /// Fleet of cargo haulers (managed resource).
    pub hauler_fleet: HaulerFleet,
    /// Active and completed cargo shipments.
    pub shipments: HashMap<Uuid, CargoShipment>,
    /// All megaprojects (active and completed).
    pub megaprojects: HashMap<MegaprojectId, Megaproject>,
    /// Passenger-transport capacity for migration batches.
    pub transport_capacity: TransportCapacity,
}

impl SystemState {
    /// Create an empty system state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ─── System Commands ──────────────────────────────────────────────────────────

/// Commands that mutate the system zoom layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemCommand {
    /// Register a new celestial body in the system node map.
    AddBody {
        /// Display name.
        name: String,
        /// Kind of body.
        kind: BodyKind,
        /// Distance from system primary in AU.
        distance_au: f32,
    },
    /// Assign a system-role to a body.
    AssignRole {
        /// Target body.
        body_id: BodyId,
        /// New role to assign.
        role: SystemRole,
    },
    /// Add a shipping route between two bodies (travel time auto-computed).
    AddShippingRoute {
        /// Origin body.
        from: BodyId,
        /// Destination body.
        to: BodyId,
    },
    /// Add a hauler to the system fleet.
    AddHauler {
        /// Maximum cargo units this hauler can carry.
        capacity: f64,
    },
    /// Dispatch a cargo shipment between two bodies.
    ///
    /// Fails if insufficient free hauler capacity is available.
    DispatchShipment {
        /// Origin body.
        from: BodyId,
        /// Destination body.
        to: BodyId,
        /// Commodity and quantity being shipped.
        cargo: Vec<(String, f64)>,
        /// Colony that should receive the cargo on arrival (optional).
        destination_colony: Option<crate::colony::ColonyId>,
    },
    /// Advance all in-transit shipments by one strategic month.
    ///
    /// Shipments that reach zero turns remaining are marked for arrival.
    AdvanceShipments,
    /// Register a new megaproject.
    RegisterMegaproject {
        /// Display name.
        name: String,
        /// Category of megaproject.
        kind: MegaprojectKind,
        /// Ordered milestone specifications.
        milestones: Vec<MilestoneSpec>,
    },
    /// Contribute resources to the current active milestone of a megaproject.
    ContributeToMegaproject {
        /// Target megaproject.
        project_id: MegaprojectId,
        /// Commodity contributions.
        resources: Vec<(String, f64)>,
        /// Research point contribution.
        research: f32,
    },
    /// Upgrade the propulsion technology level, reducing travel time across all routes.
    UpgradePropulsion {
        /// New propulsion level (must be higher than current).
        new_level: u32,
    },
}

/// Specification for a milestone when registering a megaproject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneSpec {
    /// Human-readable label.
    pub label: String,
    /// Commodity costs.
    pub resource_cost: Vec<(String, f64)>,
    /// Research points required.
    pub research_cost: f32,
}

// ─── System Events ────────────────────────────────────────────────────────────

/// Events produced by the system zoom layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    /// A new body was added to the system node map.
    BodyAdded {
        /// Assigned stable identifier.
        body_id: BodyId,
        /// Display name.
        name: String,
        /// Kind of body.
        kind: BodyKind,
        /// Distance from primary in AU.
        distance_au: f32,
    },
    /// A system-role was assigned to a body.
    RoleAssigned {
        /// Target body.
        body_id: BodyId,
        /// Previous role (for undo/display).
        previous_role: SystemRole,
        /// New role.
        new_role: SystemRole,
    },
    /// A shipping route was added between two bodies.
    ShippingRouteAdded {
        /// Route identifier.
        route_id: Uuid,
        /// Origin body.
        from: BodyId,
        /// Destination body.
        to: BodyId,
        /// Computed travel time in strategic months.
        travel_time_months: u32,
    },
    /// A hauler was added to the fleet.
    HaulerAdded {
        /// Assigned hauler identifier.
        hauler_id: Uuid,
        /// Cargo capacity.
        capacity: f64,
    },
    /// A cargo shipment was dispatched.
    ShipmentDispatched {
        /// Shipment identifier.
        shipment_id: Uuid,
        /// Origin body.
        from: BodyId,
        /// Destination body.
        to: BodyId,
        /// Cargo manifest.
        cargo: Vec<(String, f64)>,
        /// Hauler assigned to this mission.
        hauler_id: Uuid,
        /// Travel time in strategic months.
        travel_time_months: u32,
    },
    /// A cargo shipment arrived at its destination.
    ShipmentArrived {
        /// Shipment identifier.
        shipment_id: Uuid,
        /// Destination body.
        to: BodyId,
        /// Delivered cargo.
        cargo: Vec<(String, f64)>,
        /// Hauler that completed the mission (now free again).
        hauler_id: Uuid,
    },
    /// A megaproject was registered.
    MegaprojectRegistered {
        /// Assigned stable identifier.
        project_id: MegaprojectId,
        /// Display name.
        name: String,
        /// Category.
        kind: MegaprojectKind,
        /// Number of milestones.
        milestone_count: usize,
    },
    /// Resources were contributed to a megaproject milestone.
    MegaprojectContribution {
        /// Target megaproject.
        project_id: MegaprojectId,
        /// Milestone index that received the contribution.
        milestone_index: u32,
        /// Resources contributed.
        resources: Vec<(String, f64)>,
        /// Research contributed.
        research: f32,
    },
    /// A megaproject milestone was completed.
    MilestoneCompleted {
        /// Target megaproject.
        project_id: MegaprojectId,
        /// Index of the milestone that completed.
        milestone_index: u32,
        /// Label of the completed milestone.
        label: String,
    },
    /// A megaproject was fully completed (all milestones done).
    MegaprojectCompleted {
        /// The completed megaproject.
        project_id: MegaprojectId,
        /// Display name.
        name: String,
        /// Kind.
        kind: MegaprojectKind,
    },
    /// Propulsion technology was upgraded.
    PropulsionUpgraded {
        /// New propulsion level.
        new_level: u32,
    },
}

// ─── System Errors ────────────────────────────────────────────────────────────

/// Errors from the system zoom layer.
#[derive(Debug, Error)]
pub enum SystemError {
    /// The referenced body does not exist.
    #[error("body not found: {0}")]
    BodyNotFound(BodyId),
    /// The referenced megaproject does not exist.
    #[error("megaproject not found: {0}")]
    MegaprojectNotFound(MegaprojectId),
    /// Not enough free hauler capacity to dispatch the shipment.
    #[error("insufficient hauler capacity: need {needed:.1} but only {available:.1} free")]
    InsufficientHaulerCapacity {
        /// Cargo units required.
        needed: f64,
        /// Free capacity available.
        available: f64,
    },
    /// No shipping route exists between the two bodies.
    #[error("no shipping route between {from} and {to}")]
    NoRouteFound {
        /// Origin.
        from: BodyId,
        /// Destination.
        to: BodyId,
    },
    /// The megaproject has already been completed.
    #[error("megaproject already completed: {0}")]
    MegaprojectAlreadyComplete(MegaprojectId),
    /// All milestones are complete (should not normally be reached via normal flow).
    #[error("no active milestone in megaproject {0}")]
    NoActiveMilestone(MegaprojectId),
    /// The new propulsion level is not higher than the current level.
    #[error("propulsion upgrade must be higher than current level {current}")]
    PropulsionDowngrade {
        /// Current level.
        current: u32,
    },
    /// A command argument is out of range or otherwise invalid.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

// ─── System Command Processor ─────────────────────────────────────────────────

/// Apply a [`SystemCommand`] to [`SystemState`] and return resulting [`SystemEvent`]s.
///
/// # Errors
///
/// Returns [`SystemError`] when the command cannot be applied.
///
/// # Panics
///
/// Panics if capacity-check logic is inconsistent and no hauler is assigned despite availability.
#[allow(clippy::too_many_lines)]
pub fn apply_system_command(
    state: &mut SystemState,
    cmd: &SystemCommand,
) -> Result<Vec<SystemEvent>, SystemError> {
    match cmd {
        SystemCommand::AddBody {
            name,
            kind,
            distance_au,
        } => {
            if *distance_au < 0.0 {
                return Err(SystemError::InvalidArgument(
                    "distance_au must be non-negative".into(),
                ));
            }
            let body = Body::new(name.clone(), kind.clone(), *distance_au);
            let body_id = body.id.clone();
            state.node_map.add_body(body);
            Ok(vec![SystemEvent::BodyAdded {
                body_id,
                name: name.clone(),
                kind: kind.clone(),
                distance_au: *distance_au,
            }])
        }

        SystemCommand::AssignRole { body_id, role } => {
            let body = state
                .node_map
                .bodies
                .get_mut(body_id)
                .ok_or_else(|| SystemError::BodyNotFound(body_id.clone()))?;
            let previous_role = body.role.clone();
            body.role = role.clone();
            Ok(vec![SystemEvent::RoleAssigned {
                body_id: body_id.clone(),
                previous_role,
                new_role: role.clone(),
            }])
        }

        SystemCommand::AddShippingRoute { from, to } => {
            // Verify both bodies exist and compute travel time before mutating
            let travel_time = state
                .node_map
                .compute_travel_time(from, to)
                .ok_or_else(|| SystemError::NoRouteFound {
                    from: from.clone(),
                    to: to.clone(),
                })?;
            let route_id = Uuid::new_v4();
            state.node_map.routes.insert(
                route_id,
                ShippingRoute {
                    id: route_id,
                    from: from.clone(),
                    to: to.clone(),
                    travel_time_months: travel_time,
                },
            );
            Ok(vec![SystemEvent::ShippingRouteAdded {
                route_id,
                from: from.clone(),
                to: to.clone(),
                travel_time_months: travel_time,
            }])
        }

        SystemCommand::AddHauler { capacity } => {
            if *capacity <= 0.0 {
                return Err(SystemError::InvalidArgument(
                    "hauler capacity must be positive".into(),
                ));
            }
            let hauler_id = state.hauler_fleet.add_hauler(*capacity);
            Ok(vec![SystemEvent::HaulerAdded {
                hauler_id,
                capacity: *capacity,
            }])
        }

        SystemCommand::DispatchShipment {
            from,
            to,
            cargo,
            destination_colony,
        } => {
            // Verify bodies exist
            if !state.node_map.bodies.contains_key(from) {
                return Err(SystemError::BodyNotFound(from.clone()));
            }
            if !state.node_map.bodies.contains_key(to) {
                return Err(SystemError::BodyNotFound(to.clone()));
            }
            // Compute total cargo size
            let total: f64 = cargo.iter().map(|(_, q)| *q).sum();
            if total <= 0.0 {
                return Err(SystemError::InvalidArgument(
                    "cargo total must be positive".into(),
                ));
            }
            // Check available hauler capacity
            let available = state.hauler_fleet.available_capacity();
            if available < total {
                return Err(SystemError::InsufficientHaulerCapacity {
                    needed: total,
                    available,
                });
            }
            // Find a route between the bodies
            let travel_time = state
                .node_map
                .compute_travel_time(from, to)
                .ok_or_else(|| SystemError::NoRouteFound {
                    from: from.clone(),
                    to: to.clone(),
                })?;
            // Assign free haulers greedily until cargo is covered
            let mut remaining = total;
            let mut assigned_hauler_id = None;
            for hauler in state.hauler_fleet.haulers.values_mut() {
                if hauler.in_transit || remaining <= 0.0 {
                    continue;
                }
                let take = remaining.min(hauler.capacity);
                remaining -= take;
                hauler.in_transit = true;
                assigned_hauler_id = Some(hauler.id);
                if remaining <= 0.0 {
                    break;
                }
            }
            let hauler_id = assigned_hauler_id.expect("capacity check guarantees a hauler");
            let shipment_id = Uuid::new_v4();
            state.shipments.insert(
                shipment_id,
                CargoShipment {
                    id: shipment_id,
                    from: from.clone(),
                    to: to.clone(),
                    cargo: cargo.clone(),
                    hauler_id,
                    turns_remaining: travel_time,
                    destination_colony: *destination_colony,
                },
            );
            Ok(vec![SystemEvent::ShipmentDispatched {
                shipment_id,
                from: from.clone(),
                to: to.clone(),
                cargo: cargo.clone(),
                hauler_id,
                travel_time_months: travel_time,
            }])
        }

        SystemCommand::AdvanceShipments => {
            // Pass 1: decrement all turns_remaining; collect IDs that reach zero.
            let mut arrived_ids: Vec<Uuid> = Vec::new();
            for (id, shipment) in &mut state.shipments {
                if shipment.turns_remaining > 0 {
                    shipment.turns_remaining -= 1;
                }
                if shipment.turns_remaining == 0 {
                    arrived_ids.push(*id);
                }
            }
            // Pass 2: remove arrived shipments and emit events.
            let mut events = Vec::new();
            for id in arrived_ids {
                let arrived = state.shipments.remove(&id).unwrap();
                if let Some(hauler) = state.hauler_fleet.haulers.get_mut(&arrived.hauler_id) {
                    hauler.in_transit = false;
                }
                events.push(SystemEvent::ShipmentArrived {
                    shipment_id: arrived.id,
                    to: arrived.to.clone(),
                    cargo: arrived.cargo.clone(),
                    hauler_id: arrived.hauler_id,
                });
            }
            Ok(events)
        }

        SystemCommand::RegisterMegaproject {
            name,
            kind,
            milestones,
        } => {
            if milestones.is_empty() {
                return Err(SystemError::InvalidArgument(
                    "megaproject must have at least one milestone".into(),
                ));
            }
            let project_id = MegaprojectId::new();
            let milestone_count = milestones.len();
            let built_milestones: Vec<MegaprojectMilestone> = milestones
                .iter()
                .enumerate()
                .map(|(i, spec)| MegaprojectMilestone {
                    #[allow(clippy::cast_possible_truncation)]
                    index: i as u32,
                    label: spec.label.clone(),
                    resource_cost: spec.resource_cost.clone(),
                    research_cost: spec.research_cost,
                    contributed: Vec::new(),
                    research_contributed: 0.0,
                    completed: false,
                })
                .collect();
            state.megaprojects.insert(
                project_id.clone(),
                Megaproject {
                    id: project_id.clone(),
                    name: name.clone(),
                    kind: kind.clone(),
                    milestones: built_milestones,
                    completed: false,
                },
            );
            Ok(vec![SystemEvent::MegaprojectRegistered {
                project_id,
                name: name.clone(),
                kind: kind.clone(),
                milestone_count,
            }])
        }

        SystemCommand::ContributeToMegaproject {
            project_id,
            resources,
            research,
        } => {
            let project = state
                .megaprojects
                .get_mut(project_id)
                .ok_or_else(|| SystemError::MegaprojectNotFound(project_id.clone()))?;
            if project.completed {
                return Err(SystemError::MegaprojectAlreadyComplete(project_id.clone()));
            }
            let milestone_idx = project
                .next_milestone_index()
                .ok_or_else(|| SystemError::NoActiveMilestone(project_id.clone()))?;

            let milestone = &mut project.milestones[milestone_idx];
            // Apply contribution
            milestone.research_contributed += research;
            for (commodity, qty) in resources {
                if let Some(entry) = milestone
                    .contributed
                    .iter_mut()
                    .find(|(c, _)| c == commodity)
                {
                    entry.1 += qty;
                } else {
                    milestone.contributed.push((commodity.clone(), *qty));
                }
            }

            #[allow(clippy::cast_possible_truncation)]
            let milestone_idx_u32 = milestone_idx as u32;
            let mut events = vec![SystemEvent::MegaprojectContribution {
                project_id: project_id.clone(),
                milestone_index: milestone_idx_u32,
                resources: resources.clone(),
                research: *research,
            }];

            // Check if milestone is now complete
            if milestone.is_complete() {
                milestone.completed = true;
                let label = milestone.label.clone();
                events.push(SystemEvent::MilestoneCompleted {
                    project_id: project_id.clone(),
                    milestone_index: milestone_idx_u32,
                    label,
                });
                // Check if entire project is now complete
                if project.is_complete() {
                    project.completed = true;
                    events.push(SystemEvent::MegaprojectCompleted {
                        project_id: project_id.clone(),
                        name: project.name.clone(),
                        kind: project.kind.clone(),
                    });
                }
            }

            Ok(events)
        }

        SystemCommand::UpgradePropulsion { new_level } => {
            let current = state.node_map.propulsion_level;
            if *new_level <= current {
                return Err(SystemError::PropulsionDowngrade { current });
            }
            state.node_map.propulsion_level = *new_level;
            Ok(vec![SystemEvent::PropulsionUpgraded {
                new_level: *new_level,
            }])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state_with_two_bodies() -> (SystemState, BodyId, BodyId) {
        let mut state = SystemState::new();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::AddBody {
                name: "Inner World".into(),
                kind: BodyKind::InnerPlanet,
                distance_au: 1.0,
            },
        )
        .unwrap();
        let inner_id = match &events[0] {
            SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
            _ => panic!("expected BodyAdded"),
        };
        let events = apply_system_command(
            &mut state,
            &SystemCommand::AddBody {
                name: "Outer Belt".into(),
                kind: BodyKind::AsteroidBelt,
                distance_au: 3.5,
            },
        )
        .unwrap();
        let outer_id = match &events[0] {
            SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
            _ => panic!("expected BodyAdded"),
        };
        (state, inner_id, outer_id)
    }

    #[test]
    fn add_body_registers_in_map() {
        let mut state = SystemState::new();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::AddBody {
                name: "New Terra".into(),
                kind: BodyKind::InnerPlanet,
                distance_au: 1.0,
            },
        )
        .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SystemEvent::BodyAdded { .. }));
        assert_eq!(state.node_map.bodies.len(), 1);
    }

    #[test]
    fn assign_role_changes_body_role() {
        let (mut state, inner_id, _) = make_state_with_two_bodies();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::AssignRole {
                body_id: inner_id.clone(),
                role: SystemRole::Industry,
            },
        )
        .unwrap();
        assert!(matches!(
            events[0],
            SystemEvent::RoleAssigned {
                new_role: SystemRole::Industry,
                ..
            }
        ));
        assert_eq!(state.node_map.bodies[&inner_id].role, SystemRole::Industry);
    }

    #[test]
    fn assign_role_to_nonexistent_body_fails() {
        let mut state = SystemState::new();
        let bad_id = BodyId::new();
        let result = apply_system_command(
            &mut state,
            &SystemCommand::AssignRole {
                body_id: bad_id,
                role: SystemRole::Science,
            },
        );
        assert!(matches!(result, Err(SystemError::BodyNotFound(_))));
    }

    #[test]
    fn shipping_capacity_constraint_enforced() {
        let (mut state, inner_id, outer_id) = make_state_with_two_bodies();
        // Add a small hauler (capacity 10)
        apply_system_command(&mut state, &SystemCommand::AddHauler { capacity: 10.0 }).unwrap();
        // Try to ship 50 units — should fail
        let result = apply_system_command(
            &mut state,
            &SystemCommand::DispatchShipment {
                from: inner_id,
                to: outer_id,
                cargo: vec![("iron".into(), 50.0)],
                destination_colony: None,
            },
        );
        assert!(
            matches!(result, Err(SystemError::InsufficientHaulerCapacity { .. })),
            "expected InsufficientHaulerCapacity, got {:?}",
            result
        );
    }

    #[test]
    fn dispatch_shipment_with_sufficient_capacity() {
        let (mut state, inner_id, outer_id) = make_state_with_two_bodies();
        apply_system_command(&mut state, &SystemCommand::AddHauler { capacity: 100.0 }).unwrap();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::DispatchShipment {
                from: inner_id.clone(),
                to: outer_id.clone(),
                cargo: vec![("food".into(), 50.0)],
                destination_colony: None,
            },
        )
        .unwrap();
        assert!(matches!(events[0], SystemEvent::ShipmentDispatched { .. }));
        assert_eq!(state.shipments.len(), 1);
        // Hauler should be marked in-transit; available capacity should drop
        assert_eq!(state.hauler_fleet.available_capacity(), 0.0);
    }

    #[test]
    fn shipment_tracking_and_hauler_capacity() {
        // Use bodies at distance 1.0 AU apart → travel_time = ceil(1.0/1) = 1 month.
        let mut state = SystemState::new();
        let e1 = apply_system_command(
            &mut state,
            &SystemCommand::AddBody {
                name: "A".into(),
                kind: BodyKind::InnerPlanet,
                distance_au: 0.0,
            },
        )
        .unwrap();
        let e2 = apply_system_command(
            &mut state,
            &SystemCommand::AddBody {
                name: "B".into(),
                kind: BodyKind::InnerPlanet,
                distance_au: 1.0,
            },
        )
        .unwrap();
        let body_a = match &e1[0] {
            SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
            _ => panic!(),
        };
        let body_b = match &e2[0] {
            SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
            _ => panic!(),
        };
        // Add a hauler and dispatch a shipment
        apply_system_command(&mut state, &SystemCommand::AddHauler { capacity: 100.0 }).unwrap();
        apply_system_command(
            &mut state,
            &SystemCommand::DispatchShipment {
                from: body_a.clone(),
                to: body_b.clone(),
                cargo: vec![("iron".into(), 50.0)],
                destination_colony: None,
            },
        )
        .unwrap();
        // Hauler is now busy
        assert_eq!(state.hauler_fleet.available_capacity(), 0.0);
        assert_eq!(state.shipments.len(), 1);
        // Verify travel time is 1 for bodies 1 AU apart with propulsion_level=1
        let travel_time = state
            .node_map
            .compute_travel_time(&body_a, &body_b)
            .unwrap();
        assert_eq!(travel_time, 1, "expected travel time of 1 month");
        // After 1 advance the shipment (travel_time=1) should arrive
        let arrival_events =
            apply_system_command(&mut state, &SystemCommand::AdvanceShipments).unwrap();
        assert_eq!(
            state.shipments.len(),
            0,
            "shipment should be removed after arriving"
        );
        assert!(
            arrival_events
                .iter()
                .any(|e| matches!(e, SystemEvent::ShipmentArrived { .. })),
            "shipment should arrive after 1 month; events: {arrival_events:?}"
        );
        // Shipment is removed and hauler freed
        assert_eq!(state.shipments.len(), 0);
        assert_eq!(state.hauler_fleet.available_capacity(), 100.0);
    }

    #[test]
    fn megaproject_milestone_progression() {
        let mut state = SystemState::new();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::RegisterMegaproject {
                name: "Interstellar Expedition".into(),
                kind: MegaprojectKind::InterstellarExpedition,
                milestones: vec![
                    MilestoneSpec {
                        label: "Hull Construction".into(),
                        resource_cost: vec![("steel".into(), 500.0)],
                        research_cost: 100.0,
                    },
                    MilestoneSpec {
                        label: "Drive Assembly".into(),
                        resource_cost: vec![("fuel".into(), 1000.0)],
                        research_cost: 200.0,
                    },
                ],
            },
        )
        .unwrap();

        let project_id = match &events[0] {
            SystemEvent::MegaprojectRegistered { project_id, .. } => project_id.clone(),
            _ => panic!("expected MegaprojectRegistered"),
        };

        // Contribute enough to complete milestone 0
        let events = apply_system_command(
            &mut state,
            &SystemCommand::ContributeToMegaproject {
                project_id: project_id.clone(),
                resources: vec![("steel".into(), 500.0)],
                research: 100.0,
            },
        )
        .unwrap();
        // Should get Contribution + MilestoneCompleted (but not MegaprojectCompleted yet)
        assert!(events.iter().any(|e| matches!(
            e,
            SystemEvent::MilestoneCompleted {
                milestone_index: 0,
                ..
            }
        )));
        assert!(!events
            .iter()
            .any(|e| matches!(e, SystemEvent::MegaprojectCompleted { .. })));

        // Milestone 1 is now active
        let project = &state.megaprojects[&project_id];
        assert_eq!(project.next_milestone_index(), Some(1));

        // Contribute enough to complete milestone 1
        let events = apply_system_command(
            &mut state,
            &SystemCommand::ContributeToMegaproject {
                project_id: project_id.clone(),
                resources: vec![("fuel".into(), 1000.0)],
                research: 200.0,
            },
        )
        .unwrap();
        // Now both MilestoneCompleted and MegaprojectCompleted should fire
        assert!(events.iter().any(|e| matches!(
            e,
            SystemEvent::MilestoneCompleted {
                milestone_index: 1,
                ..
            }
        )));
        assert!(events
            .iter()
            .any(|e| matches!(e, SystemEvent::MegaprojectCompleted { .. })));
        assert!(state.megaprojects[&project_id].completed);
    }

    #[test]
    fn contribute_to_completed_megaproject_fails() {
        let mut state = SystemState::new();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::RegisterMegaproject {
                name: "Quick Build".into(),
                kind: MegaprojectKind::SystemPowerArray,
                milestones: vec![MilestoneSpec {
                    label: "Phase 1".into(),
                    resource_cost: vec![],
                    research_cost: 0.0,
                }],
            },
        )
        .unwrap();
        let project_id = match &events[0] {
            SystemEvent::MegaprojectRegistered { project_id, .. } => project_id.clone(),
            _ => panic!(),
        };
        // Complete it
        apply_system_command(
            &mut state,
            &SystemCommand::ContributeToMegaproject {
                project_id: project_id.clone(),
                resources: vec![],
                research: 0.0,
            },
        )
        .unwrap();
        // Try again — should fail
        let result = apply_system_command(
            &mut state,
            &SystemCommand::ContributeToMegaproject {
                project_id: project_id.clone(),
                resources: vec![],
                research: 0.0,
            },
        );
        assert!(matches!(
            result,
            Err(SystemError::MegaprojectAlreadyComplete(_))
        ));
    }

    #[test]
    fn propulsion_upgrade_reduces_travel_time() {
        let (mut state, inner_id, outer_id) = make_state_with_two_bodies();
        let t1 = state
            .node_map
            .compute_travel_time(&inner_id, &outer_id)
            .unwrap();
        apply_system_command(
            &mut state,
            &SystemCommand::UpgradePropulsion { new_level: 5 },
        )
        .unwrap();
        let t2 = state
            .node_map
            .compute_travel_time(&inner_id, &outer_id)
            .unwrap();
        assert!(
            t2 <= t1,
            "upgraded propulsion should reduce or equal travel time"
        );
    }

    #[test]
    fn propulsion_downgrade_rejected() {
        let mut state = SystemState::new();
        state.node_map.propulsion_level = 3;
        let result = apply_system_command(
            &mut state,
            &SystemCommand::UpgradePropulsion { new_level: 2 },
        );
        assert!(matches!(
            result,
            Err(SystemError::PropulsionDowngrade { .. })
        ));
    }

    #[test]
    fn world_specialization_all_roles() {
        let mut state = SystemState::new();
        let roles = [
            SystemRole::Industry,
            SystemRole::RawExtraction,
            SystemRole::Science,
            SystemRole::FuelProduction,
            SystemRole::PopulationHub,
            SystemRole::Unassigned,
        ];
        for role in &roles {
            let events = apply_system_command(
                &mut state,
                &SystemCommand::AddBody {
                    name: format!("{role:?} body"),
                    kind: BodyKind::InnerPlanet,
                    distance_au: 1.0,
                },
            )
            .unwrap();
            let body_id = match &events[0] {
                SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
                _ => panic!(),
            };
            apply_system_command(
                &mut state,
                &SystemCommand::AssignRole {
                    body_id: body_id.clone(),
                    role: role.clone(),
                },
            )
            .unwrap();
            assert_eq!(state.node_map.bodies[&body_id].role, *role);
        }
    }
}
