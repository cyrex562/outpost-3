//! Building model: placed buildings and the construction queue.
//!
//! A colony owns a list of completed [`PlacedBuilding`]s and an ordered
//! [`ConstructionQueue`] of in-progress projects. Build slots limit how many
//! buildings a colony can support; slot capacity is increased by technology.

use uuid::Uuid;

/// Unique identifier for a construction project.
pub type ProjectId = Uuid;

/// A building that has been fully constructed and is operational.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlacedBuilding {
    /// Stable identifier for this placed instance.
    pub id: Uuid,
    /// Content-pack key identifying the building type.
    pub building_type: String,
    /// Number of build slots consumed by this building.
    pub slot_cost: u32,
}

impl PlacedBuilding {
    /// Create a new placed building instance.
    #[must_use]
    pub fn new(building_type: impl Into<String>, slot_cost: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            building_type: building_type.into(),
            slot_cost,
        }
    }
}

/// An in-progress construction project.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstructionProject {
    /// Stable identifier for this project (used for cancellation).
    pub id: ProjectId,
    /// Content-pack key of the building being constructed.
    pub building_type: String,
    /// Number of build slots reserved while under construction.
    pub slot_cost: u32,
    /// Labor units consumed from the colony pool each turn.
    pub labor_per_turn: u32,
    /// Construction cost list for partial-refund calculation.
    pub construction_cost: Vec<(String, f64)>,
    /// Total number of turns required to complete construction.
    pub total_turns: u32,
    /// Turns of construction already completed.
    pub turns_completed: u32,
}

impl ConstructionProject {
    /// Create a new construction project.
    #[must_use]
    pub fn new(
        building_type: impl Into<String>,
        slot_cost: u32,
        labor_per_turn: u32,
        construction_cost: Vec<(String, f64)>,
        total_turns: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            building_type: building_type.into(),
            slot_cost,
            labor_per_turn,
            construction_cost,
            total_turns,
            turns_completed: 0,
        }
    }

    /// Returns `true` if all construction turns are complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.turns_completed >= self.total_turns
    }

    /// Returns the proportion of construction complete, in `[0.0, 1.0]`.
    #[must_use]
    pub fn progress_fraction(&self) -> f64 {
        if self.total_turns == 0 {
            1.0
        } else {
            f64::from(self.turns_completed) / f64::from(self.total_turns)
        }
    }

    /// Compute the 50 % commodity refund for cancelling this project.
    ///
    /// Only costs already spent (proportional to `turns_completed`) are
    /// considered; the refund is 50 % of that amount.
    #[must_use]
    pub fn cancel_refund(&self) -> Vec<(String, f64)> {
        let fraction_spent = self.progress_fraction();
        self.construction_cost
            .iter()
            .map(|(id, qty)| (id.clone(), qty * fraction_spent * 0.5))
            .collect()
    }
}

/// The ordered queue of in-progress construction projects for one colony.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ConstructionQueue {
    /// Projects in arrival order; the first element is the active project.
    pub projects: Vec<ConstructionProject>,
}

impl ConstructionQueue {
    /// Create a new empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Total build slots reserved by all queued projects.
    #[must_use]
    pub fn slots_reserved(&self) -> u32 {
        self.projects.iter().map(|p| p.slot_cost).sum()
    }

    /// Push a project onto the back of the queue.
    pub fn enqueue(&mut self, project: ConstructionProject) {
        self.projects.push(project);
    }

    /// Remove and return the project with the given id, if present.
    pub fn cancel(&mut self, project_id: ProjectId) -> Option<ConstructionProject> {
        if let Some(pos) = self.projects.iter().position(|p| p.id == project_id) {
            Some(self.projects.remove(pos))
        } else {
            None
        }
    }

    /// Advance the active (first) project by one turn; return it if complete.
    ///
    /// Returns `Some(project)` when the project just finished (caller should
    /// add a [`PlacedBuilding`] and pop the queue), or `None` if not yet done.
    pub fn tick_active(&mut self) -> Option<ConstructionProject> {
        let active = self.projects.first_mut()?;
        active.turns_completed += 1;
        if active.is_complete() {
            Some(self.projects.remove(0))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_project(total_turns: u32) -> ConstructionProject {
        ConstructionProject::new(
            "greenhouse",
            1,
            5,
            vec![("steel".into(), 100.0), ("glass".into(), 50.0)],
            total_turns,
        )
    }

    #[test]
    fn project_starts_incomplete() {
        let p = sample_project(3);
        assert!(!p.is_complete());
        assert!((p.progress_fraction() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn project_is_complete_after_all_turns() {
        let mut p = sample_project(2);
        p.turns_completed = 2;
        assert!(p.is_complete());
        assert!((p.progress_fraction() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cancel_refund_is_50_pct_of_spent_cost() {
        let mut p = sample_project(4);
        p.turns_completed = 2; // 50 % done
        let refund = p.cancel_refund();
        // 50 % spent × 50 % refund = 25 % of total cost
        let steel = refund.iter().find(|(id, _)| id == "steel").unwrap();
        assert!((steel.1 - 25.0).abs() < 1e-9);
    }

    #[test]
    fn cancel_refund_zero_when_just_started() {
        let p = sample_project(4); // turns_completed = 0
        let refund = p.cancel_refund();
        for (_, qty) in &refund {
            assert!(*qty < 1e-9);
        }
    }

    #[test]
    fn queue_slots_reserved_sums_all_projects() {
        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(3));
        let mut p2 = sample_project(2);
        p2.slot_cost = 2;
        q.enqueue(p2);
        assert_eq!(q.slots_reserved(), 3);
    }

    #[test]
    fn tick_active_returns_none_before_completion() {
        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(3));
        assert!(q.tick_active().is_none()); // turn 1/3
        assert!(q.tick_active().is_none()); // turn 2/3
    }

    #[test]
    fn tick_active_returns_project_on_completion() {
        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(2));
        q.tick_active(); // turn 1
        let done = q.tick_active(); // turn 2 — complete
        assert!(done.is_some());
        assert!(q.projects.is_empty());
    }

    #[test]
    fn cancel_removes_project_from_queue() {
        let mut q = ConstructionQueue::new();
        let p = sample_project(5);
        let pid = p.id;
        q.enqueue(p);
        let removed = q.cancel(pid);
        assert!(removed.is_some());
        assert!(q.projects.is_empty());
    }

    #[test]
    fn cancel_unknown_id_returns_none() {
        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(3));
        assert!(q.cancel(Uuid::new_v4()).is_none());
    }
}
