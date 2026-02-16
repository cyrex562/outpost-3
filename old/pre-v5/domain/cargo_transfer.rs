use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::domain::{
    cargo_request::{CargoRequest, CargoRequestId, CargoRequestManager, CargoRequestStatus},
    ResourceType, StationId, TrainId,
};

/// Cargo item in transit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoItem {
    pub request_id: CargoRequestId,
    pub resource: ResourceType,
    pub quantity: u32,
    pub destination: StationId,
}

/// Train cargo manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainCargo {
    pub train_id: TrainId,
    pub items: Vec<CargoItem>,
    pub capacity: u32,
}

impl TrainCargo {
    pub fn new(train_id: TrainId, capacity: u32) -> Self {
        Self {
            train_id,
            items: Vec::new(),
            capacity,
        }
    }

    /// Get current cargo weight
    pub fn current_load(&self) -> u32 {
        self.items.iter().map(|item| item.quantity).sum()
    }

    /// Get available capacity
    pub fn available_capacity(&self) -> u32 {
        self.capacity.saturating_sub(self.current_load())
    }

    /// Check if cargo can be loaded
    pub fn can_load(&self, quantity: u32) -> bool {
        self.available_capacity() >= quantity
    }

    /// Add cargo item
    pub fn add_cargo(&mut self, item: CargoItem) -> Result<(), CargoTransferError> {
        if !self.can_load(item.quantity) {
            return Err(CargoTransferError::InsufficientTrainCapacity);
        }
        self.items.push(item);
        Ok(())
    }

    /// Remove cargo for a specific destination
    pub fn remove_cargo_for_destination(&mut self, destination: StationId) -> Vec<CargoItem> {
        let (to_unload, remaining): (Vec<_>, Vec<_>) = self
            .items
            .drain(..)
            .partition(|item| item.destination == destination);
        self.items = remaining;
        to_unload
    }

    /// Get cargo items for a destination
    pub fn get_cargo_for_destination(&self, destination: StationId) -> Vec<&CargoItem> {
        self.items
            .iter()
            .filter(|item| item.destination == destination)
            .collect()
    }

    /// Total cargo items
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

/// Transfer operation result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferResult {
    pub request_id: CargoRequestId,
    pub quantity_transferred: u32,
    pub transfer_complete: bool,
}

/// Cargo transfer errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CargoTransferError {
    /// Train not found
    TrainNotFound,
    /// Station not found
    StationNotFound,
    /// Request not found
    RequestNotFound,
    /// Insufficient train capacity
    InsufficientTrainCapacity,
    /// Insufficient station capacity
    InsufficientStationCapacity,
    /// Request not in correct state
    InvalidRequestState,
    /// Train not at station
    TrainNotAtStation,
    /// No cargo to unload
    NoCargoToUnload,
}

impl fmt::Display for CargoTransferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CargoTransferError::TrainNotFound => write!(f, "Train not found"),
            CargoTransferError::StationNotFound => write!(f, "Station not found"),
            CargoTransferError::RequestNotFound => write!(f, "Request not found"),
            CargoTransferError::InsufficientTrainCapacity => {
                write!(f, "Insufficient train capacity")
            }
            CargoTransferError::InsufficientStationCapacity => {
                write!(f, "Insufficient station capacity")
            }
            CargoTransferError::InvalidRequestState => write!(f, "Invalid request state"),
            CargoTransferError::TrainNotAtStation => write!(f, "Train not at station"),
            CargoTransferError::NoCargoToUnload => write!(f, "No cargo to unload"),
        }
    }
}

/// Loading operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingOperation {
    pub train_id: TrainId,
    pub station_id: StationId,
    pub request_id: CargoRequestId,
    pub started_tick: u64,
    pub ticks_required: u64,
    pub ticks_completed: u64,
}

impl LoadingOperation {
    pub fn new(
        train_id: TrainId,
        station_id: StationId,
        request_id: CargoRequestId,
        started_tick: u64,
        quantity: u32,
    ) -> Self {
        // Loading time = quantity / 10 (10 units per tick), min 1 tick
        let ticks_required = (quantity / 10).max(1) as u64;

        Self {
            train_id,
            station_id,
            request_id,
            started_tick,
            ticks_required,
            ticks_completed: 0,
        }
    }

    /// Advance loading by ticks
    pub fn advance(&mut self, ticks: u64) -> bool {
        self.ticks_completed += ticks;
        self.is_complete()
    }

