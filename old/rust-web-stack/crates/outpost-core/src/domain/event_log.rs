//! EventLog — ordered log of fired gameplay events, distinct from the event-sourcing store.
//!
//! Records which narrative/gameplay events fired (e.g., "Equipment Failure on Titan Base"),
//! when, and their severity. Used by the UI ticker and the `/events` page.

use crate::content::{EventCategory, EventSeverity};
use crate::domain::SiteId;
use serde::{Deserialize, Serialize};

/// A single entry in the gameplay event log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FiredEvent {
    /// Auto-incremented log entry index (ordering key).
    pub index: u64,
    /// Tick at which the event fired.
    pub tick: u64,
    /// Site where the event occurred.
    pub site_id: SiteId,
    /// Definition ID of the fired event.
    pub event_id: String,
    /// Display title (copied from EventDefinition so log is self-contained).
    pub title: String,
    /// Category for filtering.
    pub category: EventCategory,
    /// Severity for colour-coding and auto-pause logic.
    pub severity: EventSeverity,
    /// Whether the event required a player choice.
    pub required_choice: bool,
    /// The choice ID the player selected, if resolved.
    pub resolved_choice_id: Option<String>,
}

/// Ordered, append-only log of all gameplay events that have fired during the session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventLog {
    entries: Vec<FiredEvent>,
    next_index: u64,
}

