use thiserror::Error;

use super::Command;
use crate::domain::{ColonyId, Loan, LoanId};
use crate::events::EventType;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum BankingCommandError {
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TakeLoan {
    pub loan_id: LoanId,
    pub colony_id: ColonyId,
    pub principal: f64,
    pub interest_rate_pct: f64,
    pub term_turns: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepayLoan {
    pub loan: Loan,
    pub payment_amount: f64,
}

impl Command for TakeLoan {
    type Error = BankingCommandError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.principal <= 0.0 {
            return Err(BankingCommandError::InvalidCommand(
                "Principal must be positive".to_string(),
            ));
        }
        if self.term_turns == 0 {
            return Err(BankingCommandError::InvalidCommand(
                "Term must be at least 1 turn".to_string(),
            ));
        }
        if self.interest_rate_pct < 0.0 {
            return Err(BankingCommandError::InvalidCommand(
                "Interest rate cannot be negative".to_string(),
            ));
        }
        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        let (_loan, event) = Loan::issue(
            self.loan_id,
            self.colony_id,
            self.principal,
            self.interest_rate_pct,
            self.term_turns,
        );
        Ok(vec![event])
    }
}

impl Command for RepayLoan {
    type Error = BankingCommandError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.payment_amount <= 0.0 {
            return Err(BankingCommandError::InvalidCommand(
                "Payment amount must be positive".to_string(),
            ));
        }
        if self.loan.remaining_principal <= 0.0 {
            return Err(BankingCommandError::InvalidCommand(
                "Loan already paid off".to_string(),
            ));
        }
        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;
        let mut loan = self.loan.clone();
        let events = loan.apply_payment(self.payment_amount);
        if let Some(EventType::LoanPaymentMade {
            loan_id,
            colony_id,
            remaining_principal,
            ..
        }) = events.first()
        {
        }
        Ok(events)
    }
}