    /// Check if loading is complete
    pub fn is_complete(&self) -> bool {
        self.ticks_completed >= self.ticks_required
    }

    /// Get progress percentage (0-100)
    pub fn progress_percent(&self) -> u8 {
        if self.ticks_required == 0 {
            return 100;
        }
        ((self.ticks_completed as f32 / self.ticks_required as f32) * 100.0)
            .min(100.0)
            .floor() as u8
    }
}

/// Manages cargo loading/unloading operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoTransferManager {
    /// Train cargo manifests
    train_cargo: HashMap<TrainId, TrainCargo>,
    /// Active loading operations
    loading_operations: Vec<LoadingOperation>,
}

impl CargoTransferManager {
    pub fn new() -> Self {
        Self {
            train_cargo: HashMap::new(),
            loading_operations: Vec::new(),
        }
    }

    /// Register a train with cargo capacity
    pub fn register_train(&mut self, train_id: TrainId, capacity: u32) {
        self.train_cargo
            .insert(train_id, TrainCargo::new(train_id, capacity));
    }

    /// Get train cargo manifest
    pub fn get_train_cargo(&self, train_id: TrainId) -> Option<&TrainCargo> {
        self.train_cargo.get(&train_id)
    }

    /// Get mutable train cargo manifest
    pub fn get_train_cargo_mut(&mut self, train_id: TrainId) -> Option<&mut TrainCargo> {
        self.train_cargo.get_mut(&train_id)
    }

    /// Start loading operation
    pub fn start_loading(
        &mut self,
        train_id: TrainId,
        station_id: StationId,
        request: &CargoRequest,
        current_tick: u64,
        request_manager: &mut CargoRequestManager,
    ) -> Result<(), CargoTransferError> {
        // Validate request state
        if request.status != CargoRequestStatus::Pending
            && request.status != CargoRequestStatus::Assigned
        {
            return Err(CargoTransferError::InvalidRequestState);
        }

        // Validate source station matches
        if request.source_station != station_id {
            return Err(CargoTransferError::TrainNotAtStation);
        }

        // Check train capacity
        let cargo = self
            .train_cargo
            .get(&train_id)
            .ok_or(CargoTransferError::TrainNotFound)?;

        if !cargo.can_load(request.quantity) {
            return Err(CargoTransferError::InsufficientTrainCapacity);
        }

        // Update request status
        request_manager
            .update_status(request.id, CargoRequestStatus::Loading)
            .map_err(|_| CargoTransferError::RequestNotFound)?;

        // Create loading operation
        let operation = LoadingOperation::new(
            train_id,
            station_id,
            request.id,
            current_tick,
            request.quantity,
        );

        self.loading_operations.push(operation);
        Ok(())
    }

    /// Advance all loading operations by ticks
    pub fn advance_loading(
        &mut self,
        ticks: u64,
        request_manager: &mut CargoRequestManager,
    ) -> Vec<TransferResult> {
        let mut completed = Vec::new();
        let mut remaining = Vec::new();

        for mut op in self.loading_operations.drain(..) {
            if op.advance(ticks) {
                // Loading complete - get request data first
                let request_data = request_manager
                    .get_request(op.request_id)
                    .map(|r| (r.id, r.resource, r.quantity, r.destination_station));

                if let Some((id, resource, quantity, destination)) = request_data {
                    let cargo_item = CargoItem {
                        request_id: id,
                        resource,
                        quantity,
                        destination,
                    };

                    if let Some(train_cargo) = self.train_cargo.get_mut(&op.train_id) {
                        if train_cargo.add_cargo(cargo_item).is_ok() {
                            // Update request to in-transit
                            let _ = request_manager
                                .update_status(op.request_id, CargoRequestStatus::InTransit);

                            completed.push(TransferResult {
                                request_id: op.request_id,
                                quantity_transferred: quantity,
                                transfer_complete: true,
                            });
                        }
                    }
                }
            } else {
                // Still loading
                remaining.push(op);
            }
        }

        self.loading_operations = remaining;
        completed
    }

    /// Unload cargo at a station
    pub fn unload_at_station(
        &mut self,
        train_id: TrainId,
        station_id: StationId,
        request_manager: &mut CargoRequestManager,
        current_tick: u64,
    ) -> Result<Vec<TransferResult>, CargoTransferError> {
        let cargo = self
            .train_cargo
            .get_mut(&train_id)
            .ok_or(CargoTransferError::TrainNotFound)?;

        let items = cargo.remove_cargo_for_destination(station_id);

        if items.is_empty() {
            return Err(CargoTransferError::NoCargoToUnload);
        }

        let mut results = Vec::new();

        for item in items {
            // Update request status to unloading, then completed
            let _ = request_manager.update_status(item.request_id, CargoRequestStatus::Unloading);
            let _ = request_manager.complete_request(item.request_id, current_tick);

            results.push(TransferResult {
                request_id: item.request_id,
                quantity_transferred: item.quantity,
                transfer_complete: true,
            });
        }

        Ok(results)
    }

