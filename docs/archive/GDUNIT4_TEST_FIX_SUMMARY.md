# GDUnit4 Test Fix Summary

## Date
October 11, 2025

## Problem
All 12 out of 15 GDUnit4 tests were failing with symptoms indicating test state was being shared across multiple test methods:

- Tests expecting 2 events were finding 4, 7, 12, 14, etc.
- Tests expecting 3 events were finding 7, 12, etc.
- Tests expecting 0 events (empty store) were finding 17+ events
- Corrupted event errors at line 18 (from accumulated test data)

## Root Cause
GDUnit4's `[Before]` and `[After]` attributes were not providing sufficient test isolation. The test suite classes were being instantiated once and reused across all test methods, causing:

1. **Shared FileEventStore instances** in `EventStoreWorkflowGdTests` - the `_eventStore` field persisted across tests
2. **Test file path collisions** - while Guid.NewGuid() should have created unique paths, the setup/teardown lifecycle wasn't guaranteeing clean state between tests

This differs from xUnit's behavior where each test gets a fresh class instance via the constructor.

## Solution
Refactored both test suites to **create isolated FileEventStore instances within each test method**:

### Changes to `EventStoreWorkflowGdTests.cs`:
- Removed class-level `_testFilePath` and `_eventStore` fields
- Removed `[Before]` and `[After]` methods
- Added helper methods:
  - `CreateUniqueTestFilePath()` - creates unique temp file path
  - `CleanupTestFile(string)` - cleans up test file
- Modified all 5 test methods to:
  - Create unique file path at start
  - Instantiate new FileEventStore
  - Use try-finally to ensure cleanup

### Changes to `FileEventStoreGdTests.cs`:
- Removed class-level `_testFilePath` field
- Removed `[Before]` and `[After]` methods  
- Added same helper methods as above
- Modified all 10 test methods to:
  - Create unique file path at start
  - Instantiate new FileEventStore
  - Use try-finally to ensure cleanup

## Test Results
### Before Fix:
- Total: 15 GDUnit4 tests
- Passed: 3
- Failed: 12

### After Fix:
- Total: 15 GDUnit4 tests
- Passed: 15 ✅
- Failed: 0

### Full Test Suite (xUnit + GDUnit4):
- Total: 30 tests
- Passed: 30 ✅
- Failed: 0

## Key Learnings
1. **GDUnit4 test isolation differs from xUnit** - GDUnit4 reuses test class instances, while xUnit creates new instances per test
2. **Explicitly manage test state** - When using frameworks with shared instances, create and cleanup resources within each test method using try-finally
3. **FileEventStore append behavior is correct** - The store correctly appends to existing files, which is production behavior. Tests must ensure clean state.
4. **Test independence is critical** - Each test must be fully isolated and not depend on execution order or shared state

## Files Modified
- `Tests/GdUnit/EventStoreWorkflowGdTests.cs`
- `Tests/GdUnit/FileEventStoreGdTests.cs`

## Recommendations
For future GDUnit4 tests in this project:
1. Avoid class-level mutable fields that persist across tests
2. Create test resources (files, stores, etc.) within each test method
3. Always use try-finally for cleanup to ensure resources are released even if test fails
4. Prefer test-scoped variables over class-scoped fields when using GDUnit4
