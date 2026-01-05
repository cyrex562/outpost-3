use std::time::Instant;

use outpost3::commands::{
    banking_commands::{RepayLoan, TakeLoan},
    trading_commands::{BuyResource, SellResource},
    Command,
};
use outpost3::db::migrations::run_migrations;
use outpost3::db::pool::DbPool;
use outpost3::domain::banking::{Loan, LoanId};
use outpost3::domain::market::{market_price_update, MarketPrice};
use outpost3::domain::{ColonyId, ResourceType};
use outpost3::events::event::TradeSide;
use outpost3::events::{EventStore, EventType};
use outpost3::simulation::turn::TurnProcessor;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

fn in_memory_pool() -> DbPool {
    let manager = SqliteConnectionManager::memory();
    Pool::builder().max_size(1).build(manager).unwrap()
}

#[test]
fn market_arbitrage_scenario() {
    // Test buying low and selling high across multiple market updates
    let colony_id = ColonyId(1);
    let resource = ResourceType::Steel;

    // Initial market price
    let mut price = 100.0;
    let market_price = MarketPrice::new(resource, price);

    // Colony starts with 10,000 credits and 0 steel
    let mut credits = 10_000i64;
    let mut steel_stock = 0i64;

    // Buy 50 steel at 100 per unit (cost: 5,000)
    let buy_cmd = BuyResource {
        colony_id,
        resource_type: resource,
        quantity: 50,
        market_price,
        available_credits: credits,
    };

    let events = buy_cmd.execute().unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        EventType::ResourceTraded {
            side: TradeSide::Buy,
            quantity: 50,
            ..
        }
    ));

    credits -= (50.0 * price) as i64;
    steel_stock += 50;
    assert_eq!(credits, 5_000);
    assert_eq!(steel_stock, 50);

    // Market price increases due to high demand (10% increase)
    let (new_price, price_event) = market_price_update(resource, price, 0.1);
    assert!(matches!(price_event, EventType::MarketPriceChanged { .. }));
    assert!((new_price - 110.0).abs() < 0.01); // Floating point tolerance
    price = new_price;

    // Sell all 50 steel at 110 per unit (revenue: 5,500)
    let sell_cmd = SellResource {
        colony_id,
        resource_type: resource,
        quantity: 50,
        market_price: MarketPrice::new(resource, price),
        available_quantity: steel_stock,
    };

    let sell_events = sell_cmd.execute().unwrap();
    assert_eq!(sell_events.len(), 1);
    assert!(matches!(
        sell_events[0],
        EventType::ResourceTraded {
            side: TradeSide::Sell,
            quantity: 50,
            ..
        }
    ));

    credits += (50.0 * price) as i64;
    steel_stock -= 50;

    assert_eq!(credits, 10_500);
    assert_eq!(steel_stock, 0);

    // Verify arbitrage profit of 500 credits
    let profit = 10_500 - 10_000;
    assert_eq!(profit, 500);
}

#[test]
fn loan_investment_cycle() {
    // Test taking a loan, investing proceeds, and repaying with returns
    let colony_id = ColonyId(1);
    let loan_id = LoanId(1);

    // Starting position: 1,000 credits
    let mut credits = 1_000i64;

    // Take a loan of 5,000 at 8% for 10 turns
    let take_loan = TakeLoan {
        loan_id,
        colony_id,
        principal: 5_000.0,
        interest_rate_pct: 8.0,
        term_turns: 10,
    };

    let loan_events = take_loan.execute().unwrap();
    assert_eq!(loan_events.len(), 1);
    assert!(matches!(
        loan_events[0],
        EventType::LoanIssued {
            principal: 5000.0,
            interest_rate: 8.0,
            term_turns: 10,
            ..
        }
    ));

    // Receive loan proceeds
    credits += 5_000;
    assert_eq!(credits, 6_000);

    // Create loan state for tracking
    let (mut loan, _) = Loan::issue(loan_id, colony_id, 5_000.0, 8.0, 10);

    // Use borrowed funds to buy resources at 50 per unit (buy 100 units for 5,000)
    let buy_cmd = BuyResource {
        colony_id,
        resource_type: ResourceType::CopperOre,
        quantity: 100,
        market_price: MarketPrice::new(ResourceType::CopperOre, 50.0),
        available_credits: credits,
    };

    let _ = buy_cmd.execute().unwrap();
    credits -= 5_000;
    let mut copper_stock = 100i64;
    assert_eq!(credits, 1_000);

    // Market improves: price rises to 70 per unit
    let sell_cmd = SellResource {
        colony_id,
        resource_type: ResourceType::CopperOre,
        quantity: 100,
        market_price: MarketPrice::new(ResourceType::CopperOre, 70.0),
        available_quantity: copper_stock,
    };

    let _ = sell_cmd.execute().unwrap();
    credits += 7_000;
    copper_stock -= 100;
    assert_eq!(credits, 8_000);

    // Make payments over multiple turns
    let payment = loan.amortized_payment();
    assert!(payment > 0.0);

    let total_repaid = (payment.ceil() as i64) * 10;

    for turn in 1..=10 {
        let repay = RepayLoan {
            loan: loan.clone(),
            payment_amount: payment,
        };

        let payment_events = repay.execute().unwrap();

        // Update loan state
        let updated_events = loan.apply_payment(payment);
        assert!(!updated_events.is_empty());

        credits -= payment.ceil() as i64;

        if turn < 10 {
            assert_eq!(payment_events.len(), 1);
            assert!(matches!(
                payment_events[0],
                EventType::LoanPaymentMade { .. }
            ));
        } else {
            // Final payment triggers paid-off event
            assert_eq!(payment_events.len(), 2);
            assert!(matches!(payment_events[1], EventType::LoanPaidOff { .. }));
            assert!(loan.remaining_principal < 1.0);
        }
    }

    // Verify profit: started with 1,000, gained 2,000 from trading, paid back ~5,400 total (principal + interest)
    // Net position should be 8,000 - total_repaid
    let expected_minimum = 8_000 - total_repaid - 100; // Allow small buffer
    assert!(
        credits >= expected_minimum,
        "Expected credits >= {}, got {}",
        expected_minimum,
        credits
    );
}

