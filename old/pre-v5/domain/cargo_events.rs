use serde::{Deserialize, Serialize};
use std::fmt;

use crate::domain::{CargoRequestId, ResourceType, StationId, TrainId};

/// Cargo-related events for event sourcing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CargoEvent {
    /// New cargo request created
    CargoRequestCreated {
        request_id: CargoRequestId,
        resource: ResourceType,
        quantity: u32,
        source_station: StationId,
        destination_station: StationId,
        priority: u8,
        tick: u64,
    },

    /// Cargo request assigned to a route
    CargoAssigned {
        request_id: CargoRequestId,
        tick: u64,
    },

    /// Train started loading cargo
    LoadingStarted {
        request_id: CargoRequestId,
        train_id: TrainId,
        station_id: StationId,
        quantity: u32,
        tick: u64,
    },

    /// Train completed loading cargo
    LoadingCompleted {
        request_id: CargoRequestId,
        train_id: TrainId,
        station_id: StationId,
        quantity: u32,
        ticks_taken: u64,
        tick: u64,
    },

    /// Cargo in transit on train
    CargoInTransit {
        request_id: CargoRequestId,
        train_id: TrainId,
        current_location: StationId,
        destination: StationId,
        tick: u64,
    },

    /// Train started unloading cargo
    UnloadingStarted {
        request_id: CargoRequestId,
        train_id: TrainId,
        station_id: StationId,
        quantity: u32,
        tick: u64,
    },

    /// Train completed unloading cargo
    UnloadingCompleted {
        request_id: CargoRequestId,
        train_id: TrainId,
        station_id: StationId,
        quantity: u32,
        tick: u64,
    },

    /// Cargo successfully delivered
    CargoDelivered {
        request_id: CargoRequestId,
        train_id: TrainId,
        destination_station: StationId,
        resource: ResourceType,
        quantity: u32,
        total_time: u64,
        tick: u64,
    },

    /// Cargo request cancelled
    CargoRequestCancelled {
        request_id: CargoRequestId,
        reason: String,
        tick: u64,
    },

    /// Train capacity updated
    TrainCapacityUpdated {
        train_id: TrainId,
        old_capacity: u32,
        new_capacity: u32,
        tick: u64,
    },

    /// Cargo flow initiated from production chain
    CargoFlowInitiated {
        source_station: StationId,
        destination_station: StationId,
        resource: ResourceType,
        initial_quantity: u32,
        priority: u8,
        tick: u64,
    },
}

impl CargoEvent {
    /// Get the tick when this event occurred
    pub fn tick(&self) -> u64 {
        match self {
            CargoEvent::CargoRequestCreated { tick, .. } => *tick,
            CargoEvent::CargoAssigned { tick, .. } => *tick,
            CargoEvent::LoadingStarted { tick, .. } => *tick,
            CargoEvent::LoadingCompleted { tick, .. } => *tick,
            CargoEvent::CargoInTransit { tick, .. } => *tick,
            CargoEvent::UnloadingStarted { tick, .. } => *tick,
            CargoEvent::UnloadingCompleted { tick, .. } => *tick,
            CargoEvent::CargoDelivered { tick, .. } => *tick,
            CargoEvent::CargoRequestCancelled { tick, .. } => *tick,
            CargoEvent::TrainCapacityUpdated { tick, .. } => *tick,
            CargoEvent::CargoFlowInitiated { tick, .. } => *tick,
        }
    }

    /// Get associated request ID if applicable
    pub fn request_id(&self) -> Option<CargoRequestId> {
        match self {
            CargoEvent::CargoRequestCreated { request_id, .. } => Some(*request_id),
            CargoEvent::CargoAssigned { request_id, .. } => Some(*request_id),
            CargoEvent::LoadingStarted { request_id, .. } => Some(*request_id),
            CargoEvent::LoadingCompleted { request_id, .. } => Some(*request_id),
            CargoEvent::CargoInTransit { request_id, .. } => Some(*request_id),
            CargoEvent::UnloadingStarted { request_id, .. } => Some(*request_id),
            CargoEvent::UnloadingCompleted { request_id, .. } => Some(*request_id),
            CargoEvent::CargoDelivered { request_id, .. } => Some(*request_id),
            CargoEvent::CargoRequestCancelled { request_id, .. } => Some(*request_id),
            CargoEvent::TrainCapacityUpdated { .. } => None,
            CargoEvent::CargoFlowInitiated { .. } => None,
        }
    }

