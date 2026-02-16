use anyhow::Result;
use tracing::info;

use crate::domain::{calculate_snapshot, ColonyId};
use crate::events::{EventStore, EventType, GameEvent};

pub struct TurnProcessor {
    event_store: EventStore,
}

#[derive(Debug, Default)]
struct FinancialSummary {
    market_price_changes: usize,
    trades: usize,
    loan_events: usize,
    economy_snapshots: usize,
    turn_advances: usize,
}

impl FinancialSummary {
    fn record(&mut self, event: &EventType) {
        match event {
            EventType::MarketPriceChanged { .. } => self.market_price_changes += 1,
            EventType::ResourceTraded { .. } => self.trades += 1,
            EventType::LoanIssued { .. }
            | EventType::LoanPaymentMade { .. }
            | EventType::LoanPaidOff { .. } => self.loan_events += 1,
            EventType::EconomySnapshot { .. } => self.economy_snapshots += 1,
            EventType::TurnAdvanced { .. } => self.turn_advances += 1,
            _ => {}
        }
    }
}

impl std::fmt::Display for FinancialSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{market_price_changes: {}, trades: {}, loan_events: {}, economy_snapshots: {}, turn_advances: {}}}",
            self.market_price_changes,
            self.trades,
            self.loan_events,
            self.economy_snapshots,
            self.turn_advances
        )
    }
}

impl TurnProcessor {
    pub fn new(event_store: EventStore) -> Self {
        Self { event_store }
    }

    pub fn process_turn(
        &self,
        turn_number: u64,
        colony_financials: &[(ColonyId, i64, i64)],
    ) -> Result<Vec<GameEvent>> {
        let mut events = vec![];

        for (colony_id, credits_before, credits_after) in colony_financials {
            let (_, event) = calculate_snapshot(*colony_id, *credits_before, *credits_after);
            events.push(event);
        }

        events.push(EventType::TurnAdvanced {
            turn_number: turn_number + 1,
        });

        let mut next_id = self.event_store.get_next_event_id()?;
        let mut summary = FinancialSummary::default();
        let mut persisted = Vec::with_capacity(events.len());

        for event_type in events {
            summary.record(&event_type);
            let event_turn = match &event_type {
                EventType::TurnAdvanced { turn_number } => *turn_number,
                _ => turn_number,
            };
            let event = GameEvent::new(next_id, event_turn, event_type.clone());
            self.event_store.save_event(&event)?;
            persisted.push(event);
            next_id += 1;
        }

        info!("Turn {} financial summary: {}", turn_number, summary);

        Ok(persisted)
    }
}
