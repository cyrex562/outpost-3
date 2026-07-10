//! Directive system — automated per-colony action rules.
//!
//! A [`Directive`] binds a [`Predicate`] to a [`Command`]: when the predicate
//! is true for a colony, the command fires automatically via the drive API.
//!
//! Directives are managed through a [`DirectiveStore`] which is embedded in
//! `GameState`. Per-turn evaluation (see `GameEngine::apply`):
//! 1. Colonies in the manual-override set skip directive evaluation entirely.
//! 2. For each remaining colony, all directives targeting that colony are
//!    evaluated; the highest-priority matching directive fires its action.
//!
//! See `docs/DESIGN.md §5, §12, §12A, §14`.

use std::collections::HashSet;

use uuid::Uuid;

use crate::colony::ColonyId;
use crate::predicate::{Predicate, PredicateContext};
use crate::Command;

/// Stable identifier for a [`Directive`].
pub type DirectiveId = Uuid;

/// A single automation rule: when `predicate` is true for `colony_id`, fire `action`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Directive {
    /// Stable identifier for this directive.
    pub id: DirectiveId,
    /// The colony this directive targets.
    pub colony_id: ColonyId,
    /// Condition that must hold for the action to fire.
    pub predicate: Predicate,
    /// Command to execute via the drive interface when the predicate matches.
    pub action: Command,
    /// Evaluation priority — higher value = checked first; ties broken by insertion order.
    pub priority: u8,
}

impl Directive {
    /// Create a new directive with a freshly generated UUID.
    #[must_use]
    pub fn new(
        colony_id: ColonyId,
        predicate: Predicate,
        action: Command,
        priority: u8,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            colony_id,
            predicate,
            action,
            priority,
        }
    }
}

/// Stores all active directives and the manual-override registry.
///
/// Embedded in `GameState`; persisted as part of the SQLite snapshot.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DirectiveStore {
    /// All active directives, ordered by insertion time.
    pub directives: Vec<Directive>,
    /// Colonies where manual override is active (directive evaluation skipped).
    pub manual_override: HashSet<ColonyId>,
}

impl DirectiveStore {
    /// Register a directive, replacing any existing one with the same ID.
    pub fn set_directive(&mut self, directive: Directive) {
        if let Some(existing) = self.directives.iter_mut().find(|d| d.id == directive.id) {
            *existing = directive;
        } else {
            self.directives.push(directive);
        }
    }

    /// Remove a directive by ID. No-op if the ID is not found.
    pub fn remove_directive(&mut self, id: DirectiveId) {
        self.directives.retain(|d| d.id != id);
    }

    /// Enable or disable manual override for a colony.
    pub fn set_manual_override(&mut self, colony_id: ColonyId, enabled: bool) {
        if enabled {
            self.manual_override.insert(colony_id);
        } else {
            self.manual_override.remove(&colony_id);
        }
    }

    /// Return `true` if the colony is currently in manual-override mode.
    #[must_use]
    pub fn is_manual_override(&self, colony_id: ColonyId) -> bool {
        self.manual_override.contains(&colony_id)
    }

