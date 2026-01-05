# Logging Gap Analysis

**Date**: 2026-01-04  
**Scope**: Server-side Commands and Events logging audit

## Summary

This document identifies missing logging across command execution and event processing in the Outpost-3 server codebase. The analysis covers all Command implementations and EventType variants to ensure structured logging for debugging, monitoring, and audit trails.

## Logging Standards

### Required Logging Points

1. **Command Execution**: All Command::execute() implementations should log:
   - Command type and parameters (structured fields)
   - Execution success or failure
   - Key business metrics (costs, quantities, etc.)

2. **Event Generation**: Events should be logged when:
   - Emitted from commands (implicit via command logging)
   - Applied to domain state (in reducers/services)
   - Persisted to event store

3. **Validation Failures**: Log validation errors with context:
   - Command that failed
   - Validation rule violated
   - Input values that caused failure

### Structured Logging Format

Use `tracing` macros with structured fields:

```rust
use tracing::{info, warn, error, debug};

// Good: structured fields
info!(
    colony_id = ?self.colony_id,
    resource_type = ?self.resource_type,
    quantity = self.quantity,
    "Resource extraction completed"
);

// Avoid: string interpolation only
info!("Resource extraction completed for colony {}", self.colony_id);
```

## Command Logging Audit

### Colony Commands (`src/commands/colony_commands.rs`)

| Command | Has Logging? | Location | Status | Notes |
|---------|--------------|----------|--------|-------|
| `FoundColony` | ❌ No | execute() | **MISSING** | Should log colony_id, planet_id, name, starting_resources |
| `ConstructBuilding` | ❌ No | execute() | **MISSING** | Should log building_id, colony_id, building_type, cost |
| `AdvanceTurn` | ❌ No | execute() | **MISSING** | Should log colony_id, turn_number, resources_before, resources_after |
| `AllocateLabor` | ❌ No | execute() | **MISSING** | Should log colony_id, building_id, workers_allocated |
| `DeallocateLabor` | ❌ No | execute() | **MISSING** | Should log colony_id, building_id, workers_deallocated |
| `ChangeBuildingState` | ❌ No | execute() | **MISSING** | Should log building_id, colony_id, old_state, new_state |
| `UpgradeBuilding` | ❌ No | execute() | **MISSING** | Should log building_id, colony_id, old_level, new_level, cost |
| `RepairBuilding` | ❌ No | execute() | **MISSING** | Should log building_id, colony_id, damage_severity, repair_cost |
| `SetRecipe` | ❌ No | execute() | **MISSING** | Should log building_id, colony_id, old_recipe_index, new_recipe_index |

**Total Colony Commands**: 9  
**With Logging**: 0 (0%)  
**Missing Logging**: 9 (100%)

### Trading Commands (`src/commands/trading_commands.rs`)

| Command | Has Logging? | Location | Status | Notes |
|---------|--------------|----------|--------|-------|
| `BuyResource` | ✅ Yes | execute() line 75 | **OK** | Logs "Trade executed: BUY {quantity} {resource_type} @ {price}" |
| `SellResource` | ✅ Yes | execute() line 120 | **OK** | Logs "Trade executed: SELL {quantity} {resource_type} @ {price}" |

**Total Trading Commands**: 2  
**With Logging**: 2 (100%)  
**Missing Logging**: 0 (0%)

**Recommendation**: Trading commands have good coverage, but should add structured fields (colony_id, resource_type as fields, not just string interpolation).

### Banking Commands (`src/commands/banking_commands.rs`)

| Command | Has Logging? | Location | Status | Notes |
|---------|--------------|----------|--------|-------|
| `TakeLoan` | ❌ No | execute() | **MISSING** | Should log loan_id, colony_id, principal, interest_rate, term_turns |
| `RepayLoan` | ✅ Yes | execute() line 90 | **PARTIAL** | Logs payment amount and remaining principal, but missing loan_id, colony_id fields |

**Total Banking Commands**: 2  
**With Logging**: 1 (50%)  
**Missing Logging**: 1 (50%)

**Recommendation**: `RepayLoan` has string-only logging; should use structured fields. `TakeLoan` completely missing.

## Event Logging Audit

Events are currently NOT logged when emitted or applied. This is a significant gap.

### Colony Events

**EventType variants** (from `src/events/event.rs`):