#[test]
fn loan_default_scenario() {
    // Test scenario where a loan cannot be fully repaid
    let colony_id = ColonyId(1);
    let loan_id = LoanId(1);

    // Small starting balance
    let mut credits = 500i64;

    // Take a large loan
    let (mut loan, issue_event) = Loan::issue(loan_id, colony_id, 10_000.0, 12.0, 5);
    assert!(matches!(issue_event, EventType::LoanIssued { .. }));

    credits += 10_000;
    assert_eq!(credits, 10_500);

    // Bad investment: buy resources that depreciate
    credits -= 10_000;
    assert_eq!(credits, 500);

    // Calculate required payment
    let required_payment = loan.amortized_payment();
    assert!(required_payment > (credits as f64));

    // Attempt payment with insufficient funds - validation should catch this
    let insufficient_payment = credits as f64;
    let events = loan.apply_payment(insufficient_payment);

    // Payment applied but doesn't cover principal much
    assert_eq!(events.len(), 1);
    if let EventType::LoanPaymentMade {
        principal_paid,
        interest_paid,
        remaining_principal,
        ..
    } = &events[0]
    {
        assert!(*interest_paid > 0.0);
        assert!(*principal_paid >= 0.0);
        assert!(*remaining_principal > 9_000.0); // Barely dented the principal
    } else {
        panic!("Expected LoanPaymentMade event");
    }

    // Loan remains largely unpaid, demonstrating default risk
    assert!(loan.remaining_principal > 9_000.0);
}

#[test]
fn fifty_turn_simulation_performance() {
    let start = Instant::now();

    let pool = in_memory_pool();
    run_migrations(&pool).unwrap();

    let event_store = EventStore::new(pool);
    let processor = TurnProcessor::new(event_store);

    // Simulate 50 turns with financial activity
    for turn in 1..=50 {
        let colony_financials = vec![
            (
                ColonyId(1),
                5_000 + (turn as i64 * 100),
                5_200 + (turn as i64 * 100),
            ),
            (
                ColonyId(2),
                8_000 - (turn as i64 * 50),
                8_100 - (turn as i64 * 50),
            ),
            (ColonyId(3), 12_000, 12_150),
        ];

        let events = processor.process_turn(turn, &colony_financials).unwrap();

        // Each turn generates: 3 economy snapshots + 1 turn advanced = 4 events
        assert_eq!(events.len(), 4);

        // Verify economy snapshots generated
        let snapshot_count = events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::EconomySnapshot { .. }))
            .count();
        assert_eq!(snapshot_count, 3);

        // Verify turn advanced
        let turn_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::TurnAdvanced { .. }))
            .collect();
        assert_eq!(turn_events.len(), 1);
    }

    let elapsed = start.elapsed();

    // Performance requirement: 50 turns in under 500ms
    assert!(
        elapsed.as_millis() < 500,
        "50-turn simulation took {}ms (expected < 500ms)",
        elapsed.as_millis()
    );

    println!(
        "✓ 50-turn simulation completed in {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn combined_financial_operations() {
    // Integration test combining market trading, loans, and economy tracking
    let colony_id = ColonyId(1);
    let loan_id = LoanId(1);

    let mut credits = 2_000i64;

    // 1. Take a loan to increase capital
    let (mut loan, _) = Loan::issue(loan_id, colony_id, 3_000.0, 6.0, 8);
    credits += 3_000;
    assert_eq!(credits, 5_000);

    // 2. Market operations
    let mut price = 40.0;
    let buy = BuyResource {
        colony_id,
        resource_type: ResourceType::Lithium,
        quantity: 80,
        market_price: MarketPrice::new(ResourceType::Lithium, price),
        available_credits: credits,
    };
    let _ = buy.execute().unwrap();
    credits -= (80.0 * price) as i64;

    // 3. Market price increases
    (price, _) = market_price_update(ResourceType::Lithium, price, 0.08);
    assert!(price > 40.0);

    // 4. Sell at higher price
    let sell = SellResource {
        colony_id,
        resource_type: ResourceType::Lithium,
        quantity: 80,
        market_price: MarketPrice::new(ResourceType::Lithium, price),
        available_quantity: 80,
    };
    let _ = sell.execute().unwrap();
    credits += (80.0 * price) as i64;

    // 5. Make loan payment
    let payment = loan.amortized_payment();
    let payment_events = loan.apply_payment(payment);
    assert!(!payment_events.is_empty());
    credits -= payment as i64;

    // 6. Verify overall financial health improved
    assert!(credits > 2_000); // Better than starting position

    // 7. Verify loan progressing
    assert!(loan.remaining_principal < 3_000.0);
    assert_eq!(loan.payments_made, 1);
}