    /// Evaluate all directives for `colony_id` against `ctx`.
    ///
    /// Returns a reference to the action of the highest-priority matching
    /// directive, or `None` if the colony is under manual override or no
    /// directive matches.
    #[must_use]
    pub fn evaluate_for_colony<'a>(
        &'a self,
        colony_id: ColonyId,
        ctx: &PredicateContext,
    ) -> Option<&'a Command> {
        if self.manual_override.contains(&colony_id) {
            return None;
        }
        self.directives
            .iter()
            .filter(|d| d.colony_id == colony_id)
            .filter(|d| d.predicate.evaluate(ctx))
            .max_by_key(|d| d.priority)
            .map(|d| &d.action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicate::{Metric, Predicate};

    fn make_ctx(colony_id: ColonyId) -> PredicateContext {
        PredicateContext {
            colony_id,
            population: 100.0,
            stability: 0.8,
            available_labour: 80.0,
            system_research: 0.0,
            sol: 1,
            month: 0,
        }
    }

    #[test]
    fn directive_new_generates_unique_ids() {
        let id = Uuid::new_v4();
        let d1 = Directive::new(id, Predicate::Always, Command::AdvanceColonySol, 0);
        let d2 = Directive::new(id, Predicate::Always, Command::AdvanceColonySol, 0);
        assert_ne!(d1.id, d2.id);
    }

    #[test]
    fn set_directive_inserts_new() {
        let mut store = DirectiveStore::default();
        let col = Uuid::new_v4();
        let d = Directive::new(col, Predicate::Always, Command::AdvanceColonySol, 5);
        store.set_directive(d.clone());
        assert_eq!(store.directives.len(), 1);
        assert_eq!(store.directives[0].id, d.id);
    }

    #[test]
    fn set_directive_replaces_existing() {
        let mut store = DirectiveStore::default();
        let col = Uuid::new_v4();
        let d = Directive::new(col, Predicate::Always, Command::AdvanceColonySol, 5);
        let id = d.id;
        store.set_directive(d);
        let d2 = Directive {
            id,
            colony_id: col,
            predicate: Predicate::Never,
            action: Command::AdvanceStrategicMonth,
            priority: 10,
        };
        store.set_directive(d2);
        assert_eq!(store.directives.len(), 1);
        assert_eq!(store.directives[0].priority, 10);
    }

    #[test]
    fn remove_directive_removes_by_id() {
        let mut store = DirectiveStore::default();
        let col = Uuid::new_v4();
        let d = Directive::new(col, Predicate::Always, Command::AdvanceColonySol, 0);
        let id = d.id;
        store.set_directive(d);
        store.remove_directive(id);
        assert!(store.directives.is_empty());
    }

    #[test]
    fn manual_override_suppresses_evaluation() {
        let mut store = DirectiveStore::default();
        let col = Uuid::new_v4();
        let d = Directive::new(col, Predicate::Always, Command::AdvanceColonySol, 0);
        store.set_directive(d);
        store.set_manual_override(col, true);
        let ctx = make_ctx(col);
        assert!(store.evaluate_for_colony(col, &ctx).is_none());
    }

    #[test]
    fn evaluate_fires_always_predicate() {
        let mut store = DirectiveStore::default();
        let col = Uuid::new_v4();
        let d = Directive::new(col, Predicate::Always, Command::AdvanceColonySol, 0);
        store.set_directive(d);
        let ctx = make_ctx(col);
        assert!(store.evaluate_for_colony(col, &ctx).is_some());
    }

    #[test]
    fn evaluate_returns_none_for_never_predicate() {
        let mut store = DirectiveStore::default();
        let col = Uuid::new_v4();
        let d = Directive::new(col, Predicate::Never, Command::AdvanceColonySol, 0);
        store.set_directive(d);
        let ctx = make_ctx(col);
        assert!(store.evaluate_for_colony(col, &ctx).is_none());
    }

    #[test]
    fn evaluate_fires_highest_priority() {
        let mut store = DirectiveStore::default();
        let col = Uuid::new_v4();
        let low = Directive {
            id: Uuid::new_v4(),
            colony_id: col,
            predicate: Predicate::Always,
            action: Command::AdvanceStrategicMonth,
            priority: 1,
        };
        let high = Directive {
            id: Uuid::new_v4(),
            colony_id: col,
            predicate: Predicate::Always,
            action: Command::AdvanceColonySol,
            priority: 10,
        };
        store.set_directive(low);
        store.set_directive(high);
        let ctx = make_ctx(col);
        let fired = store.evaluate_for_colony(col, &ctx).unwrap();
        assert!(matches!(fired, Command::AdvanceColonySol));
    }

    #[test]
    fn evaluate_conditional_predicate() {
        let mut store = DirectiveStore::default();
        let col = Uuid::new_v4();
        // Fires when stability < 0.5
        let d = Directive::new(
            col,
            Predicate::lt(Metric::Stability, 0.5),
            Command::AdvanceColonySol,
            0,
        );
        store.set_directive(d);

        // stability = 0.8 → no fire
        let ctx_high = make_ctx(col);
        assert!(store.evaluate_for_colony(col, &ctx_high).is_none());

        // stability = 0.3 → fires
        let ctx_low = PredicateContext {
            colony_id: col,
            stability: 0.3,
            ..make_ctx(col)
        };
        assert!(store.evaluate_for_colony(col, &ctx_low).is_some());
    }

    #[test]
    fn is_manual_override_reflects_state() {
        let mut store = DirectiveStore::default();
        let col = Uuid::new_v4();
        assert!(!store.is_manual_override(col));
        store.set_manual_override(col, true);
        assert!(store.is_manual_override(col));
        store.set_manual_override(col, false);
        assert!(!store.is_manual_override(col));
    }
}