| Event | Current Logging | Status | Recommendation |
|-------|-----------------|--------|----------------|
| `ColonyFounded` | ❌ None | **MISSING** | Log when applied to state: colony_id, planet_id, name |
| `BuildingConstructionStarted` | ❌ None | **MISSING** | Log building_id, colony_id, building_type, turn_number |
| `BuildingConstructionCompleted` | ❌ None | **MISSING** | Log building_id, colony_id, completion_turn |
| `BuildingStateChanged` | ❌ None | **MISSING** | Log building_id, new_state, reason |
| `ResourcesExtracted` | ❌ None | **MISSING** | Log colony_id, resource_type, amount, extraction_rate |
| `ResourcesConsumed` | ❌ None | **MISSING** | Log colony_id, resource_type, amount, consumer (building/system) |
| `ResourcesProduced` | ❌ None | **MISSING** | Log colony_id, resource_type, amount, producer |
| `PopulationChanged` | ❌ None | **MISSING** | Log colony_id, old_population, new_population |
| `PowerGridUpdated` | ❌ None | **MISSING** | Log colony_id, generation, consumption, deficit/surplus |
| `PopulationGrew` | ❌ None | **MISSING** | Log colony_id, old_population, new_population, growth_rate |
| `LaborAllocated` | ❌ None | **MISSING** | Log colony_id, building_id, workers_allocated |
| `LaborDeallocated` | ❌ None | **MISSING** | Log colony_id, building_id, workers_deallocated |
| `BuildingDamaged` | ❌ None | **MISSING** | Log building_id, colony_id, damage_severity, cause |
| `BuildingRepaired` | ❌ None | **MISSING** | Log building_id, colony_id, repair_cost |
| `BuildingUpgraded` | ❌ None | **MISSING** | Log building_id, colony_id, old_level, new_level |
| `BuildingRecipeChanged` | ❌ None | **MISSING** | Log building_id, colony_id, old_recipe_index, new_recipe_index |

### Trading Events

| Event | Current Logging | Status | Recommendation |
|-------|-----------------|--------|----------------|
| `ResourceTraded` | ❌ None | **MISSING** | Log colony_id, resource_type, quantity, price, side |
| `MarketPriceChanged` | ❌ None | **MISSING** | Log resource_type, old_price, new_price, volatility |

### Banking Events

| Event | Current Logging | Status | Recommendation |
|-------|-----------------|--------|----------------|
| `LoanIssued` | ❌ None | **MISSING** | Log loan_id, colony_id, principal, interest_rate, term_turns |
| `LoanPaymentMade` | ❌ None | **MISSING** | Log loan_id, payment_amount, remaining_principal, turn_number |
| `LoanDefaulted` | ❌ None | **MISSING** | Log loan_id, colony_id, defaulted_amount, penalty |

**Total Events**: 22  
**With Logging**: 0 (0%)  
**Missing Logging**: 22 (100%)

## Validation Logging Audit

Validation errors are NOT currently logged. When commands fail validation, the error is returned but not logged, making debugging difficult.

### Recommendations

1. **Add logging to Command::validate()** - Log validation failures with context:

   ```rust
   fn validate(&self) -> Result<(), Self::Error> {
       if self.quantity <= 0 {
           warn!(
               colony_id = ?self.colony_id,
               quantity = self.quantity,
               "Validation failed: quantity must be positive"
           );
           return Err(TradingError::InvalidCommand(...));
       }
       Ok(())
   }
   ```

2. **Use debug! for successful validation** to avoid noise in production:

   ```rust
   fn validate(&self) -> Result<(), Self::Error> {
       // ... validation checks ...
       debug!(colony_id = ?self.colony_id, "Command validation passed");
       Ok(())
   }
   ```

## Overall Statistics

| Category | Total Items | With Logging | Percentage | Status |
|----------|-------------|--------------|------------|--------|
| Colony Commands | 9 | 0 | 0% | ❌ Critical Gap |
| Trading Commands | 2 | 2 | 100% | ✅ Good (needs structured fields) |
| Banking Commands | 2 | 1 | 50% | ⚠️ Partial |
| All Commands | 13 | 3 | 23% | ❌ Poor Coverage |
| Events | 22 | 0 | 0% | ❌ Critical Gap |
| **Total** | **35** | **3** | **9%** | **❌ Requires Immediate Action** |

## Priority Recommendations

### High Priority (P0) - Implement Immediately

1. **Add logging to all Colony Commands** (9 commands)
   - These are core game mechanics; logging essential for debugging
   - Should include structured fields for all key parameters

2. **Add event application logging** (22 events)
   - Log when events are applied to domain state
   - Critical for audit trail and debugging state inconsistencies