impl EventLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a new entry and return its assigned index.
    pub fn push(
        &mut self,
        tick: u64,
        site_id: SiteId,
        event_id: String,
        title: String,
        category: EventCategory,
        severity: EventSeverity,
        required_choice: bool,
    ) -> u64 {
        let index = self.next_index;
        self.next_index += 1;
        self.entries.push(FiredEvent {
            index,
            tick,
            site_id,
            event_id,
            title,
            category,
            severity,
            required_choice,
            resolved_choice_id: None,
        });
        index
    }

    /// Mark a previously-pushed entry as resolved with the given choice.
    pub fn resolve(&mut self, index: u64, choice_id: String) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.index == index) {
            entry.resolved_choice_id = Some(choice_id);
        }
    }

    /// All entries in chronological order (oldest first).
    pub fn all(&self) -> &[FiredEvent] {
        &self.entries
    }

    /// Most recent `n` entries (newest first).
    pub fn recent(&self, n: usize) -> Vec<&FiredEvent> {
        self.entries.iter().rev().take(n).collect()
    }

    /// Filter by category.
    pub fn by_category(&self, category: &EventCategory) -> Vec<&FiredEvent> {
        self.entries.iter().filter(|e| &e.category == category).collect()
    }

    /// Filter by severity.
    pub fn by_severity(&self, severity: &EventSeverity) -> Vec<&FiredEvent> {
        self.entries.iter().filter(|e| &e.severity == severity).collect()
    }

    /// Count of unresolved entries that required a choice.
    pub fn unresolved_choices(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.required_choice && e.resolved_choice_id.is_none())
            .count()
    }

    /// Count unread entries by severity (for badge display).
    pub fn count_by_severity(&self, severity: &EventSeverity) -> usize {
        self.entries.iter().filter(|e| &e.severity == severity).count()
    }

    /// Total number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove an entry by its index value.
    pub fn remove_by_index(&mut self, index: u64) {
        self.entries.retain(|e| e.index != index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::SiteId;

    fn site() -> SiteId {
        SiteId::new()
    }

    fn push_entry(log: &mut EventLog, tick: u64, category: EventCategory, severity: EventSeverity) -> u64 {
        log.push(
            tick,
            site(),
            "test_event".into(),
            "Test Event".into(),
            category,
            severity,
            false,
        )
    }

    #[test]
    fn test_push_increments_index() {
        let mut log = EventLog::new();
        let i0 = push_entry(&mut log, 1, EventCategory::General, EventSeverity::Info);
        let i1 = push_entry(&mut log, 2, EventCategory::General, EventSeverity::Info);
        assert_eq!(i0, 0);
        assert_eq!(i1, 1);
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn test_all_returns_chronological_order() {
        let mut log = EventLog::new();
        push_entry(&mut log, 10, EventCategory::Disaster, EventSeverity::Critical);
        push_entry(&mut log, 20, EventCategory::Social,   EventSeverity::Info);
        push_entry(&mut log, 30, EventCategory::Economic, EventSeverity::Warning);

        let all = log.all();
        assert_eq!(all[0].tick, 10);
        assert_eq!(all[1].tick, 20);
        assert_eq!(all[2].tick, 30);
    }

    #[test]
    fn test_recent_returns_newest_first() {
        let mut log = EventLog::new();
        push_entry(&mut log, 1, EventCategory::General, EventSeverity::Info);
        push_entry(&mut log, 2, EventCategory::General, EventSeverity::Info);
        push_entry(&mut log, 3, EventCategory::General, EventSeverity::Info);

        let recent = log.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].tick, 3); // newest first
        assert_eq!(recent[1].tick, 2);
    }

    #[test]
    fn test_recent_returns_all_when_fewer_than_n() {
        let mut log = EventLog::new();
        push_entry(&mut log, 1, EventCategory::General, EventSeverity::Info);

        let recent = log.recent(10);
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn test_by_category_filter() {
        let mut log = EventLog::new();
        push_entry(&mut log, 1, EventCategory::Disaster,  EventSeverity::Critical);
        push_entry(&mut log, 2, EventCategory::Discovery, EventSeverity::Info);
        push_entry(&mut log, 3, EventCategory::Disaster,  EventSeverity::Warning);

        let disasters = log.by_category(&EventCategory::Disaster);
        assert_eq!(disasters.len(), 2);
        let discoveries = log.by_category(&EventCategory::Discovery);
        assert_eq!(discoveries.len(), 1);
    }

    #[test]
    fn test_by_severity_filter() {
        let mut log = EventLog::new();
        push_entry(&mut log, 1, EventCategory::General, EventSeverity::Info);
        push_entry(&mut log, 2, EventCategory::General, EventSeverity::Warning);
        push_entry(&mut log, 3, EventCategory::General, EventSeverity::Critical);
        push_entry(&mut log, 4, EventCategory::General, EventSeverity::Critical);

        assert_eq!(log.by_severity(&EventSeverity::Critical).len(), 2);
        assert_eq!(log.by_severity(&EventSeverity::Warning).len(), 1);
        assert_eq!(log.by_severity(&EventSeverity::Info).len(), 1);
    }

    #[test]
    fn test_resolve_marks_choice() {
        let mut log = EventLog::new();
        let site_id = site();
        let idx = log.push(
            5, site_id, "choice_event".into(), "Choice Event".into(),
            EventCategory::General, EventSeverity::Warning, true,
        );

        assert_eq!(log.unresolved_choices(), 1);
        log.resolve(idx, "option_a".into());
        assert_eq!(log.unresolved_choices(), 0);
        assert_eq!(
            log.all()[0].resolved_choice_id.as_deref(),
            Some("option_a")
        );
    }

    #[test]
    fn test_count_by_severity() {
        let mut log = EventLog::new();
        push_entry(&mut log, 1, EventCategory::General, EventSeverity::Critical);
        push_entry(&mut log, 2, EventCategory::General, EventSeverity::Critical);
        push_entry(&mut log, 3, EventCategory::General, EventSeverity::Warning);

        assert_eq!(log.count_by_severity(&EventSeverity::Critical), 2);
        assert_eq!(log.count_by_severity(&EventSeverity::Warning), 1);
        assert_eq!(log.count_by_severity(&EventSeverity::Info), 0);
    }

    #[test]
    fn test_empty_log() {
        let log = EventLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
        assert_eq!(log.recent(5).len(), 0);
        assert_eq!(log.unresolved_choices(), 0);
    }
}
