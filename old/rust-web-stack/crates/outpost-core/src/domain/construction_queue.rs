//! Construction queue — manages building construction jobs.
//!
//! Construction jobs track in-progress building construction at a site.
//! Jobs progress each tick based on available labor and resources.

use crate::domain::ids::{BuildingId, SiteId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A construction job in progress.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConstructionJob {
    /// Unique identifier for this construction job.
    pub id: BuildingId, // The building ID for the building being constructed
    /// Site where construction is taking place.
    pub site_id: SiteId,
    /// Building definition ID (references content/buildings.yaml).
    pub building_def_id: String,
    /// Construction progress in ticks (0 to construction_time_ticks).
    pub progress_ticks: u64,
    /// Total ticks required for completion (from building definition).
    pub total_ticks_required: u64,
    /// Resources committed when construction started.
    pub resources_committed: HashMap<String, f64>,
    /// When construction started (game tick).
    pub started_at_tick: u64,
    /// Whether construction is paused.
    pub paused: bool,
    /// Workers currently assigned to this job.
    pub workers_assigned: u32,
}

impl ConstructionJob {
    /// Create a new construction job.
    pub fn new(
        site_id: SiteId,
        building_def_id: String,
        total_ticks_required: u64,
        resources_committed: HashMap<String, f64>,
        started_at_tick: u64,
    ) -> Self {
        Self {
            id: BuildingId::new(), // Generate new UUID for the building being constructed
            site_id,
            building_def_id,
            progress_ticks: 0,
            total_ticks_required,
            resources_committed,
            started_at_tick,
            paused: false,
            workers_assigned: 0,
        }
    }

    /// Get construction progress as a percentage (0-100).
    pub fn progress_percentage(&self) -> u8 {
        if self.total_ticks_required == 0 {
            return 100;
        }
        let percent = (self.progress_ticks as f64 / self.total_ticks_required as f64) * 100.0;
        percent.min(100.0) as u8
    }

    /// Check if construction is complete.
    pub fn is_complete(&self) -> bool {
        self.progress_ticks >= self.total_ticks_required
    }

    /// Advance construction progress by a certain amount based on available labor.
    /// Returns the actual progress made (may be less than requested if near completion).
    pub fn advance_progress(&mut self, labor_available: u32, efficiency: f64) -> u64 {
        if self.paused || self.is_complete() {
            return 0;
        }

        // Progress = labor * efficiency
        let progress_amount = (labor_available as f64 * efficiency).round() as u64;
        
        // Don't overshoot completion
        let remaining = self.total_ticks_required.saturating_sub(self.progress_ticks);
        let actual_progress = progress_amount.min(remaining);
        
        self.progress_ticks += actual_progress;
        actual_progress
    }

    /// Pause construction.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume construction.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Toggle pause state.
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Calculate partial refund on cancellation (returns a percentage of committed resources).
    /// waste_percentage is how much is lost (0.0-1.0).
    pub fn calculate_refund(&self, waste_percentage: f64) -> HashMap<String, f64> {
        let refund_multiplier = 1.0 - waste_percentage.clamp(0.0, 1.0);
        
        // Return proportion based on how much work is NOT done
        let work_remaining_ratio = 
            (self.total_ticks_required.saturating_sub(self.progress_ticks) as f64) 
            / (self.total_ticks_required as f64);
        
        let total_refund_multiplier = work_remaining_ratio * refund_multiplier;
        
        self.resources_committed
            .iter()
            .map(|(resource, amount)| {
                (resource.clone(), amount * total_refund_multiplier)
            })
            .collect()
    }

    /// Get estimated ticks remaining.
    pub fn ticks_remaining(&self) -> u64 {
        self.total_ticks_required.saturating_sub(self.progress_ticks)
    }
}

/// A queue of construction jobs for a site.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConstructionQueue {
    /// Queue of construction jobs (ordered).
    jobs: Vec<ConstructionJob>,
}