    /// Get associated train ID if applicable
    pub fn train_id(&self) -> Option<TrainId> {
        match self {
            CargoEvent::LoadingStarted { train_id, .. } => Some(*train_id),
            CargoEvent::LoadingCompleted { train_id, .. } => Some(*train_id),
            CargoEvent::CargoInTransit { train_id, .. } => Some(*train_id),
            CargoEvent::UnloadingStarted { train_id, .. } => Some(*train_id),
            CargoEvent::UnloadingCompleted { train_id, .. } => Some(*train_id),
            CargoEvent::CargoDelivered { train_id, .. } => Some(*train_id),
            CargoEvent::TrainCapacityUpdated { train_id, .. } => Some(*train_id),
            _ => None,
        }
    }

    /// Get event type as string
    pub fn event_type(&self) -> &'static str {
        match self {
            CargoEvent::CargoRequestCreated { .. } => "CargoRequestCreated",
            CargoEvent::CargoAssigned { .. } => "CargoAssigned",
            CargoEvent::LoadingStarted { .. } => "LoadingStarted",
            CargoEvent::LoadingCompleted { .. } => "LoadingCompleted",
            CargoEvent::CargoInTransit { .. } => "CargoInTransit",
            CargoEvent::UnloadingStarted { .. } => "UnloadingStarted",
            CargoEvent::UnloadingCompleted { .. } => "UnloadingCompleted",
            CargoEvent::CargoDelivered { .. } => "CargoDelivered",
            CargoEvent::CargoRequestCancelled { .. } => "CargoRequestCancelled",
            CargoEvent::TrainCapacityUpdated { .. } => "TrainCapacityUpdated",
            CargoEvent::CargoFlowInitiated { .. } => "CargoFlowInitiated",
        }
    }

    /// Get resource associated with event if applicable
    pub fn resource(&self) -> Option<ResourceType> {
        match self {
            CargoEvent::CargoRequestCreated { resource, .. } => Some(*resource),
            CargoEvent::CargoDelivered { resource, .. } => Some(*resource),
            CargoEvent::CargoFlowInitiated { resource, .. } => Some(*resource),
            _ => None,
        }
    }

    /// Get quantity associated with event if applicable
    pub fn quantity(&self) -> Option<u32> {
        match self {
            CargoEvent::CargoRequestCreated { quantity, .. } => Some(*quantity),
            CargoEvent::LoadingStarted { quantity, .. } => Some(*quantity),
            CargoEvent::LoadingCompleted { quantity, .. } => Some(*quantity),
            CargoEvent::UnloadingStarted { quantity, .. } => Some(*quantity),
            CargoEvent::UnloadingCompleted { quantity, .. } => Some(*quantity),
            CargoEvent::CargoDelivered { quantity, .. } => Some(*quantity),
            CargoEvent::CargoFlowInitiated {
                initial_quantity, ..
            } => Some(*initial_quantity),
            _ => None,
        }
    }
}

