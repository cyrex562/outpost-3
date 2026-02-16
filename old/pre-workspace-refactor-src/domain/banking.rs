use serde::{Deserialize, Serialize};
use tracing::info;

use crate::domain::ColonyId;
use crate::events::EventType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoanId(pub u64);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loan {
    pub id: LoanId,
    pub colony_id: ColonyId,
    pub principal: f64,
    pub interest_rate_pct: f64,
    pub term_turns: u32,
    pub remaining_principal: f64,
    pub payments_made: u32,
}

impl Loan {
    pub fn issue(
        loan_id: LoanId,
        colony_id: ColonyId,
        principal: f64,
        interest_rate_pct: f64,
        term_turns: u32,
    ) -> (Self, EventType) {
        let loan = Self {
            id: loan_id,
            colony_id,
            principal,
            interest_rate_pct,
            term_turns,
            remaining_principal: principal,
            payments_made: 0,
        };
        info!("Loan issued: {} @ {}%", principal, interest_rate_pct);
        let event = EventType::LoanIssued {
            loan_id,
            colony_id,
            principal,
            interest_rate: interest_rate_pct,
            term_turns,
        };
        (loan, event)
    }

    pub fn rate_per_turn(&self) -> f64 {
        self.interest_rate_pct / 100.0
    }

    pub fn amortized_payment(&self) -> f64 {
        amortized_payment(self.principal, self.interest_rate_pct, self.term_turns)
    }

    pub fn apply_payment(&mut self, amount: f64) -> Vec<EventType> {
        let mut events = Vec::new();
        if amount <= 0.0 {
            return events;
        }

        let interest_due = self.remaining_principal * self.rate_per_turn();
        let interest_paid = amount.min(interest_due).max(0.0);
        let principal_paid = (amount - interest_paid)
            .max(0.0)
            .min(self.remaining_principal);

        self.remaining_principal -= principal_paid;
        self.payments_made += 1;

        events.push(EventType::LoanPaymentMade {
            loan_id: self.id,
            colony_id: self.colony_id,
            amount,
            principal_paid,
            interest_paid,
            remaining_principal: self.remaining_principal,
        });

        if self.remaining_principal <= 1e-6 {
            self.remaining_principal = 0.0;
            events.push(EventType::LoanPaidOff {
                loan_id: self.id,
                colony_id: self.colony_id,
            });
        }

        events
    }
}

pub fn amortized_payment(principal: f64, interest_rate_pct: f64, term_turns: u32) -> f64 {
    if term_turns == 0 {
        return 0.0;
    }

    let n = term_turns as f64;
    let r = interest_rate_pct / 100.0;
    if r.abs() < f64::EPSILON {
        principal / n
    } else {
        let numerator = principal * r;
        let denominator = 1.0 - (1.0 + r).powf(-n);
        numerator / denominator
    }
}
