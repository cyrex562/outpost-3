use outpost3::commands::{BankingCommandError, Command, RepayLoan, TakeLoan};
use outpost3::domain::{ColonyId, Loan, LoanId};
use outpost3::events::EventType;

#[test]
fn take_loan_validation_fails_for_nonpositive_principal() {
    let cmd = TakeLoan {
        loan_id: LoanId(1),
        colony_id: ColonyId(1),
        principal: 0.0,
        interest_rate_pct: 5.0,
        term_turns: 12,
    };

    let result = cmd.validate();
    assert_eq!(
        result,
        Err(BankingCommandError::InvalidCommand(
            "Principal must be positive".to_string()
        ))
    );
}

#[test]
fn loan_issue_and_repay_flow() {
    let take = TakeLoan {
        loan_id: LoanId(10),
        colony_id: ColonyId(2),
        principal: 500.0,
        interest_rate_pct: 0.0,
        term_turns: 6,
    };

    let issued_events = take.execute().expect("take loan execute");
    assert_eq!(issued_events.len(), 1);
    match &issued_events[0] {
        EventType::LoanIssued {
            loan_id,
            colony_id,
            principal,
            interest_rate,
            term_turns,
        } => {
            assert_eq!(*loan_id, LoanId(10));
            assert_eq!(*colony_id, ColonyId(2));
            assert_eq!(*principal, 500.0);
            assert_eq!(*interest_rate, 0.0);
            assert_eq!(*term_turns, 6);
        }
        _ => panic!("Expected LoanIssued"),
    }

    let (loan_state, _) = Loan::issue(LoanId(10), ColonyId(2), 500.0, 0.0, 6);
    let repay = RepayLoan {
        loan: loan_state,
        payment_amount: 600.0,
    };

    let payment_events = repay.execute().expect("repay execute");
    assert!(payment_events
        .iter()
        .any(|ev| matches!(ev, EventType::LoanPaymentMade { .. })));
    assert!(payment_events
        .iter()
        .any(|ev| matches!(ev, EventType::LoanPaidOff { .. })));
}