    /// Get active loading operations for a train
    pub fn get_loading_operations(&self, train_id: TrainId) -> Vec<&LoadingOperation> {
        self.loading_operations
            .iter()
            .filter(|op| op.train_id == train_id)
            .collect()
    }

    /// Get total active loading operations
    pub fn active_loading_count(&self) -> usize {
        self.loading_operations.len()
    }

    /// Remove train from tracking
    pub fn remove_train(&mut self, train_id: TrainId) {
        self.train_cargo.remove(&train_id);
        self.loading_operations.retain(|op| op.train_id != train_id);
    }
}

impl Default for CargoTransferManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_train_cargo_creation() {
        let cargo = TrainCargo::new(TrainId(1), 500);
        assert_eq!(cargo.train_id, TrainId(1));
        assert_eq!(cargo.capacity, 500);
        assert_eq!(cargo.current_load(), 0);
        assert_eq!(cargo.available_capacity(), 500);
    }

    #[test]
    fn test_train_cargo_add_item() {
        let mut cargo = TrainCargo::new(TrainId(1), 500);

        let item = CargoItem {
            request_id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 100,
            destination: StationId(2),
        };

        cargo.add_cargo(item).unwrap();
        assert_eq!(cargo.current_load(), 100);
        assert_eq!(cargo.available_capacity(), 400);
        assert_eq!(cargo.item_count(), 1);
    }

    #[test]
    fn test_train_cargo_capacity_exceeded() {
        let mut cargo = TrainCargo::new(TrainId(1), 100);

        let item = CargoItem {
            request_id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 150,
            destination: StationId(2),
        };

        let result = cargo.add_cargo(item);
        assert_eq!(result, Err(CargoTransferError::InsufficientTrainCapacity));
    }

    #[test]
    fn test_train_cargo_unload_by_destination() {
        let mut cargo = TrainCargo::new(TrainId(1), 500);

        cargo
            .add_cargo(CargoItem {
                request_id: CargoRequestId(1),
                resource: ResourceType::Steel,
                quantity: 100,
                destination: StationId(2),
            })
            .unwrap();

        cargo
            .add_cargo(CargoItem {
                request_id: CargoRequestId(2),
                resource: ResourceType::Food,
                quantity: 50,
                destination: StationId(3),
            })
            .unwrap();

        cargo
            .add_cargo(CargoItem {
                request_id: CargoRequestId(3),
                resource: ResourceType::Water,
                quantity: 75,
                destination: StationId(2),
            })
            .unwrap();

        let unloaded = cargo.remove_cargo_for_destination(StationId(2));
        assert_eq!(unloaded.len(), 2);
        assert_eq!(cargo.item_count(), 1);
        assert_eq!(cargo.current_load(), 50);
    }

    #[test]
    fn test_loading_operation() {
        let mut op = LoadingOperation::new(TrainId(1), StationId(1), CargoRequestId(1), 0, 100);

        assert_eq!(op.ticks_required, 10); // 100 / 10
        assert!(!op.is_complete());
        assert_eq!(op.progress_percent(), 0);

        op.advance(5);
        assert!(!op.is_complete());
        assert_eq!(op.progress_percent(), 50);

        op.advance(5);
        assert!(op.is_complete());
        assert_eq!(op.progress_percent(), 100);
    }

    #[test]
    fn test_loading_operation_min_ticks() {
        let op = LoadingOperation::new(TrainId(1), StationId(1), CargoRequestId(1), 0, 5);

        // Small quantity should take minimum 1 tick
        assert_eq!(op.ticks_required, 1);
    }

    #[test]
    fn test_transfer_manager_creation() {
        let manager = CargoTransferManager::new();
        assert_eq!(manager.active_loading_count(), 0);
    }

    #[test]
    fn test_transfer_manager_register_train() {
        let mut manager = CargoTransferManager::new();
        manager.register_train(TrainId(1), 500);

        let cargo = manager.get_train_cargo(TrainId(1)).unwrap();
        assert_eq!(cargo.capacity, 500);
    }

    #[test]
    fn test_transfer_manager_start_loading() {
        let mut manager = CargoTransferManager::new();
        let mut request_manager = CargoRequestManager::new();

        manager.register_train(TrainId(1), 500);

        let request_id = request_manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();

        let request = request_manager.get_request(request_id).unwrap().clone();

        manager
            .start_loading(TrainId(1), StationId(1), &request, 0, &mut request_manager)
            .unwrap();

        assert_eq!(manager.active_loading_count(), 1);

        let updated_request = request_manager.get_request(request_id).unwrap();
        assert_eq!(updated_request.status, CargoRequestStatus::Loading);
    }

    #[test]
    fn test_transfer_manager_invalid_source_station() {
        let mut manager = CargoTransferManager::new();
        let mut request_manager = CargoRequestManager::new();

        manager.register_train(TrainId(1), 500);

        let request_id = request_manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();

        let request = request_manager.get_request(request_id).unwrap().clone();

        // Try to load at wrong station
        let result =
            manager.start_loading(TrainId(1), StationId(3), &request, 0, &mut request_manager);

        assert_eq!(result, Err(CargoTransferError::TrainNotAtStation));
    }

    #[test]
    fn test_transfer_manager_advance_loading() {
        let mut manager = CargoTransferManager::new();
        let mut request_manager = CargoRequestManager::new();

        manager.register_train(TrainId(1), 500);

        let request_id = request_manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();

        let request = request_manager.get_request(request_id).unwrap().clone();

        manager
            .start_loading(TrainId(1), StationId(1), &request, 0, &mut request_manager)
            .unwrap();

        // Advance loading
        let results = manager.advance_loading(10, &mut request_manager);

        assert_eq!(results.len(), 1);
        assert!(results[0].transfer_complete);
        assert_eq!(manager.active_loading_count(), 0);

        // Cargo should be on train now
        let cargo = manager.get_train_cargo(TrainId(1)).unwrap();
        assert_eq!(cargo.item_count(), 1);
        assert_eq!(cargo.current_load(), 100);

        // Request should be in transit
        let updated_request = request_manager.get_request(request_id).unwrap();
        assert_eq!(updated_request.status, CargoRequestStatus::InTransit);
    }

    #[test]
    fn test_transfer_manager_unload_at_station() {
        let mut manager = CargoTransferManager::new();
        let mut request_manager = CargoRequestManager::new();

        manager.register_train(TrainId(1), 500);

        // Add cargo directly to train
        let cargo = manager.get_train_cargo_mut(TrainId(1)).unwrap();
        cargo
            .add_cargo(CargoItem {
                request_id: CargoRequestId(1),
                resource: ResourceType::Steel,
                quantity: 100,
                destination: StationId(2),
            })
            .unwrap();

        // Create corresponding request
        let request_id = request_manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();
        request_manager
            .update_status(request_id, CargoRequestStatus::InTransit)
            .unwrap();

        // Unload at destination
        let results = manager
            .unload_at_station(TrainId(1), StationId(2), &mut request_manager, 100)
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].quantity_transferred, 100);

        // Train should be empty
        let cargo = manager.get_train_cargo(TrainId(1)).unwrap();
        assert_eq!(cargo.item_count(), 0);

        // Request should be completed
        let request = request_manager.get_request(request_id).unwrap();
        assert_eq!(request.status, CargoRequestStatus::Completed);
        assert_eq!(request.completed_tick, Some(100));
    }

    #[test]
    fn test_transfer_manager_unload_wrong_station() {
        let mut manager = CargoTransferManager::new();
        let mut request_manager = CargoRequestManager::new();

        manager.register_train(TrainId(1), 500);

        let cargo = manager.get_train_cargo_mut(TrainId(1)).unwrap();
        cargo
            .add_cargo(CargoItem {
                request_id: CargoRequestId(1),
                resource: ResourceType::Steel,
                quantity: 100,
                destination: StationId(2),
            })
            .unwrap();

        // Try to unload at wrong station
        let result = manager.unload_at_station(TrainId(1), StationId(3), &mut request_manager, 100);

        assert_eq!(result, Err(CargoTransferError::NoCargoToUnload));
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            CargoTransferError::TrainNotFound.to_string(),
            "Train not found"
        );
        assert_eq!(
            CargoTransferError::InsufficientTrainCapacity.to_string(),
            "Insufficient train capacity"
        );
        assert_eq!(
            CargoTransferError::NoCargoToUnload.to_string(),
            "No cargo to unload"
        );
    }
}
