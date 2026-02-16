use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

use crate::domain::{ColonyId, RouteId, TrainId};

/// Represents a train queued for a route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedTrain {
    pub train_id: TrainId,
    pub route_id: RouteId,
    pub priority: u8,
    pub time_queued: u64, // Turn number when added to queue
}

impl QueuedTrain {
    pub fn new(train_id: TrainId, route_id: RouteId, priority: u8, time_queued: u64) -> Self {
        Self {
            train_id,
            route_id,
            priority,
            time_queued,
        }
    }
}

/// State of a train on a route
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainRouteState {
    /// Waiting in queue to depart
    Queued,
    /// Currently moving on the route
    Active,
    /// Waiting for cargo at a station
    Loading,
    /// Unloading cargo at a station
    Unloading,
    /// Route complete, train idle
    Completed,
    /// Route was cancelled
    Cancelled,
}

impl fmt::Display for TrainRouteState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TrainRouteState::Queued => write!(f, "Queued"),
            TrainRouteState::Active => write!(f, "Active"),
            TrainRouteState::Loading => write!(f, "Loading"),
            TrainRouteState::Unloading => write!(f, "Unloading"),
            TrainRouteState::Completed => write!(f, "Completed"),
            TrainRouteState::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Represents a train's assignment to a route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainAssignment {
    pub train_id: TrainId,
    pub route_id: RouteId,
    pub state: TrainRouteState,
    pub assigned_turn: u64,
    pub dispatch_turn: Option<u64>, // When train actually started moving
}

impl TrainAssignment {
    pub fn new(train_id: TrainId, route_id: RouteId, turn: u64) -> Self {
        Self {
            train_id,
            route_id,
            state: TrainRouteState::Queued,
            assigned_turn: turn,
            dispatch_turn: None,
        }
    }

    /// Transition train to active state
    pub fn dispatch(&mut self, turn: u64) {
        self.state = TrainRouteState::Active;
        self.dispatch_turn = Some(turn);
    }

    /// Check if train has been waiting longer than specified turns
    pub fn waiting_for_turns(&self, current_turn: u64) -> u64 {
        current_turn.saturating_sub(self.assigned_turn)
    }
}

/// Queueing error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    /// Train is already assigned to another route
    TrainAlreadyAssigned,
    /// Train not found in queue
    TrainNotInQueue,
    /// Route not found
    RouteNotFound,
    /// Invalid priority value
    InvalidPriority,
    /// Train not found in assignments
    TrainNotFound,
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QueueError::TrainAlreadyAssigned => {
                write!(f, "Train is already assigned to another route")
            }
            QueueError::TrainNotInQueue => write!(f, "Train not found in queue"),
            QueueError::RouteNotFound => write!(f, "Route not found"),
            QueueError::InvalidPriority => write!(f, "Invalid priority value"),
            QueueError::TrainNotFound => write!(f, "Train not found"),
        }
    }
}

/// Route queue - manages train queuing and dispatch for a specific route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteQueue {
    pub route_id: RouteId,
    /// Queue of trains waiting to be dispatched
    queue: VecDeque<QueuedTrain>,
    /// Currently active train on this route
    active_train: Option<TrainId>,
    /// Trains that have completed this route
    completed_trains: Vec<TrainId>,
}

impl RouteQueue {
    pub fn new(route_id: RouteId) -> Self {
        Self {
            route_id,
            queue: VecDeque::new(),
            active_train: None,
            completed_trains: Vec::new(),
        }
    }

    /// Add a train to the queue
    pub fn enqueue(
        &mut self,
        train_id: TrainId,
        priority: u8,
        turn: u64,
    ) -> Result<(), QueueError> {
        if priority > 10 {
            return Err(QueueError::InvalidPriority);
        }

        let queued = QueuedTrain::new(train_id, self.route_id, priority, turn);

        // Insert in priority order (higher priority first, then FIFO)
        let insert_pos = self
            .queue
            .iter()
            .position(|t| {
                if t.priority == priority {
                    false // Keep in FIFO order for same priority
                } else {
                    t.priority < priority
                }
            })
            .unwrap_or(self.queue.len());

        self.queue.insert(insert_pos, queued);
        Ok(())
    }

