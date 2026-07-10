//! System-wide research pool — accumulates research produced by all colonies.
//!
//! Research is a **virtual commodity** produced by `research_lab` buildings
//! each colony-sol turn.  After the production step, the engine drains each
//! colony's `research` commodity balance into the `SystemResearchPool`, which
//! is the spend currency for tech unlocks in Phase 4b.
//!
//! See `docs/DESIGN.md §7A`.

/// System-wide accumulator for research points.
///
/// Fed each turn by draining `research` from every colony's commodity pool.
/// The value here is the spend currency for tech unlocks (Phase 4b).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SystemResearchPool {
    /// Total accumulated research points across all colonies this game.
    pub total: f32,
}

impl SystemResearchPool {
    /// Create a new pool starting at zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Deposit `amount` research points into the pool.
    pub fn deposit(&mut self, amount: f32) {
        if amount > 0.0 {
            self.total += amount;
        }
    }

    /// Current accumulated total.
    #[must_use]
    pub fn total(&self) -> f32 {
        self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_starts_at_zero() {
        let pool = SystemResearchPool::new();
        assert_eq!(pool.total(), 0.0);
    }

    #[test]
    fn deposit_accumulates() {
        let mut pool = SystemResearchPool::new();
        pool.deposit(10.0);
        pool.deposit(5.5);
        assert!((pool.total() - 15.5).abs() < 1e-4);
    }

    #[test]
    fn negative_deposit_is_ignored() {
        let mut pool = SystemResearchPool::new();
        pool.deposit(10.0);
        pool.deposit(-3.0);
        assert!((pool.total() - 10.0).abs() < 1e-4);
    }
}