impl fmt::Display for CargoEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CargoEvent::CargoRequestCreated {
                request_id,
                resource,
                quantity,
                ..
            } => {
                write!(
                    f,
                    "Cargo request {:?} created: {} x {:?}",
                    request_id, quantity, resource
                )
            }
            CargoEvent::CargoAssigned { request_id, .. } => {
                write!(f, "Cargo request {:?} assigned", request_id)
            }
            CargoEvent::LoadingStarted {
                request_id,
                train_id,
                quantity,
                ..
            } => {
                write!(
                    f,
                    "Loading started: request {:?} on train {:?} ({})",
                    request_id, train_id, quantity
                )
            }
            CargoEvent::LoadingCompleted { request_id, .. } => {
                write!(f, "Loading completed: request {:?}", request_id)
            }
            CargoEvent::CargoInTransit {
                request_id,
                train_id,
                ..
            } => {
                write!(
                    f,
                    "Cargo in transit: request {:?} on train {:?}",
                    request_id, train_id
                )
            }
            CargoEvent::UnloadingStarted { request_id, .. } => {
                write!(f, "Unloading started: request {:?}", request_id)
            }
            CargoEvent::UnloadingCompleted { request_id, .. } => {
                write!(f, "Unloading completed: request {:?}", request_id)
            }
            CargoEvent::CargoDelivered {
                request_id,
                resource,
                quantity,
                ..
            } => {
                write!(
                    f,
                    "Cargo delivered: request {:?}, {} x {:?}",
                    request_id, quantity, resource
                )
            }
            CargoEvent::CargoRequestCancelled {
                request_id, reason, ..
            } => {
                write!(f, "Cargo request {:?} cancelled: {}", request_id, reason)
            }
            CargoEvent::TrainCapacityUpdated {
                train_id,
                old_capacity,
                new_capacity,
                ..
            } => {
                write!(
                    f,
                    "Train {:?} capacity updated: {} -> {}",
                    train_id, old_capacity, new_capacity
                )
            }
            CargoEvent::CargoFlowInitiated {
                resource,
                initial_quantity,
                ..
            } => {
                write!(
                    f,
                    "Cargo flow initiated: {} x {:?}",
                    initial_quantity, resource
                )
            }
        }
    }
}

/// Cargo event log entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoEventLogEntry {
    pub sequence: u64,
    pub event: CargoEvent,
    pub recorded_tick: u64,
}

impl CargoEventLogEntry {
    pub fn new(sequence: u64, event: CargoEvent, recorded_tick: u64) -> Self {
        Self {
            sequence,
            event,
            recorded_tick,
        }
    }
}

/// Manages cargo event log for event sourcing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoEventLog {
    entries: Vec<CargoEventLogEntry>,
    next_sequence: u64,
}

impl CargoEventLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_sequence: 1,
        }
    }

    /// Append an event to the log
    pub fn append(&mut self, event: CargoEvent, recorded_tick: u64) -> u64 {
        let sequence = self.next_sequence;
        let entry = CargoEventLogEntry::new(sequence, event, recorded_tick);
        self.entries.push(entry);
        self.next_sequence += 1;
        sequence
    }

    /// Get all events
    pub fn all_events(&self) -> Vec<&CargoEvent> {
        self.entries.iter().map(|e| &e.event).collect()
    }

    /// Get events for a specific request
    pub fn events_for_request(&self, request_id: CargoRequestId) -> Vec<&CargoEvent> {
        self.entries
            .iter()
            .filter(|e| e.event.request_id() == Some(request_id))
            .map(|e| &e.event)
            .collect()
    }

    /// Get events for a specific train
    pub fn events_for_train(&self, train_id: TrainId) -> Vec<&CargoEvent> {
        self.entries
            .iter()
            .filter(|e| e.event.train_id() == Some(train_id))
            .map(|e| &e.event)
            .collect()
    }

    /// Get events by type
    pub fn events_by_type(&self, event_type: &str) -> Vec<&CargoEvent> {
        self.entries
            .iter()
            .filter(|e| e.event.event_type() == event_type)
            .map(|e| &e.event)
            .collect()
    }

    /// Get events in tick range
    pub fn events_in_range(&self, start_tick: u64, end_tick: u64) -> Vec<&CargoEvent> {
        self.entries
            .iter()
            .filter(|e| {
                let tick = e.event.tick();
                tick >= start_tick && tick <= end_tick
            })
            .map(|e| &e.event)
            .collect()
    }

    /// Get total event count
    pub fn event_count(&self) -> usize {
        self.entries.len()
    }

    /// Get entries
    pub fn entries(&self) -> &[CargoEventLogEntry] {
        &self.entries
    }

    /// Clear log (for testing/reset)
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_sequence = 1;
    }
}

impl Default for CargoEventLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_event_tick() {
        let event = CargoEvent::CargoRequestCreated {
            request_id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 100,
            source_station: StationId(1),
            destination_station: StationId(2),
            priority: 5,
            tick: 42,
        };