    /// Get the next train to dispatch (highest priority, earliest if tied)
    pub fn peek_next(&self) -> Option<&QueuedTrain> {
        self.queue.front()
    }

    /// Dispatch the next train in queue
    pub fn dispatch_next(&mut self) -> Result<TrainId, QueueError> {
        if let Some(queued) = self.queue.pop_front() {
            self.active_train = Some(queued.train_id);
            Ok(queued.train_id)
        } else {
            Err(QueueError::TrainNotInQueue)
        }
    }

    /// Remove a specific train from queue (if not yet dispatched)
    pub fn remove_from_queue(&mut self, train_id: TrainId) -> Result<(), QueueError> {
        if self.active_train == Some(train_id) {
            return Err(QueueError::TrainAlreadyAssigned);
        }

        let initial_len = self.queue.len();
        self.queue.retain(|t| t.train_id != train_id);

        if self.queue.len() < initial_len {
            Ok(())
        } else {
            Err(QueueError::TrainNotInQueue)
        }
    }

    /// Mark the active train as completed
    pub fn complete_train(&mut self) {
        if let Some(train_id) = self.active_train.take() {
            self.completed_trains.push(train_id);
        }
    }

    /// Cancel the active train's route
    pub fn cancel_active_train(&mut self) -> Option<TrainId> {
        self.active_train.take()
    }

    /// Get queue length
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Check if a train is in this queue
    pub fn has_train(&self, train_id: TrainId) -> bool {
        self.active_train == Some(train_id)
            || self.queue.iter().any(|t| t.train_id == train_id)
            || self.completed_trains.contains(&train_id)
    }

    /// Get current active train on this route
    pub fn active_train(&self) -> Option<TrainId> {
        self.active_train
    }

    /// Get all trains in queue (not including active)
    pub fn queued_trains(&self) -> Vec<&QueuedTrain> {
        self.queue.iter().collect()
    }
}

/// Colony-wide train assignment manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainAssignmentManager {
    colony_id: ColonyId,
    /// Per-route queues
    route_queues: Vec<RouteQueue>,
    /// Trains to their current assignments
    assignments: Vec<TrainAssignment>,
}

impl TrainAssignmentManager {
    pub fn new(colony_id: ColonyId) -> Self {
        Self {
            colony_id,
            route_queues: Vec::new(),
            assignments: Vec::new(),
        }
    }

    /// Get or create queue for a route
    fn get_or_create_queue(&mut self, route_id: RouteId) -> &mut RouteQueue {
        if !self.route_queues.iter().any(|q| q.route_id == route_id) {
            self.route_queues.push(RouteQueue::new(route_id));
        }
        self.route_queues
            .iter_mut()
            .find(|q| q.route_id == route_id)
            .unwrap()
    }

    /// Assign a train to a route (adds to queue)
    pub fn assign_train(
        &mut self,
        train_id: TrainId,
        route_id: RouteId,
        priority: u8,
        turn: u64,
    ) -> Result<(), QueueError> {
        // Check if train is already assigned to an active route
        if self
            .assignments
            .iter()
            .any(|a| a.train_id == train_id && a.state != TrainRouteState::Completed)
        {
            return Err(QueueError::TrainAlreadyAssigned);
        }

        // Create assignment
        let assignment = TrainAssignment::new(train_id, route_id, turn);
        self.assignments.push(assignment);

        // Add to route queue
        let queue = self.get_or_create_queue(route_id);
        queue.enqueue(train_id, priority, turn)?;

        Ok(())
    }

    /// Dispatch the next train waiting on a route
    pub fn dispatch_next_on_route(
        &mut self,
        route_id: RouteId,
        turn: u64,
    ) -> Result<TrainId, QueueError> {
        let queue = self
            .route_queues
            .iter_mut()
            .find(|q| q.route_id == route_id)
            .ok_or(QueueError::RouteNotFound)?;

        let train_id = queue.dispatch_next()?;

        // Update assignment
        if let Some(assignment) = self.assignments.iter_mut().find(|a| a.train_id == train_id) {
            assignment.dispatch(turn);
        }

        Ok(train_id)
    }

