/// Log aggregation utilities for complex multi-step operations
/// Task 7.6.5: Provides operation_id correlation for tracking related log entries
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{info, Span};

/// Thread-safe operation ID generator
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for correlating logs across multi-step operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(u64);

impl OperationId {
    /// Generate a new unique operation ID
    pub fn new() -> Self {
        Self(OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the numeric value of the operation ID
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Create an operation ID from a specific value (for testing)
    #[cfg(test)]
    pub fn from_value(value: u64) -> Self {
        Self(value)
    }
}

impl Default for OperationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for OperationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "op-{}", self.0)
    }
}

/// Context for multi-step operations with automatic span tracking
pub struct OperationContext {
    operation_id: OperationId,
    operation_name: String,
    _span: Span,
}

impl OperationContext {
    /// Create a new operation context with a unique operation_id
    ///
    /// # Arguments
    /// * `operation_name` - Human-readable name for the operation
    ///
    /// # Example
    /// ```
    /// use outpost::utils::logging::OperationContext;
    ///
    /// let ctx = OperationContext::new("place_building_with_effects");
    /// // All logs within this context will share the same operation_id
    /// ```
    pub fn new(operation_name: impl Into<String>) -> Self {
        let operation_id = OperationId::new();
        let operation_name = operation_name.into();

        let span = tracing::info_span!(
            "operation",
            operation_id = %operation_id,
            operation_name = %operation_name
        );

        Self {
            operation_id,
            operation_name,
            _span: span,
        }
    }

    /// Get the operation ID for this context
    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    /// Get the operation name
    pub fn operation_name(&self) -> &str {
        &self.operation_name
    }

    /// Log an info message within this operation context
    pub fn info(&self, message: &str) {
        info!(
            operation_id = %self.operation_id,
            operation_name = %self.operation_name,
            "{}",
            message
        );
    }

    /// Log a step within the operation
    pub fn log_step(&self, step_name: &str, details: &str) {
        info!(
            operation_id = %self.operation_id,
            operation_name = %self.operation_name,
            step = step_name,
            "{}",
            details
        );
    }

    /// Log completion of the operation
    pub fn log_complete(&self, result: &str) {
        info!(
            operation_id = %self.operation_id,
            operation_name = %self.operation_name,
            result = result,
            "Operation completed"
        );
    }

    /// Log an error within the operation
    pub fn log_error(&self, error: &str) {
        tracing::error!(
            operation_id = %self.operation_id,
            operation_name = %self.operation_name,
            error = error,
            "Operation failed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_id_unique() {
        let id1 = OperationId::new();
        let id2 = OperationId::new();
        let id3 = OperationId::new();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_operation_id_display() {
        let id = OperationId::from_value(42);
        assert_eq!(format!("{}", id), "op-42");
    }

    #[test]
    fn test_operation_id_value() {
        let id = OperationId::from_value(123);
        assert_eq!(id.value(), 123);
    }

    #[test]
    fn test_operation_context_creation() {
        let ctx = OperationContext::new("test_operation");
        assert_eq!(ctx.operation_name(), "test_operation");
        assert!(ctx.operation_id().value() > 0);
    }

    #[test]
    fn test_operation_contexts_have_unique_ids() {
        let ctx1 = OperationContext::new("operation_1");
        let ctx2 = OperationContext::new("operation_2");

        assert_ne!(ctx1.operation_id(), ctx2.operation_id());
    }

    #[test]
    fn test_operation_context_logging() {
        // This is a smoke test - actual log output would need log capture
        let ctx = OperationContext::new("test_op");
        ctx.info("Starting operation");
        ctx.log_step("step1", "Processing data");
        ctx.log_complete("success");
        // If no panic, logs were emitted successfully
    }

    #[test]
    fn test_operation_id_default() {
        let id1 = OperationId::default();
        let id2 = OperationId::default();
        assert_ne!(id1, id2); // Default should generate unique IDs
    }
}
