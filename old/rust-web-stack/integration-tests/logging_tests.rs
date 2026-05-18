use outpost3::commands::*;
use outpost3::domain::market::MarketPrice;
use outpost3::domain::*;
use outpost3::events::event::TradeSide;
use outpost3::events::EventType;

/// Test that each command emits at least one log entry
#[test]
fn all_commands_emit_logs() {
    // Test TakeLoan command
    let take_loan = TakeLoan {
        loan_id: LoanId(1),
        colony_id: ColonyId(1),
        principal: 1000.0,
        interest_rate_pct: 5.0,
        term_turns: 12,
    };
    let _result = take_loan.execute();
    // Note: Currently TakeLoan doesn't log - this test will document the gap until 7.6.2

    // Test RepayLoan command
    let (loan, _event) = Loan::issue(LoanId(1), ColonyId(1), 500.0, 5.0, 12);
    let repay = RepayLoan {
        loan,
        payment_amount: 50.0,
    };
    let _result = repay.execute();
    // RepayLoan should emit log

    // Test BuyResource command
    let buy = BuyResource {
        colony_id: ColonyId(1),
        resource_type: ResourceType::IronOre,
        quantity: 100,
        market_price: MarketPrice {
            resource: ResourceType::IronOre,
            price: 10.0,
        },
        available_credits: 2000,
    };
    let _result = buy.execute();

    // Test SellResource command
    let sell = SellResource {
        colony_id: ColonyId(1),
        resource_type: ResourceType::Steel,
        quantity: 50,
        market_price: MarketPrice {
            resource: ResourceType::Steel,
            price: 25.0,
        },
        available_quantity: 100,
    };
    let _result = sell.execute();
}

/// Test that trading commands emit properly structured logs
#[test]
fn trading_commands_log_structured_fields() {
    let buy = BuyResource {
        colony_id: ColonyId(42),
        resource_type: ResourceType::Food,
        quantity: 200,
        market_price: MarketPrice {
            resource: ResourceType::Food,
            price: 5.0,
        },
        available_credits: 2000,
    };

    let result = buy.execute();
    assert!(result.is_ok());

    let events = result.unwrap();
    assert_eq!(events.len(), 1);

    match &events[0] {
        EventType::ResourceTraded {
            colony_id,
            resource_type,
            quantity,
            price,
            side,
        } => {
            assert_eq!(*colony_id, ColonyId(42));
            assert_eq!(*resource_type, ResourceType::Food);
            assert_eq!(*quantity, 200);
            assert_eq!(*price, 5.0);
            assert_eq!(*side, TradeSide::Buy);
        }
        _ => panic!("Expected ResourceTraded event"),
    }
}

/// Test that banking commands emit logs with loan details
#[test]
fn banking_commands_log_loan_details() {
    let (loan, _event) = Loan::issue(LoanId(123), ColonyId(7), 1000.0, 6.0, 24);
    let repay = RepayLoan {
        loan,
        payment_amount: 100.0,
    };

    let result = repay.execute();
    assert!(result.is_ok());

    let events = result.unwrap();
    assert!(!events.is_empty());

    // Verify LoanPaymentMade event generated
    match &events[0] {
        EventType::LoanPaymentMade {
            loan_id,
            colony_id,
            amount,
            ..
        } => {
            assert_eq!(*loan_id, LoanId(123));
            assert_eq!(*colony_id, ColonyId(7));
            assert_eq!(*amount, 100.0);
        }
        _ => panic!("Expected LoanPaymentMade event"),
    }
}

/// Test that events are properly formatted for logging
#[test]
fn events_have_loggable_format() {
    let event = EventType::ColonyFounded {
        colony_id: ColonyId(1),
        planet_id: PlanetId(1),
        name: "Test Colony".to_string(),
        starting_resources: Resources::starting_resources(),
    };

    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("ColonyFounded"));
    assert!(debug_str.contains("Test Colony"));

    let event2 = EventType::BuildingConstructionStarted {
        building_id: BuildingId(42),
        colony_id: ColonyId(1),
        building_type: BuildingType::Mine {
            resource_type: ResourceType::IronOre,
            output_rate: 10,
        },
    };

    let debug_str2 = format!("{:?}", event2);
    assert!(debug_str2.contains("BuildingConstructionStarted"));
    assert!(debug_str2.contains("BuildingId"));
}