    /// Get trains assigned to a route
    pub fn get_route_trains(&self, route_id: RouteId) -> Vec<TrainId> {
        self.assignments
            .iter()
            .filter(|a| a.route_id == route_id)
            .map(|a| a.train_id)
            .collect()
    }

    /// Get all queued trains for a route
    pub fn get_queued_for_route(&self, route_id: RouteId) -> Vec<&QueuedTrain> {
        self.route_queues
            .iter()
            .find(|q| q.route_id == route_id)
            .map(|q| q.queued_trains())
            .unwrap_or_default()
    }

    /// Get train's current assignment
    pub fn get_assignment(&self, train_id: TrainId) -> Option<&TrainAssignment> {
        self.assignments.iter().find(|a| a.train_id == train_id)
    }

    /// Get mutable train assignment
    pub fn get_assignment_mut(&mut self, train_id: TrainId) -> Option<&mut TrainAssignment> {
        self.assignments.iter_mut().find(|a| a.train_id == train_id)
    }

    /// Mark a train as completed on its route
    pub fn complete_train(&mut self, train_id: TrainId) -> Result<(), QueueError> {
        let assignment = self
            .get_assignment_mut(train_id)
            .ok_or(QueueError::TrainNotFound)?;

        let route_id = assignment.route_id;
        assignment.state = TrainRouteState::Completed;

        if let Some(queue) = self
            .route_queues
            .iter_mut()
            .find(|q| q.route_id == route_id)
        {
            queue.complete_train();
        }

        Ok(())
    }

    /// Cancel a train's route
    pub fn cancel_train(&mut self, train_id: TrainId) -> Result<(), QueueError> {
        let assignment = self
            .get_assignment_mut(train_id)
            .ok_or(QueueError::TrainNotFound)?;

        let route_id = assignment.route_id;
        assignment.state = TrainRouteState::Cancelled;

        if let Some(queue) = self
            .route_queues
            .iter_mut()
            .find(|q| q.route_id == route_id)
        {
            queue.cancel_active_train();
            queue.remove_from_queue(train_id).ok();
        }

        Ok(())
    }

    /// Get total trains queued across all routes
    pub fn total_queued(&self) -> usize {
        self.route_queues.iter().map(|q| q.queue_len()).sum()
    }

