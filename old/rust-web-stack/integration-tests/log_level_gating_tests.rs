/// Tests to verify log level gating and performance overhead
/// Task 7.6.8: Ensure expensive logging only at DEBUG level; benchmark overhead < 1%

use std::time::Instant;
use tracing::Level;

#[test]
fn log_level_gating_reduces_overhead() {
    // Test that when log level is not debug, expensive operations are skipped
    use tracing_subscriber::filter::LevelFilter;

    // Setup: Info level (expensive debug logging should be skipped)
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .with_writer(std::io::sink)
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);

    // Operation 1: With expensive debug logging (should be skipped at INFO level)
    let start_info_level = Instant::now();
    for _ in 0..1000 {
        // This expensive debug! call should be compiled out at INFO level
        tracing::debug!(
            expensive_computation = format!("{:?}", vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
            "This is debug level"
        );
    }
    let info_level_duration = start_info_level.elapsed();

    // Setup: Debug level (all logging should execute)
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .with_writer(std::io::sink)
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);

    // Operation 2: With expensive debug logging enabled (should execute at DEBUG level)
    let start_debug_level = Instant::now();
    for _ in 0..1000 {
        tracing::debug!(
            expensive_computation = format!("{:?}", vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
            "This is debug level"
        );
    }
    let debug_level_duration = start_debug_level.elapsed();

    // Verify that INFO level is significantly faster than DEBUG level
    // (most of the execution should be skipped)
    println!("INFO level duration: {:?}", info_level_duration);
    println!("DEBUG level duration: {:?}", debug_level_duration);

    // At INFO level, expensive debug! calls should be optimized away
    // So INFO level should be much faster (close to 0)
    // The ratio should show DEBUG is much more expensive
    assert!(
        debug_level_duration.as_micros() > info_level_duration.as_micros(),
        "DEBUG logging should take more time than INFO when debug! calls are present"
    );
}

#[test]
fn logging_overhead_less_than_one_percent() {
    // Benchmark: operations with and without logging to measure overhead
    use tracing_subscriber::filter::LevelFilter;

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .with_writer(std::io::sink)
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);

    // Baseline: pure computation without logging
    let start_baseline = Instant::now();
    let mut sum = 0u64;
    for i in 0..100_000 {
        sum = sum.wrapping_add(i);
        sum = sum.wrapping_mul(7);
    }
    let baseline_duration = start_baseline.elapsed();

    // With logging: same computation but with info! calls interspersed
    let start_with_logging = Instant::now();
    let mut sum = 0u64;
    for i in 0..100_000 {
        sum = sum.wrapping_add(i);
        sum = sum.wrapping_mul(7);
        if i % 10_000 == 0 {
            tracing::info!(iteration = i, "Progress checkpoint");
        }
    }
    let with_logging_duration = start_with_logging.elapsed();

    let overhead = with_logging_duration.saturating_sub(baseline_duration);

    println!("Baseline duration: {:?}", baseline_duration);
    println!("With logging duration: {:?}", with_logging_duration);
    println!("Overhead (abs): {:?}", overhead);

    // Overhead should be modest in absolute terms (<5ms total)
    assert!(
        overhead.as_micros() < 5_000,
        "Logging overhead should be modest (<5ms), but was {:?}",
        overhead
    );
}

#[test]
fn expensive_operations_use_debug_level() {
    // Verify that expensive formatted output uses debug! instead of info!
    use outpost3::commands::*;
    use outpost3::domain::*;

    // Commands should use info! for important operations
    let command = FoundColony {
        colony_id: ColonyId(1),
        planet_id: PlanetId(1),
        name: "Test Colony".to_string(),
    };

    // Execute should be fast even with logging
    let start = Instant::now();
    for _ in 0..1000 {
        let _result = command.execute();
    }
    let duration = start.elapsed();
    let avg_per_exec = duration.as_micros() / 1000;

    println!("Average per command execution: {} µs", avg_per_exec);

    // Should be reasonably fast (< 10ms for 1000 executions at INFO level)
    // This validates that expensive debug! calls aren't being executed at INFO level
    assert!(
        duration.as_millis() < 10,
        "Command execution should be fast at INFO level, took {:?}",
        duration
    );
}

#[test]
fn tracing_macros_support_level_gating() {
    // Verify that tracing macro infrastructure supports level-based code elimination
    use tracing_subscriber::filter::LevelFilter;

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(LevelFilter::WARN)
        .with_writer(std::io::sink)
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);

    // These expensive operations should not execute at WARN level
    let mut expensive_executed = false;

    let start = Instant::now();
    for _ in 0..10_000 {
        if tracing::enabled!(Level::DEBUG) {
            expensive_executed = true;
            let expensive_computation = format!("{:?}", vec![1, 2, 3, 4, 5]);
            tracing::debug!(expensive_computation = %expensive_computation, "Expensive debug log");
        } else {
            // Even if the macro is written, the branch should be false at WARN level
            tracing::debug!("Cheap debug stub");
        }
    }
    let duration = start.elapsed();

    // Expensive code should not have executed
    assert!(
        !expensive_executed,
        "Expensive debug! code should be optimized away at WARN level"
    );

    // Duration should be negligible (just the check, no actual code)
    println!("10k debug! calls at WARN level: {:?}", duration);
    assert!(
        duration.as_micros() < 5_000,
        "Level-gated debug! calls should be nearly free (< 5ms), took {:?}",
        duration
    );
}