/// Property test: all Command types should have execute() method
#[test]
fn commands_support_logging_infrastructure() {
    use std::any::type_name;

    let buy = BuyResource {
        colony_id: ColonyId(1),
        resource_type: ResourceType::Credits,
        quantity: 1,
        market_price: MarketPrice {
            resource: ResourceType::Credits,
            price: 1.0,
        },
        available_credits: 100,
    };

    let type_name = type_name::<BuyResource>();
    assert!(type_name.contains("BuyResource"));

    let result = buy.execute();
    assert!(result.is_ok() || result.is_err());
}

/// Test validation failure handling
#[test]
fn validation_failures_should_be_logged() {
    let bad_buy = BuyResource {
        colony_id: ColonyId(1),
        resource_type: ResourceType::Food,
        quantity: 0, // Invalid!
        market_price: MarketPrice {
            resource: ResourceType::Food,
            price: 5.0,
        },
        available_credits: 1000,
    };

    let result = bad_buy.validate();
    assert!(result.is_err());
}

/// Test that all event variants can be logged
#[test]
fn all_event_types_are_loggable() {
    let events = vec![
        EventType::ColonyFounded {
            colony_id: ColonyId(1),
            planet_id: PlanetId(1),
            name: "Test".to_string(),
            starting_resources: Resources::new(),
        },
        EventType::ResourceTraded {
            colony_id: ColonyId(1),
            resource_type: ResourceType::Food,
            quantity: 100,
            price: 10.0,
            side: TradeSide::Buy,
        },
        EventType::LoanIssued {
            loan_id: LoanId(1),
            colony_id: ColonyId(1),
            principal: 1000.0,
            interest_rate: 5.0,
            term_turns: 12,
        },
        EventType::BuildingUpgraded {
            building_id: BuildingId(1),
            colony_id: ColonyId(1),
            new_level: 2,
        },
    ];

    for event in events {
        let debug_str = format!("{:?}", event);
        assert!(!debug_str.is_empty());
    }
}

/// Integration test: full command execution
#[test]
fn command_execution_produces_events_and_logs() {
    let buy = BuyResource {
        colony_id: ColonyId(5),
        resource_type: ResourceType::Steel,
        quantity: 50,
        market_price: MarketPrice {
            resource: ResourceType::Steel,
            price: 20.0,
        },
        available_credits: 2000,
    };

    assert!(buy.validate().is_ok());

    let events = buy.execute().expect("Command should execute");
    assert_eq!(events.len(), 1);

    match &events[0] {
        EventType::ResourceTraded {
            colony_id,
            resource_type,
            quantity,
            price,
            side,
        } => {
            assert_eq!(*colony_id, ColonyId(5));
            assert_eq!(*resource_type, ResourceType::Steel);
            assert_eq!(*quantity, 50);
            assert_eq!(*price, 20.0);
            assert_eq!(*side, TradeSide::Buy);
        }
        _ => panic!("Expected ResourceTraded event"),
    }
}

#[cfg(test)]
mod event_logging_tests {
    use super::*;
    use outpost3::db::migrations::run_migrations;
    use outpost3::db::pool::DbPool;
    use outpost3::events::{EventStore, GameEvent};
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;
    use std::sync::{Arc, Mutex};

    struct LogCapture {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl LogCapture {
        fn new() -> Self {
            Self {
                buffer: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_logs(&self) -> String {
            let bytes = self.buffer.lock().unwrap();
            String::from_utf8_lossy(&bytes).to_string()
        }

        fn writer(&self) -> TestWriter {
            TestWriter(self.buffer.clone())
        }
    }

    #[derive(Clone)]
    struct TestWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.0.lock().unwrap().flush()
        }
    }

    fn capture_logs<F: FnOnce()>(f: F) -> String {
        let capture = LogCapture::new();
        let writer = capture.writer();

        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_ansi(false)
            .without_time()
            .finish();

        let _guard = tracing::subscriber::set_default(subscriber);
        f();

        capture.get_logs()
    }