impl ConstructionQueue {
    /// Create a new empty construction queue.
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    /// Add a construction job to the queue.
    pub fn enqueue(&mut self, job: ConstructionJob) {
        self.jobs.push(job);
    }

    /// Remove a job from the queue by building ID.
    pub fn remove(&mut self, building_id: &BuildingId) -> Option<ConstructionJob> {
        if let Some(pos) = self.jobs.iter().position(|j| &j.id == building_id) {
            Some(self.jobs.remove(pos))
        } else {
            None
        }
    }

    /// Get a job by building ID.
    pub fn get(&self, building_id: &BuildingId) -> Option<&ConstructionJob> {
        self.jobs.iter().find(|j| &j.id == building_id)
    }

    /// Get a mutable job by building ID.
    pub fn get_mut(&mut self, building_id: &BuildingId) -> Option<&mut ConstructionJob> {
        self.jobs.iter_mut().find(|j| &j.id == building_id)
    }

    /// Get all jobs in the queue.
    pub fn all_jobs(&self) -> &[ConstructionJob] {
        &self.jobs
    }

    /// Get number of jobs in queue.
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    /// Check if queue is empty.
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// Get all active (non-paused) jobs.
    pub fn active_jobs(&self) -> impl Iterator<Item = &ConstructionJob> {
        self.jobs.iter().filter(|j| !j.paused)
    }

    /// Get all paused jobs.
    pub fn paused_jobs(&self) -> impl Iterator<Item = &ConstructionJob> {
        self.jobs.iter().filter(|j| j.paused)
    }

    /// Get all completed jobs (progress >= 100%).
    pub fn completed_jobs(&self) -> Vec<&ConstructionJob> {
        self.jobs.iter().filter(|j| j.is_complete()).collect()
    }