        assert_eq!(event.tick(), 42);
    }

    #[test]
    fn test_cargo_event_request_id() {
        let event = CargoEvent::CargoRequestCreated {
            request_id: CargoRequestId(5),
            resource: ResourceType::Steel,
            quantity: 100,
            source_station: StationId(1),
            destination_station: StationId(2),
            priority: 5,
            tick: 0,
        };

        assert_eq!(event.request_id(), Some(CargoRequestId(5)));
    }

    #[test]
    fn test_cargo_event_train_id() {
        let event = CargoEvent::LoadingStarted {
            request_id: CargoRequestId(1),
            train_id: TrainId(3),
            station_id: StationId(1),
            quantity: 100,
            tick: 0,
        };

        assert_eq!(event.train_id(), Some(TrainId(3)));
    }

    #[test]
    fn test_cargo_event_type() {
        let event = CargoEvent::CargoDelivered {
            request_id: CargoRequestId(1),
            train_id: TrainId(1),
            destination_station: StationId(2),
            resource: ResourceType::Steel,
            quantity: 100,
            total_time: 50,
            tick: 100,
        };

        assert_eq!(event.event_type(), "CargoDelivered");
    }

    #[test]
    fn test_cargo_event_resource() {
        let event = CargoEvent::CargoRequestCreated {
            request_id: CargoRequestId(1),
            resource: ResourceType::Food,
            quantity: 100,
            source_station: StationId(1),
            destination_station: StationId(2),
            priority: 5,
            tick: 0,
        };

        assert_eq!(event.resource(), Some(ResourceType::Food));
    }

    #[test]
    fn test_cargo_event_quantity() {
        let event = CargoEvent::LoadingStarted {
            request_id: CargoRequestId(1),
            train_id: TrainId(1),
            station_id: StationId(1),
            quantity: 250,
            tick: 0,
        };

        assert_eq!(event.quantity(), Some(250));
    }

    #[test]
    fn test_cargo_event_display() {
        let event = CargoEvent::CargoRequestCreated {
            request_id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 100,
            source_station: StationId(1),
            destination_station: StationId(2),
            priority: 5,
            tick: 0,
        };

        let display_str = format!("{}", event);
        assert!(display_str.contains("Cargo request"));
        assert!(display_str.contains("Steel"));
    }

    #[test]
    fn test_event_log_creation() {
        let log = CargoEventLog::new();
        assert_eq!(log.event_count(), 0);
    }

    #[test]
    fn test_event_log_append() {
        let mut log = CargoEventLog::new();

        let event = CargoEvent::CargoRequestCreated {
            request_id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 100,
            source_station: StationId(1),
            destination_station: StationId(2),
            priority: 5,
            tick: 0,
        };

        let seq = log.append(event, 0);
        assert_eq!(seq, 1);
        assert_eq!(log.event_count(), 1);
    }

    #[test]
    fn test_event_log_sequence_increment() {
        let mut log = CargoEventLog::new();

        let event1 = CargoEvent::CargoRequestCreated {
            request_id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 100,
            source_station: StationId(1),
            destination_station: StationId(2),
            priority: 5,
            tick: 0,
        };

        let event2 = CargoEvent::CargoAssigned {
            request_id: CargoRequestId(1),
            tick: 10,
        };

        let seq1 = log.append(event1, 0);
        let seq2 = log.append(event2, 10);

        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
    }

    #[test]
    fn test_event_log_events_for_request() {
        let mut log = CargoEventLog::new();

        let event1 = CargoEvent::CargoRequestCreated {
            request_id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 100,
            source_station: StationId(1),
            destination_station: StationId(2),
            priority: 5,
            tick: 0,
        };

        let event2 = CargoEvent::CargoAssigned {
            request_id: CargoRequestId(1),
            tick: 10,
        };

        let event3 = CargoEvent::CargoAssigned {
            request_id: CargoRequestId(2),
            tick: 15,
        };

        log.append(event1, 0);
        log.append(event2, 10);
        log.append(event3, 15);

        let events = log.events_for_request(CargoRequestId(1));
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_event_log_events_for_train() {
        let mut log = CargoEventLog::new();

        let event1 = CargoEvent::LoadingStarted {
            request_id: CargoRequestId(1),
            train_id: TrainId(1),
            station_id: StationId(1),
            quantity: 100,
            tick: 0,
        };

        let event2 = CargoEvent::LoadingCompleted {
            request_id: CargoRequestId(1),
            train_id: TrainId(1),
            station_id: StationId(1),
            quantity: 100,
            ticks_taken: 10,
            tick: 10,
        };

        let event3 = CargoEvent::LoadingStarted {
            request_id: CargoRequestId(2),
            train_id: TrainId(2),
            station_id: StationId(1),
            quantity: 50,
            tick: 5,
        };

        log.append(event1, 0);
        log.append(event2, 10);
        log.append(event3, 5);

        let events = log.events_for_train(TrainId(1));
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_event_log_events_by_type() {
        let mut log = CargoEventLog::new();

        let event1 = CargoEvent::CargoRequestCreated {
            request_id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 100,
            source_station: StationId(1),
            destination_station: StationId(2),
            priority: 5,
            tick: 0,
        };

        let event2 = CargoEvent::CargoAssigned {
            request_id: CargoRequestId(1),
            tick: 10,
        };

        let event3 = CargoEvent::CargoRequestCreated {
            request_id: CargoRequestId(2),
            resource: ResourceType::Food,
            quantity: 50,
            source_station: StationId(1),
            destination_station: StationId(2),
            priority: 3,
            tick: 15,
        };

        log.append(event1, 0);
        log.append(event2, 10);
        log.append(event3, 15);

        let created_events = log.events_by_type("CargoRequestCreated");
        assert_eq!(created_events.len(), 2);

        let assigned_events = log.events_by_type("CargoAssigned");
        assert_eq!(assigned_events.len(), 1);
    }

    #[test]
    fn test_event_log_events_in_range() {
        let mut log = CargoEventLog::new();

        let event1 = CargoEvent::CargoRequestCreated {
            request_id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 100,
            source_station: StationId(1),
            destination_station: StationId(2),
            priority: 5,
            tick: 10,
        };

        let event2 = CargoEvent::CargoAssigned {
            request_id: CargoRequestId(1),
            tick: 50,
        };

        let event3 = CargoEvent::CargoDelivered {
            request_id: CargoRequestId(1),
            train_id: TrainId(1),
            destination_station: StationId(2),
            resource: ResourceType::Steel,
            quantity: 100,
            total_time: 40,
            tick: 100,
        };

        log.append(event1, 10);
        log.append(event2, 50);
        log.append(event3, 100);

        let range_events = log.events_in_range(25, 75);
        assert_eq!(range_events.len(), 1); // Only event2 with tick 50
    }

    #[test]
    fn test_event_log_clear() {
        let mut log = CargoEventLog::new();

        let event = CargoEvent::CargoAssigned {
            request_id: CargoRequestId(1),
            tick: 0,
        };

        log.append(event, 0);
        assert_eq!(log.event_count(), 1);

        log.clear();
        assert_eq!(log.event_count(), 0);
    }

    #[test]
    fn test_cargo_event_log_entry() {
        let event = CargoEvent::CargoAssigned {
            request_id: CargoRequestId(1),
            tick: 42,
        };

        let entry = CargoEventLogEntry::new(1, event, 42);
        assert_eq!(entry.sequence, 1);
        assert_eq!(entry.event.tick(), 42);
        assert_eq!(entry.recorded_tick, 42);
    }

    #[test]
    fn test_event_no_request_id() {
        let event = CargoEvent::TrainCapacityUpdated {
            train_id: TrainId(1),
            old_capacity: 500,
            new_capacity: 600,
            tick: 0,
        };

        assert_eq!(event.request_id(), None);
    }

    #[test]
    fn test_event_no_train_id() {
        let event = CargoEvent::CargoFlowInitiated {
            source_station: StationId(1),
            destination_station: StationId(2),
            resource: ResourceType::Steel,
            initial_quantity: 100,
            priority: 5,
            tick: 0,
        };

        assert_eq!(event.train_id(), None);
    }
}
