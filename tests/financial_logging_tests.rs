use std::io::Write;
use std::sync::{Arc, Mutex};

use outpost3::commands::{
    banking_commands::RepayLoan, banking_commands::TakeLoan, trading_commands::BuyResource, Command,
};
use outpost3::db::migrations::run_migrations;
use outpost3::db::pool::DbPool;
use outpost3::domain::banking::{Loan, LoanId};
use outpost3::domain::market::{market_price_update, MarketPrice};
use outpost3::domain::{calculate_snapshot, ColonyId, ResourceType};
use outpost3::events::{EventStore, EventType, GameEvent};
use outpost3::simulation::turn::TurnProcessor;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tracing_subscriber::fmt::MakeWriter;

struct TestWriter(Arc<Mutex<Vec<u8>>>);

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.0.lock().unwrap();
        guard.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for TestWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        TestWriter(self.0.clone())
    }
}

fn capture_logs<F: FnOnce()>(f: F) -> String {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = TestWriter(buffer.clone());

    let subscriber = tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .without_time()
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);
    f();

    let bytes = buffer.lock().unwrap();
    String::from_utf8_lossy(&bytes).to_string()
}

fn in_memory_pool() -> DbPool {
    let manager = SqliteConnectionManager::memory();
    Pool::builder().max_size(1).build(manager).unwrap()
}

#[test]
fn financial_events_emit_logs() {
    let logs = capture_logs(|| {
        // Market price adjustment
        let _ = market_price_update(ResourceType::Steel, 100.0, 0.05);

        // Trading command
        let buy = BuyResource {
            colony_id: ColonyId(1),
            resource_type: ResourceType::Steel,
            quantity: 5,
            market_price: MarketPrice::new(ResourceType::Steel, 12.5),
            available_credits: 1_000,
        };
        let _ = buy.execute().unwrap();

        // Loan issue + repayment
        let take = TakeLoan {
            loan_id: LoanId(42),
            colony_id: ColonyId(1),
            principal: 500.0,
            interest_rate_pct: 5.0,
            term_turns: 12,
        };
        let _ = take.execute().unwrap();

        let (loan, _) = Loan::issue(LoanId(43), ColonyId(2), 750.0, 6.5, 10);
        let repay = RepayLoan {
            loan,
            payment_amount: 120.0,
        };
        let _ = repay.execute().unwrap();

        // Economy snapshot
        let _ = calculate_snapshot(ColonyId(3), 1_000, 1_150);

        // Turn summary logging (also persists events)
        let pool = in_memory_pool();
        run_migrations(&pool).unwrap();
        let processor = TurnProcessor::new(EventStore::new(pool));
        let _persisted = processor
            .process_turn(5, &[(ColonyId(99), 2_000, 2_150)])
            .unwrap();
    });

    assert!(logs.contains("Market price update"));
    assert!(logs.contains("Trade executed"));
    assert!(logs.contains("Loan issued"));
    assert!(logs.contains("Loan payment"));
    assert!(logs.contains("Economy update"));
}

#[test]
fn financial_events_persist_in_event_store() {
    let pool = in_memory_pool();
    run_migrations(&pool).unwrap();

    // Use same pool for processor and verification
    let processor_store = EventStore::new(pool.clone());
    let processor = TurnProcessor::new(processor_store);
    let persisted = processor
        .process_turn(7, &[(ColonyId(1), 5_000, 5_400)])
        .unwrap();

    let verification_store = EventStore::new(pool.clone());

    let next_id = verification_store.get_next_event_id().unwrap();
    let trade_event = GameEvent::new(
        next_id,
        7,
        EventType::ResourceTraded {
            colony_id: ColonyId(1),
            resource_type: ResourceType::Nickel,
            quantity: 10,
            price: 55.0,
            side: outpost3::events::event::TradeSide::Buy,
        },
    );
    verification_store.save_event(&trade_event).unwrap();

    let saved = verification_store.get_all_events().unwrap();

    assert_eq!(saved.len(), persisted.len() + 1);
    assert!(saved
        .iter()
        .any(|e| matches!(e.event_type, EventType::EconomySnapshot { .. })));
    assert!(saved
        .iter()
        .any(|e| matches!(e.event_type, EventType::ResourceTraded { .. })));
    assert!(saved
        .iter()
        .any(|e| matches!(e.event_type, EventType::TurnAdvanced { .. })));
}