3. **Fix Trading/Banking command logging**
   - Convert string interpolation to structured fields
   - Add missing fields (colony_id, loan_id, etc.)

### Medium Priority (P1) - Next Sprint

1. **Add validation logging**
   - Warn level for validation failures
   - Debug level for successful validations

2. **Add command dispatch logging**
   - Log in command handler before validation
   - Track command frequency and patterns

### Low Priority (P2) - Future Enhancement

1. **Add operation correlation IDs**
   - Track multi-command operations (e.g., turn processing)
   - Correlate commands → events → state changes

2. **Add performance metrics**
   - Log command execution time
   - Track slow commands for optimization

## Implementation Plan

### Phase 1: Core Command Logging (Task 7.6.2)

Add structured logging to all Command::execute() implementations:

**Files to modify**:

- `src/commands/colony_commands.rs` - 9 commands
- `src/commands/trading_commands.rs` - 2 commands (improve existing)
- `src/commands/banking_commands.rs` - 2 commands (add/improve)

**Example pattern**:

```rust
impl Command for FoundColony {
    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;
        
        info!(
            colony_id = ?self.colony_id,
            planet_id = ?self.planet_id,
            name = %self.name,
            "Colony founded"
        );
        
        Ok(vec![EventType::ColonyFounded { ... }])
    }
}
```

### Phase 2: Event Logging (Task 7.6.3)

Add logging when events are applied to domain state.

**Options**:

1. Log in event store when persisting
2. Log in service layer when applying to state
3. Add Event::log() method to EventType

**Recommendation**: Log in service layer (closer to business logic).

### Phase 3: Validation Logging (Task 7.6.7)

Add logging to validation methods.

### Phase 4: Tests (Task 7.6.1, 7.6.7)

Create tests to enforce logging:

- Property test: all Commands must call a tracing macro
- Integration test: execute each command, verify logs emitted
- Regression test: fail if new Command added without logging

## Testing Strategy

### Unit Tests

Test each Command emits expected logs:

```rust
#[test]
fn found_colony_emits_log() {
    let subscriber = tracing_subscriber::fmt()
        .with_test_writer()
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    
    let cmd = FoundColony { ... };
    let _ = cmd.execute();
    
    // Assert log contains expected fields
    // (requires test log capture infrastructure)
}
```

### Property Tests

Ensure all Commands have logging:

```rust
#[test]
fn all_commands_emit_logs() {
    // For each Command type, verify execute() calls at least one tracing macro
    // Could use macro-based introspection or runtime log capture
}
```

### Integration Tests

Full command → event → state flow with log verification:

```rust
#[test]
fn command_execution_logging_flow() {
    // Execute command
    // Verify command log emitted
    // Verify event log emitted
    // Verify state change log emitted
}
```

## Log Level Guidelines

- **info!**: Successful command execution, important state changes
- **warn!**: Validation failures, recoverable errors
- **error!**: Command execution failures, critical errors
- **debug!**: Detailed debugging info, successful validation
- **trace!**: Very verbose info (rarely needed)

## Example Logging Implementations

### Good Example (Trading Commands)

```rust
info!(
    "Trade executed: BUY {} {:?} @ {}",
    self.quantity, self.resource_type, self.market_price.price
);
```

**Improvement needed**: Use structured fields instead of string interpolation:

```rust
info!(
    colony_id = ?self.colony_id,
    resource_type = ?self.resource_type,
    quantity = self.quantity,
    price = self.market_price.price,
    side = "BUY",
    "Trade executed"
);
```

### Missing Example (Colony Commands)

Currently:

```rust
impl Command for FoundColony {
    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;
        // No logging!
        Ok(vec![EventType::ColonyFounded { ... }])
    }
}
```

Should be:

```rust
impl Command for FoundColony {
    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;
        
        info!(
            colony_id = ?self.colony_id,
            planet_id = ?self.planet_id,
            name = %self.name,
            starting_resources = ?Resources::starting_resources(),
            "Colony founded"
        );
        
        Ok(vec![EventType::ColonyFounded { ... }])
    }
}
```

## Conclusion

The Outpost-3 codebase has **significant logging gaps** with only 9% coverage across Commands and Events. Immediate action required:

1. **Add logging to all 9 Colony Commands** (highest priority)
2. **Implement event application logging** (22 events)
3. **Improve existing Trading/Banking logs** (structured fields)
4. **Add validation logging** (prevent silent failures)
5. **Create tests to enforce logging** (prevent regressions)

This audit provides the foundation for Tasks 7.6.2 (Structured Logging for Commands) and 7.6.3 (Event Logging).
