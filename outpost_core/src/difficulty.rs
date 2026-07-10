//! Difficulty grade-tables — outermost per-quantity scalar per difficulty preset.
//!
//! Five presets span the design spectrum (§10): Sandbox → Easy → Normal → Hard → Brutal.
//! The difficulty scalar is applied last in `resolve()`:
//! ```text
//! effective = base × (1 + Σ tech_bonuses) × difficulty_scalar
//! ```
//! Grade tables are authored as data; the preset merely selects a named entry.

use serde::{Deserialize, Serialize};

use crate::modifier::{DifficultyScalar, ModifiableQuantity};

// ─── DifficultyPreset ────────────────────────────────────────────────────────

/// One of five difficulty presets spanning sandbox → brutal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DifficultyPreset {
    /// No penalties; ideal for exploration and creative play.
    Sandbox,
    /// Mild penalties; forgiving for new players.
    Easy,
    /// Balanced challenge; the intended experience.
    #[default]
    Normal,
    /// Significant penalties; for experienced players.
    Hard,
    /// Maximum penalties; near-unforgiving resource pressure.
    Brutal,
}

impl DifficultyPreset {
    /// Human-readable label for the preset.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            DifficultyPreset::Sandbox => "Sandbox",
            DifficultyPreset::Easy => "Easy",
            DifficultyPreset::Normal => "Normal",
            DifficultyPreset::Hard => "Hard",
            DifficultyPreset::Brutal => "Brutal",
        }
    }
}

// ─── DifficultyGradeRow ───────────────────────────────────────────────────────

/// One row in a grade table: a scalar per [`DifficultyPreset`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyGradeRow {
    /// The quantity this row scales.
    pub quantity: ModifiableQuantity,
    /// Scalars indexed by preset (sandbox, easy, normal, hard, brutal).
    pub sandbox: f32,
    /// Easy preset scalar.
    pub easy: f32,
    /// Normal preset scalar.
    pub normal: f32,
    /// Hard preset scalar.
    pub hard: f32,
    /// Brutal preset scalar.
    pub brutal: f32,
}

impl DifficultyGradeRow {
    /// Return the scalar for a given [`DifficultyPreset`].
    #[must_use]
    pub fn scalar_for(&self, preset: DifficultyPreset) -> f32 {
        match preset {
            DifficultyPreset::Sandbox => self.sandbox,
            DifficultyPreset::Easy => self.easy,
            DifficultyPreset::Normal => self.normal,
            DifficultyPreset::Hard => self.hard,
            DifficultyPreset::Brutal => self.brutal,
        }
    }
}

// ─── DifficultyGradeTable ────────────────────────────────────────────────────

/// Complete grade table mapping each [`ModifiableQuantity`] to its per-preset scalars.
///
/// Loaded from content data; use [`DifficultyGradeTable::build_scalar`] to
/// materialise a [`DifficultyScalar`] for a chosen preset.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DifficultyGradeTable {
    rows: Vec<DifficultyGradeRow>,
}

impl DifficultyGradeTable {
    /// Construct an empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a row to the table.
    pub fn add_row(&mut self, row: DifficultyGradeRow) {
        self.rows.push(row);
    }

    /// Materialise a [`DifficultyScalar`] for the given [`DifficultyPreset`].
    ///
    /// Only quantities with rows in this table are overridden; all others
    /// default to `1.0` (the [`DifficultyScalar`] default).
    #[must_use]
    pub fn build_scalar(&self, preset: DifficultyPreset) -> DifficultyScalar {
        let mut scalar = DifficultyScalar::new();
        for row in &self.rows {
            scalar.set(row.quantity.clone(), row.scalar_for(preset));
        }
        scalar
    }
}

