//! In-process domain event dispatcher.
//!
//! Ported from `src/harsh_realm/gm/domain_events.py`. The Python version is
//! async-capable; the Rust port is synchronous (all ported handlers are sync),
//! preserving the same bounded-cascade semantics.

use std::collections::HashMap;

use crate::events::GameEvent;

const MAX_CASCADE_DEPTH: u32 = 10;

/// A domain handler: receives one event and returns any follow-on events.
///
/// The `'h` lifetime lets handlers borrow subsystem services (which themselves
/// borrow a database) for as long as the dispatcher lives.
pub type DomainHandlerFn<'h> = Box<dyn Fn(&GameEvent) -> Vec<GameEvent> + 'h>;

/// Dispatch domain events through registered subscribers.
pub struct DomainEventDispatcher<'h> {
    handlers: HashMap<String, Vec<DomainHandlerFn<'h>>>,
    max_cascade_depth: u32,
}

impl Default for DomainEventDispatcher<'_> {
    fn default() -> Self {
        Self::new(MAX_CASCADE_DEPTH)
    }
}

impl<'h> DomainEventDispatcher<'h> {
    /// Create a dispatcher with an explicit cascade depth limit.
    pub fn new(max_cascade_depth: u32) -> Self {
        DomainEventDispatcher {
            handlers: HashMap::new(),
            max_cascade_depth,
        }
    }

    /// Register a handler for a specific event type.
    pub fn subscribe(&mut self, event_type: &str, handler: DomainHandlerFn<'h>) {
        self.handlers
            .entry(event_type.to_string())
            .or_default()
            .push(handler);
    }

    /// Dispatch an event and return the full domain cascade (including `event`).
    pub fn dispatch(&self, event: GameEvent) -> Vec<GameEvent> {
        let mut accumulator: Vec<GameEvent> = Vec::new();
        self.dispatch_inner(event, 0, &mut accumulator);
        accumulator
    }

    fn dispatch_inner(&self, event: GameEvent, depth: u32, accumulator: &mut Vec<GameEvent>) {
        if depth > self.max_cascade_depth {
            // Mirrors the bounded cascade in [`crate::events::EventBus`]: drop
            // events beyond the depth limit rather than risk an infinite loop.
            let _ = &event;
            return;
        }

        accumulator.push(event.clone());

        let Some(handlers) = self.handlers.get(&event.event_type) else {
            return;
        };
        for handler in handlers {
            let children = handler(&event);
            for child in children {
                self.dispatch_inner(child, depth + 1, accumulator);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn ev(kind: &str) -> GameEvent {
        GameEvent::new(0, kind, crate::runtime::JsonObject::new())
    }

    #[test]
    fn dispatch_collects_cascade() {
        let mut d = DomainEventDispatcher::default();
        d.subscribe("a", Box::new(|_| vec![ev("b")]));
        d.subscribe("b", Box::new(|_| vec![ev("c")]));
        let events = d.dispatch(ev("a"));
        let kinds: Vec<&str> = events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(kinds, vec!["a", "b", "c"]);
    }

    #[test]
    fn multiple_handlers_run_in_order() {
        let mut d = DomainEventDispatcher::default();
        let counter = Rc::new(Cell::new(0));
        let c1 = counter.clone();
        d.subscribe(
            "x",
            Box::new(move |_| {
                c1.set(c1.get() + 1);
                vec![]
            }),
        );
        let c2 = counter.clone();
        d.subscribe(
            "x",
            Box::new(move |_| {
                c2.set(c2.get() + 1);
                vec![]
            }),
        );
        d.dispatch(ev("x"));
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn cascade_depth_is_bounded() {
        let mut d = DomainEventDispatcher::new(3);
        d.subscribe("loop", Box::new(|_| vec![ev("loop")]));
        // Would be infinite without the bound; ensure it terminates.
        let events = d.dispatch(ev("loop"));
        assert!(events.len() <= 5);
    }
}