    /// Remove all completed jobs and return them.
    pub fn remove_completed(&mut self) -> Vec<ConstructionJob> {
        let mut completed = Vec::new();
        self.jobs.retain(|job| {
            if job.is_complete() {
                completed.push(job.clone());
                false // Remove from queue
            } else {
                true // Keep in queue
            }
        });
        completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_resources() -> HashMap<String, f64> {
        let mut resources = HashMap::new();
        resources.insert("steel".to_string(), 100.0);
        resources.insert("electronics".to_string(), 50.0);
        resources
    }

    #[test]
    fn test_construction_job_creation() {
        let site_id = SiteId::new();
        let resources = create_test_resources();
        
        let job = ConstructionJob::new(
            site_id,
            "iron_mine".to_string(),
            200,
            resources.clone(),
            0,
        );

        assert_eq!(job.building_def_id, "iron_mine");
        assert_eq!(job.progress_ticks, 0);
        assert_eq!(job.total_ticks_required, 200);
        assert_eq!(job.progress_percentage(), 0);
        assert!(!job.is_complete());
        assert!(!job.paused);
        assert_eq!(job.resources_committed.get("steel"), Some(&100.0));
    }

    #[test]
    fn test_construction_progress() {
        let site_id = SiteId::new();
        let resources = create_test_resources();
        
        let mut job = ConstructionJob::new(
            site_id,
            "iron_mine".to_string(),
            100,
            resources,
            0,
        );

        // Advance by 10 ticks with 5 workers at 1.0 efficiency
        let progress = job.advance_progress(5, 1.0);
        assert_eq!(progress, 5);
        assert_eq!(job.progress_ticks, 5);
        assert_eq!(job.progress_percentage(), 5);

        // Advance more
        job.advance_progress(10, 1.0);
        assert_eq!(job.progress_ticks, 15);
        assert_eq!(job.progress_percentage(), 15);
    }

    #[test]
    fn test_construction_completion() {
        let site_id = SiteId::new();
        let resources = create_test_resources();
        
        let mut job = ConstructionJob::new(
            site_id,
            "iron_mine".to_string(),
            10,
            resources,
            0,
        );

        // Advance to completion
        job.advance_progress(20, 1.0); // Request more than needed
        assert_eq!(job.progress_ticks, 10); // Should cap at total required
        assert_eq!(job.progress_percentage(), 100);
        assert!(job.is_complete());

        // Further progress should do nothing
        let additional = job.advance_progress(5, 1.0);
        assert_eq!(additional, 0);
        assert_eq!(job.progress_ticks, 10);
    }

    #[test]
    fn test_construction_pause_resume() {
        let site_id = SiteId::new();
        let resources = create_test_resources();
        
        let mut job = ConstructionJob::new(
            site_id,
            "iron_mine".to_string(),
            100,
            resources,
            0,
        );

        // Advance progress
        job.advance_progress(10, 1.0);
        assert_eq!(job.progress_ticks, 10);

        // Pause
        job.pause();
        assert!(job.paused);
        
        // Paused job should not progress
        let progress = job.advance_progress(10, 1.0);
        assert_eq!(progress, 0);
        assert_eq!(job.progress_ticks, 10); // No change

        // Resume
        job.resume();
        assert!(!job.paused);
        job.advance_progress(10, 1.0);
        assert_eq!(job.progress_ticks, 20);
    }

    #[test]
    fn test_refund_calculation() {
        let site_id = SiteId::new();
        let mut resources = HashMap::new();
        resources.insert("steel".to_string(), 100.0);
        
        let mut job = ConstructionJob::new(
            site_id,
            "iron_mine".to_string(),
            100,
            resources,
            0,
        );

        // No progress = full refund (minus waste)
        let refund = job.calculate_refund(0.2); // 20% waste
        assert_eq!(refund.get("steel"), Some(&80.0)); // 100 * 1.0 * 0.8

        // 50% complete
        job.progress_ticks = 50;
        let refund = job.calculate_refund(0.2);
        // 50% work remaining * 80% refund = 40% of original
        assert_eq!(refund.get("steel"), Some(&40.0));

        // 100% complete = no refund
        job.progress_ticks = 100;
        let refund = job.calculate_refund(0.2);
        assert_eq!(refund.get("steel"), Some(&0.0));
    }

    #[test]
    fn test_construction_queue_operations() {
        let mut queue = ConstructionQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        let site_id = SiteId::new();
        let resources = create_test_resources();

        // Enqueue job
        let job1 = ConstructionJob::new(
            site_id,
            "iron_mine".to_string(),
            100,
            resources.clone(),
            0,
        );
        let job1_id = job1.id;
        queue.enqueue(job1);

        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());

        // Get job
        let retrieved = queue.get(&job1_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().building_def_id, "iron_mine");

        // Enqueue another job
        let job2 = ConstructionJob::new(
            site_id,
            "smelter".to_string(),
            200,
            resources,
            0,
        );
        queue.enqueue(job2);
        assert_eq!(queue.len(), 2);

        // Remove job
        let removed = queue.remove(&job1_id);
        assert!(removed.is_some());
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_queue_completed_jobs() {
        let mut queue = ConstructionQueue::new();
        let site_id = SiteId::new();
        let resources = create_test_resources();

        // Add two jobs
        let mut job1 = ConstructionJob::new(
            site_id,
            "iron_mine".to_string(),
            10,
            resources.clone(),
            0,
        );
        job1.progress_ticks = 10; // Complete
        
        let job2 = ConstructionJob::new(
            site_id,
            "smelter".to_string(),
            20,
            resources,
            0,
        );
        // job2 incomplete

        queue.enqueue(job1);
        queue.enqueue(job2);

        // Check completed
        let completed = queue.completed_jobs();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].building_def_id, "iron_mine");

        // Remove completed
        let removed = queue.remove_completed();
        assert_eq!(removed.len(), 1);
        assert_eq!(queue.len(), 1); // Only incomplete job remains
        assert_eq!(queue.all_jobs()[0].building_def_id, "smelter");
    }
}