/// Build the canonical default grade table used when no content pack overrides it.
///
/// Values are chosen to represent the intended difficulty curve:
/// - Sandbox: all scalars 1.0 (no penalty).
/// - Easy: mild reductions on pressure quantities (growth, research).
/// - Normal: baseline (1.0 for most, slight challenge on stability).
/// - Hard: noticeable penalties across production and growth.
/// - Brutal: harsh penalties reflecting near-unforgiving pressure.
#[must_use]
pub fn default_grade_table() -> DifficultyGradeTable {
    let mut table = DifficultyGradeTable::new();

    // Production rate — harder difficulty reduces output
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::ProductionRate("*".into()),
        sandbox: 1.20,
        easy: 1.10,
        normal: 1.00,
        hard: 0.85,
        brutal: 0.70,
    });

    // Labour efficiency
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::LaborEfficiency,
        sandbox: 1.20,
        easy: 1.10,
        normal: 1.00,
        hard: 0.85,
        brutal: 0.70,
    });

    // Research rate
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::ResearchRate,
        sandbox: 1.50,
        easy: 1.20,
        normal: 1.00,
        hard: 0.80,
        brutal: 0.60,
    });

    // Population growth
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::PopulationGrowth,
        sandbox: 1.50,
        easy: 1.20,
        normal: 1.00,
        hard: 0.80,
        brutal: 0.60,
    });

    // Stability rate — harder = faster stability decay
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::StabilityRate,
        sandbox: 0.50,
        easy: 0.75,
        normal: 1.00,
        hard: 1.30,
        brutal: 1.60,
    });

    // Storage capacity
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::StorageCapacity,
        sandbox: 1.50,
        easy: 1.20,
        normal: 1.00,
        hard: 0.85,
        brutal: 0.70,
    });

    table
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modifier::{resolve, ModifiableQuantity, ModifierAccumulator};

    fn make_table() -> DifficultyGradeTable {
        let mut t = DifficultyGradeTable::new();
        t.add_row(DifficultyGradeRow {
            quantity: ModifiableQuantity::LaborEfficiency,
            sandbox: 1.50,
            easy: 1.20,
            normal: 1.00,
            hard: 0.80,
            brutal: 0.60,
        });
        t
    }

    #[test]
    fn sandbox_scalar_is_highest() {
        let table = make_table();
        let s = table.build_scalar(DifficultyPreset::Sandbox);
        let b = table.build_scalar(DifficultyPreset::Brutal);
        let q = ModifiableQuantity::LaborEfficiency;
        assert!(s.scalar_for(&q) > b.scalar_for(&q));
    }

    #[test]
    fn five_presets_produce_five_distinct_scalars() {
        let table = make_table();
        let presets = [
            DifficultyPreset::Sandbox,
            DifficultyPreset::Easy,
            DifficultyPreset::Normal,
            DifficultyPreset::Hard,
            DifficultyPreset::Brutal,
        ];
        let q = ModifiableQuantity::LaborEfficiency;
        let scalars: Vec<f32> = presets
            .iter()
            .map(|&p| table.build_scalar(p).scalar_for(&q))
            .collect();
        // Each must be distinct
        for i in 0..scalars.len() {
            for j in (i + 1)..scalars.len() {
                assert!(
                    (scalars[i] - scalars[j]).abs() > 1e-4,
                    "presets {i} and {j} have the same scalar"
                );
            }
        }
    }

    #[test]
    fn normal_preset_is_unit_scalar() {
        let table = make_table();
        let scalar = table.build_scalar(DifficultyPreset::Normal);
        let q = ModifiableQuantity::LaborEfficiency;
        assert!((scalar.scalar_for(&q) - 1.00).abs() < 1e-4);
    }

    #[test]
    fn difficulty_scalar_applied_outermost_in_resolve() {
        let table = default_grade_table();
        let brutal = table.build_scalar(DifficultyPreset::Brutal);
        let q = ModifiableQuantity::LaborEfficiency;

        let mut accum = ModifierAccumulator::new();
        accum.add(crate::modifier::ModifierDescriptor::new(
            q.clone(),
            "tech",
            0.50,
        ));

        // base=100, tech: ×1.50 → 150, brutal scalar (0.70): 150×0.70 = 105
        let result = resolve(100.0, &q, &accum, &brutal);
        assert!((result - 105.0).abs() < 1e-2, "expected 105 got {result}");
    }

    #[test]
    fn default_grade_table_has_five_quantities() {
        let table = default_grade_table();
        // We added rows for 6 quantities (ProductionRate, Labor, Research, Pop, Stability, Storage)
        assert!(table.rows.len() >= 5);
    }

    #[test]
    fn preset_labels_are_non_empty() {
        let presets = [
            DifficultyPreset::Sandbox,
            DifficultyPreset::Easy,
            DifficultyPreset::Normal,
            DifficultyPreset::Hard,
            DifficultyPreset::Brutal,
        ];
        for p in presets {
            assert!(!p.label().is_empty());
        }
    }
}
