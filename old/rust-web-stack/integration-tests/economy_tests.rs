use outpost3::domain::{calculate_snapshot, ColonyId, EconomySnapshot};
use outpost3::events::EventType;
use proptest::prelude::*;

#[test]
fn gdp_calculation_positive_delta() {
    let (snapshot, event) = calculate_snapshot(ColonyId(1), 1000, 1300);
    assert_eq!(snapshot.gdp, 300);
    assert_eq!(snapshot.income, 300);
    assert_eq!(snapshot.expenses, 0);
    assert_eq!(snapshot.net_worth, 1300);

    match event {
        EventType::EconomySnapshot { colony_id, gdp, income, expenses, net_worth } => {
            assert_eq!(colony_id, ColonyId(1));
            assert_eq!(gdp, 300);
            assert_eq!(income, 300);
            assert_eq!(expenses, 0);
            assert_eq!(net_worth, 1300);
        }
        _ => panic!("Expected EconomySnapshot event"),
    }
}

proptest! {
    #[test]
    fn income_minus_expenses_equals_delta(before in -10_000i64..10_000i64, delta in -5_000i64..5_000i64) {
        let after = before + delta;
        let (snapshot, _) = calculate_snapshot(ColonyId(99), before, after);
        prop_assert_eq!(snapshot.income - snapshot.expenses, delta);
        prop_assert_eq!(snapshot.net_worth, after);
        prop_assert_eq!(snapshot.gdp, delta);
    }
}