    /// Get total active trains
    pub fn total_active(&self) -> usize {
        self.route_queues
            .iter()
            .filter(|q| q.active_train.is_some())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_basic() {
        let mut queue = RouteQueue::new(RouteId(1));
        assert!(queue.enqueue(TrainId(1), 5, 0).is_ok());
        assert_eq!(queue.queue_len(), 1);

        let next = queue.peek_next();
        assert!(next.is_some());
        assert_eq!(next.unwrap().train_id, TrainId(1));
    }

    #[test]
    fn test_queue_priority_ordering() {
        let mut queue = RouteQueue::new(RouteId(1));
        queue.enqueue(TrainId(1), 3, 0).unwrap();
        queue.enqueue(TrainId(2), 7, 1).unwrap();
        queue.enqueue(TrainId(3), 5, 2).unwrap();

        // Should be: 7, 5, 3 (by priority)
        let trains: Vec<_> = queue.queued_trains().iter().map(|t| t.train_id.0).collect();
        assert_eq!(trains, vec![2, 3, 1]);
    }

    #[test]
    fn test_queue_fifo_same_priority() {
        let mut queue = RouteQueue::new(RouteId(1));
        queue.enqueue(TrainId(1), 5, 0).unwrap();
        queue.enqueue(TrainId(2), 5, 1).unwrap();
        queue.enqueue(TrainId(3), 5, 2).unwrap();

        // Should be FIFO for same priority
        assert_eq!(queue.dispatch_next().unwrap(), TrainId(1));
        assert_eq!(queue.dispatch_next().unwrap(), TrainId(2));
        assert_eq!(queue.dispatch_next().unwrap(), TrainId(3));
    }

    #[test]
    fn test_queue_dispatch() {
        let mut queue = RouteQueue::new(RouteId(1));
        queue.enqueue(TrainId(1), 5, 0).unwrap();
        assert_eq!(queue.queue_len(), 1);
        assert!(queue.active_train().is_none());

        let dispatched = queue.dispatch_next().unwrap();
        assert_eq!(dispatched, TrainId(1));
        assert_eq!(queue.queue_len(), 0);
        assert_eq!(queue.active_train(), Some(TrainId(1)));
    }

    #[test]
    fn test_assignment_manager() {
        let mut manager = TrainAssignmentManager::new(ColonyId(1));
        assert!(manager.assign_train(TrainId(1), RouteId(1), 5, 0).is_ok());

        let assignment = manager.get_assignment(TrainId(1));
        assert!(assignment.is_some());
        assert_eq!(assignment.unwrap().state, TrainRouteState::Queued);
    }

    #[test]
    fn test_assignment_duplicate() {
        let mut manager = TrainAssignmentManager::new(ColonyId(1));
        assert!(manager.assign_train(TrainId(1), RouteId(1), 5, 0).is_ok());

        // Try to assign same train again
        let result = manager.assign_train(TrainId(1), RouteId(2), 5, 0);
        assert_eq!(result, Err(QueueError::TrainAlreadyAssigned));
    }

    #[test]
    fn test_assignment_dispatch() {
        let mut manager = TrainAssignmentManager::new(ColonyId(1));
        manager.assign_train(TrainId(1), RouteId(1), 5, 0).unwrap();

        let dispatched = manager.dispatch_next_on_route(RouteId(1), 1).unwrap();
        assert_eq!(dispatched, TrainId(1));

        let assignment = manager.get_assignment(TrainId(1)).unwrap();
        assert_eq!(assignment.state, TrainRouteState::Active);
        assert_eq!(assignment.dispatch_turn, Some(1));
    }

    #[test]
    fn test_multi_route_queueing() {
        let mut manager = TrainAssignmentManager::new(ColonyId(1));

        // Assign trains to different routes
        manager.assign_train(TrainId(1), RouteId(1), 5, 0).unwrap();
        manager.assign_train(TrainId(2), RouteId(1), 3, 0).unwrap();
        manager.assign_train(TrainId(3), RouteId(2), 7, 0).unwrap();

        assert_eq!(manager.total_queued(), 3);

        // Dispatch from route 1 (should get priority 5 first)
        let next = manager.dispatch_next_on_route(RouteId(1), 1).unwrap();
        assert_eq!(next, TrainId(1));

        // Dispatch from route 2
        let next = manager.dispatch_next_on_route(RouteId(2), 1).unwrap();
        assert_eq!(next, TrainId(3));

        assert_eq!(manager.total_active(), 2);
        assert_eq!(manager.total_queued(), 1);
    }

    #[test]
    fn test_assignment_completion() {
        let mut manager = TrainAssignmentManager::new(ColonyId(1));
        manager.assign_train(TrainId(1), RouteId(1), 5, 0).unwrap();
        manager.dispatch_next_on_route(RouteId(1), 1).unwrap();

        assert!(manager.complete_train(TrainId(1)).is_ok());
        let assignment = manager.get_assignment(TrainId(1)).unwrap();
        assert_eq!(assignment.state, TrainRouteState::Completed);
    }

    #[test]
    fn test_queued_train_waiting_time() {
        let assignment = TrainAssignment::new(TrainId(1), RouteId(1), 5);
        assert_eq!(assignment.waiting_for_turns(5), 0);
        assert_eq!(assignment.waiting_for_turns(10), 5);
        assert_eq!(assignment.waiting_for_turns(15), 10);
    }

    #[test]
    fn test_invalid_priority() {
        let mut queue = RouteQueue::new(RouteId(1));
        let result = queue.enqueue(TrainId(1), 15, 0);
        assert_eq!(result, Err(QueueError::InvalidPriority));
    }
}