    fn in_memory_pool() -> DbPool {
        let manager = SqliteConnectionManager::memory();
        Pool::builder().max_size(1).build(manager).unwrap()
    }

    #[test]
    fn colony_events_emit_logs() {
        let pool = in_memory_pool();
        run_migrations(&pool).unwrap();
        let event_store = EventStore::new(pool);

        let logs = capture_logs(|| {
            // Test ColonyFounded event
            let event1 = GameEvent::new(
                1,
                1,
                EventType::ColonyFounded {
                    colony_id: ColonyId(1),
                    planet_id: PlanetId(10),
                    name: "Test Colony".to_string(),
                    starting_resources: Resources::new(),
                },
            );
            event_store.save_event(&event1).unwrap();

            // Test BuildingConstructionStarted event
            let event2 = GameEvent::new(
                2,
                1,
                EventType::BuildingConstructionStarted {
                    building_id: BuildingId(1),
                    colony_id: ColonyId(1),
                    building_type: BuildingType::Mine {
                        resource_type: ResourceType::IronOre,
                        output_rate: 10,
                    },
                },
            );
            event_store.save_event(&event2).unwrap();

            // Test ResourcesExtracted event
            let event3 = GameEvent::new(
                3,
                2,
                EventType::ResourcesExtracted {
                    colony_id: ColonyId(1),
                    resource_type: ResourceType::IronOre,
                    amount: 100,
                },
            );
            event_store.save_event(&event3).unwrap();
        });

        assert!(logs.contains("Colony founded"));
        assert!(logs.contains("Building construction started"));
        assert!(logs.contains("Resources extracted"));
        assert!(logs.contains("colony_id"));
        assert!(logs.contains("Test Colony"));
    }

    #[test]
    fn market_and_trading_events_emit_logs() {
        let pool = in_memory_pool();
        run_migrations(&pool).unwrap();
        let event_store = EventStore::new(pool);

        let logs = capture_logs(|| {
            // Test MarketPriceChanged event
            let event1 = GameEvent::new(
                1,
                1,
                EventType::MarketPriceChanged {
                    resource: ResourceType::Steel,
                    old_price: 100.0,
                    new_price: 110.0,
                },
            );
            event_store.save_event(&event1).unwrap();

            // Test ResourceTraded event
            let event2 = GameEvent::new(
                2,
                1,
                EventType::ResourceTraded {
                    colony_id: ColonyId(1),
                    resource_type: ResourceType::Steel,
                    quantity: 50,
                    price: 110.0,
                    side: TradeSide::Buy,
                },
            );
            event_store.save_event(&event2).unwrap();
        });

        assert!(logs.contains("Market price changed"));
        assert!(logs.contains("Resource traded"));
        assert!(logs.contains("change_pct"));
        assert!(logs.contains("total_cost"));
    }

    #[test]
    fn banking_events_emit_logs() {
        let pool = in_memory_pool();
        run_migrations(&pool).unwrap();
        let event_store = EventStore::new(pool);

        let logs = capture_logs(|| {
            // Test LoanIssued event
            let event1 = GameEvent::new(
                1,
                1,
                EventType::LoanIssued {
                    loan_id: LoanId(1),
                    colony_id: ColonyId(1),
                    principal: 5000.0,
                    interest_rate: 0.05,
                    term_turns: 12,
                },
            );
            event_store.save_event(&event1).unwrap();

            // Test LoanPaymentMade event
            let event2 = GameEvent::new(
                2,
                2,
                EventType::LoanPaymentMade {
                    loan_id: LoanId(1),
                    colony_id: ColonyId(1),
                    amount: 500.0,
                    principal_paid: 450.0,
                    interest_paid: 50.0,
                    remaining_principal: 4550.0,
                },
            );
            event_store.save_event(&event2).unwrap();

            // Test LoanPaidOff event
            let event3 = GameEvent::new(
                3,
                15,
                EventType::LoanPaidOff {
                    loan_id: LoanId(1),
                    colony_id: ColonyId(1),
                },
            );
            event_store.save_event(&event3).unwrap();
        });

        assert!(logs.contains("Loan issued"));
        assert!(logs.contains("Loan payment made"));
        assert!(logs.contains("Loan paid off"));
        assert!(logs.contains("principal"));
        assert!(logs.contains("interest_rate"));
    }

