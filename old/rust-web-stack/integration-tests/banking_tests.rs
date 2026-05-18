use outpost3::domain::{amortized_payment, ColonyId, Loan, LoanId};
use outpost3::events::EventType;
use proptest::prelude::*;

#[test]
fn loan_issue_and_payoff_flow() {
    let colony_id = ColonyId(7);
    let (mut loan, issued_event) = Loan::issue(LoanId(1), colony_id, 1000.0, 5.0, 12);

    match issued_event {
        EventType::LoanIssued {
            loan_id,
            colony_id: ev_colony,
            principal,
            interest_rate,
            term_turns,
        } => {
            assert_eq!(loan_id, LoanId(1));
            assert_eq!(ev_colony, colony_id);
            assert_eq!(principal, 1000.0);
            assert_eq!(interest_rate, 5.0);
            assert_eq!(term_turns, 12);
        }
        _ => panic!("Unexpected event type"),
    }

    let payment = loan.amortized_payment();
    let mut payoff_seen = false;

    for _ in 0..=loan.term_turns {
        let events = loan.apply_payment(payment);
        assert!(!events.is_empty());

        match &events[0] {
            EventType::LoanPaymentMade {
                loan_id,
                colony_id: ev_colony,
                amount,
                remaining_principal,
                ..
            } => {
                assert_eq!(*loan_id, LoanId(1));
                assert_eq!(*ev_colony, colony_id);
                assert_eq!(*amount, payment);
                assert!(*remaining_principal >= 0.0);
            }
            _ => panic!("Expected LoanPaymentMade"),
        }

        if events.iter().any(|ev| matches!(ev, EventType::LoanPaidOff { .. })) {
            payoff_seen = true;
            break;
        }
    }

    assert!(payoff_seen, "Loan should be paid off within term");
    assert!(loan.remaining_principal <= 1e-3);
}

proptest! {
    #[test]
    fn amortized_total_exceeds_principal(principal in 100.0f64..5000.0, rate in 0.1f64..20.0, term in 1u32..36u32) {
        let payment = amortized_payment(principal, rate, term);
        let total_paid = payment * term as f64;
        prop_assert!(total_paid > principal + 1e-6);
    }
}