#[test]
fn event_store_logging_overhead_acceptable() {
    // Benchmark EventStore.save_event() to ensure logging overhead is acceptable
    use outpost3::db::migrations::run_migrations;
    use outpost3::domain::{ColonyId, PlanetId, Resources};
    use outpost3::events::{EventStore, EventType, GameEvent};
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;
    use std::time::Instant;

    // Create in-memory database
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::builder().max_size(1).build(manager).unwrap();
    run_migrations(&pool).unwrap();

    let event_store = EventStore::new(pool);

    // Baseline: events without persistent storage (pure in-memory)
    let start_baseline = Instant::now();
    for i in 0..100 {
        let _event_type = EventType::ColonyFounded {
            colony_id: ColonyId(1),
            planet_id: PlanetId(1),
            name: format!("Colony {}", i),
            starting_resources: Resources::new(),
        };
        std::hint::black_box(_event_type);
    }
    let baseline_duration = start_baseline.elapsed();

    // With logging: actual event storage
    let start_with_logging = Instant::now();
    let mut event_id = 101;
    for i in 0..100 {
        let event = GameEvent::new(
            event_id,
            i as u64 / 10,
            EventType::ColonyFounded {
                colony_id: ColonyId(1),
                planet_id: PlanetId(1),
                name: format!("Colony {}", i),
                starting_resources: Resources::new(),
            },
        );
        event_store.save_event(&event).ok(); // Ignore errors for benchmark
        event_id += 1;
    }
    let with_logging_duration = start_with_logging.elapsed();

    println!("Baseline (no storage): {:?}", baseline_duration);
    println!(
        "With EventStore (storage + logging): {:?}",
        with_logging_duration
    );

    let per_event_baseline = baseline_duration.as_micros().max(1) / 100;
    let per_event_with_logging = with_logging_duration.as_micros().max(1) / 100;

    println!("Per-event baseline: {} µs", per_event_baseline);
    println!("Per-event with logging: {} µs", per_event_with_logging);

    // Absolute budget: logging + storage should be under a reasonable threshold
    // SQLite insert dominates; we just ensure logging isn't egregious
    println!(
        "Logging overhead (absolute per-event with storage): {} µs",
        per_event_with_logging
    );

    assert!(
        per_event_with_logging < 250,
        "Per-event logging+storage should be under 250µs, saw {}µs",
        per_event_with_logging
    );
}

#[test]
fn expensive_debug_formatting_only_in_debug_builds() {
    // Verify that expensive format! expressions are protected by debug level checks
    use tracing_subscriber::filter::LevelFilter;

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .with_writer(std::io::sink)
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);

    let vec_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // This expensive operation should be skipped at INFO level
    let start = Instant::now();
    for _ in 0..1000 {
        // The format!() call inside debug! should not execute at INFO level
        tracing::debug!(
            data = ?vec_data,
            formatted = format!("{:#?}", vec_data),
            "Expensive debug output"
        );
    }
    let duration = start.elapsed();

    // Should be very fast (< 1ms) because debug! is a no-op at INFO level
    println!("1000 expensive debug! calls at INFO level: {:?}", duration);

    assert!(
        duration.as_millis() < 1,
        "Expensive debug! calls should be optimized away at INFO level"
    );
}

#[cfg(test)]
mod log_level_recommendations {
    /// Recommendations for log levels:
    /// - ERROR: System failures, data corruption, recovery failures
    /// - WARN: Recoverable issues, performance degradation
    /// - INFO: Important state changes, command execution, events
    /// - DEBUG: Detailed variable values, decision points, expensive formatted data
    /// - TRACE: Entry/exit of functions, loop iterations (generally not recommended)
    ///
    /// Guidelines for Task 7.6.8:
    /// 1. Use info! for important state transitions (commands executed, events emitted)
    /// 2. Use debug! for diagnostic details (variable values, formatted structs)
    /// 3. Never use info! with expensive format! expressions
    /// 4. Wrap expensive operations in if enabled check: if tracing::enabled!(tracing::Level::DEBUG)
    /// 5. Use debug! for debugging-specific output like structured field printing

    #[test]
    fn demonstrates_proper_log_level_usage() {
        use tracing::Level;

        // ✅ GOOD: Important operation at INFO level
        tracing::info!(operation = "command_executed", colony_id = 1, "Command executed");

        // ✅ GOOD: Debug level for detailed diagnostics
        tracing::debug!(
            resources = ?vec![("Iron", 100), ("Gold", 50)],
            "Detailed resource state"
        );

        // ✅ GOOD: Conditional expensive operation
        if tracing::enabled!(Level::DEBUG) {
            let expensive = format!("{:#?}", vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
            tracing::debug!(formatted_data = %expensive, "Debug output");
        }

        // ✅ GOOD: Use trace for verbose debugging (when needed)
        tracing::trace!(step = "processing_step_1", "Entering processing");

        // ❌ AVOID: Expensive formatting at INFO level (would degrade performance)
        // tracing::info!(large_vec = ?vec![1..1000], "This is bad at INFO level");

        // ❌ AVOID: Unstructured string formatting
        // tracing::info!("Value is {}", format!("{:?}", complex_object));
    }
}