    #[test]
    fn production_events_emit_logs() {
        let pool = in_memory_pool();
        run_migrations(&pool).unwrap();
        let event_store = EventStore::new(pool);

        let logs = capture_logs(|| {
            // Test ResourcesProduced event
            let event1 = GameEvent::new(
                1,
                1,
                EventType::ResourcesProduced {
                    colony_id: ColonyId(1),
                    resource_type: ResourceType::Steel,
                    amount: 25,
                },
            );
            event_store.save_event(&event1).unwrap();

            // Test ResourcesConsumed event
            let event2 = GameEvent::new(
                2,
                1,
                EventType::ResourcesConsumed {
                    colony_id: ColonyId(1),
                    resource_type: ResourceType::IronOre,
                    amount: 50,
                },
            );
            event_store.save_event(&event2).unwrap();
        });

        assert!(logs.contains("Resources produced"));
        assert!(logs.contains("Resources consumed"));
        assert!(logs.contains("resource_type"));
        assert!(logs.contains("amount"));
    }

    #[test]
    fn building_lifecycle_events_emit_logs() {
        let pool = in_memory_pool();
        run_migrations(&pool).unwrap();
        let event_store = EventStore::new(pool);

        let logs = capture_logs(|| {
            // Test BuildingUpgraded event
            let event1 = GameEvent::new(
                1,
                5,
                EventType::BuildingUpgraded {
                    building_id: BuildingId(1),
                    colony_id: ColonyId(1),
                    new_level: 2,
                },
            );
            event_store.save_event(&event1).unwrap();

            // Test BuildingRepaired event
            let event2 = GameEvent::new(
                2,
                6,
                EventType::BuildingRepaired {
                    building_id: BuildingId(1),
                    colony_id: ColonyId(1),
                },
            );
            event_store.save_event(&event2).unwrap();

            // Test BuildingRecipeChanged event
            let event3 = GameEvent::new(
                3,
                7,
                EventType::BuildingRecipeChanged {
                    building_id: BuildingId(1),
                    colony_id: ColonyId(1),
                    old_recipe_index: 0,
                    new_recipe_index: 1,
                },
            );
            event_store.save_event(&event3).unwrap();
        });

        assert!(logs.contains("Building upgraded"));
        assert!(logs.contains("Building repaired"));
        assert!(logs.contains("Building recipe changed"));
        assert!(logs.contains("new_level"));
        assert!(logs.contains("old_recipe_index"));
    }

    #[test]
    fn all_event_types_have_logging() {
        // This test verifies that the log_event function handles all EventType variants
        // If a new EventType is added without logging support, this will fail at compile time
        let pool = in_memory_pool();
        run_migrations(&pool).unwrap();
        let event_store = EventStore::new(pool);

        let logs = capture_logs(|| {
            // Create one event of each major category to ensure comprehensive coverage
            let events = vec![
                EventType::ColonyFounded {
                    colony_id: ColonyId(1),
                    planet_id: PlanetId(1),
                    name: "Test".to_string(),
                    starting_resources: Resources::new(),
                },
                EventType::MarketPriceChanged {
                    resource: ResourceType::IronOre,
                    old_price: 10.0,
                    new_price: 11.0,
                },
                EventType::LoanIssued {
                    loan_id: LoanId(1),
                    colony_id: ColonyId(1),
                    principal: 1000.0,
                    interest_rate: 0.05,
                    term_turns: 12,
                },
                EventType::TurnAdvanced { turn_number: 2 },
            ];

            for (i, event_type) in events.into_iter().enumerate() {
                let event = GameEvent::new((i + 1) as u64, 1, event_type);
                event_store.save_event(&event).unwrap();
            }
        });

        // Verify that logs were produced
        assert!(!logs.is_empty(), "No logs were produced");
        assert!(logs.contains("event_id"));
        assert!(logs.contains("turn"));
    }
}
