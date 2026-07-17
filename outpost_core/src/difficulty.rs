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

/// One of five difficulty presets spanning sandbox → brutal, plus a
/// `Custom` variant that indicates the active [`DifficultyScalar`] was
/// assembled from user-chosen slider values rather than a grade-table row.
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
    /// User-authored scalar map (from the custom-difficulty panel).
    ///
    /// The grade table is *not* consulted for this preset; the caller
    /// supplies the [`DifficultyScalar`] via `Command::SetCustomDifficulty`.
    Custom,
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
            DifficultyPreset::Custom => "Custom",
        }
    }

    /// The five built-in presets, in canonical order.
    ///
    /// Excludes [`DifficultyPreset::Custom`], which is not a grade-table row.
    pub const BUILTIN: [DifficultyPreset; 5] = [
        DifficultyPreset::Sandbox,
        DifficultyPreset::Easy,
        DifficultyPreset::Normal,
        DifficultyPreset::Hard,
        DifficultyPreset::Brutal,
    ];
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
    ///
    /// [`DifficultyPreset::Custom`] falls back to the `Normal` column — the
    /// caller is expected to supply an explicit [`DifficultyScalar`] rather
    /// than materialising one from the grade table.
    #[must_use]
    pub fn scalar_for(&self, preset: DifficultyPreset) -> f32 {
        match preset {
            DifficultyPreset::Sandbox => self.sandbox,
            DifficultyPreset::Easy => self.easy,
            DifficultyPreset::Normal | DifficultyPreset::Custom => self.normal,
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
#[allow(clippy::too_many_lines)]
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

    // Hazard trigger probability — harder difficulty raises hazard frequency
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::HazardProbability,
        sandbox: 0.50,
        easy: 0.75,
        normal: 1.00,
        hard: 1.40,
        brutal: 2.00,
    });

    // Slot capacity — harder difficulty reduces available build slots.
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::SlotCapacity,
        sandbox: 1.50,
        easy: 1.20,
        normal: 1.00,
        hard: 0.85,
        brutal: 0.70,
    });

    // Resource consumption — harder difficulty increases per-capita drain.
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::ResourceConsumption,
        sandbox: 0.70,
        easy: 0.85,
        normal: 1.00,
        hard: 1.15,
        brutal: 1.30,
    });

    // Research cost — harder difficulty inflates the cost of every tech.
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::ResearchCost,
        sandbox: 0.70,
        easy: 0.85,
        normal: 1.00,
        hard: 1.20,
        brutal: 1.50,
    });

    // Power requirement — harder difficulty makes buildings/recipes hungrier.
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::PowerRequirement,
        sandbox: 0.70,
        easy: 0.85,
        normal: 1.00,
        hard: 1.15,
        brutal: 1.30,
    });

    // Maintenance consumption (issue #180) — harder difficulty inflates the
    // per-sol upkeep drain of every authored `BuildingDef::maintenance` entry.
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::MaintenanceConsumption,
        sandbox: 0.70,
        easy: 0.85,
        normal: 1.00,
        hard: 1.15,
        brutal: 1.30,
    });

    // Deposit abundance (issue #232) — harder difficulty means leaner system
    // generation; easier difficulty is more generous. Coverage (every curated
    // commodity existing *somewhere*) is unaffected by this — only how
    // generous the placed deposits read; see `system_gen::generate_system`'s
    // doc comment. Sandbox intentionally goes well past 1.0 ("effectively
    // abundant") per the user's "could be made infinite at some difficulty
    // levels" framing — modeled as a large generosity multiplier rather than
    // literal infinite/unlimited stock, since deposits carry no depletion
    // mechanic to make "infinite" a meaningful distinct state (see this
    // module's PR description for why true infinite-deposit gating is
    // explicitly deferred, not built here).
    table.add_row(DifficultyGradeRow {
        quantity: ModifiableQuantity::DepositAbundance,
        sandbox: 3.00,
        easy: 1.50,
        normal: 1.00,
        hard: 0.70,
        brutal: 0.50,
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

    #[test]
    fn default_grade_table_includes_hazard_probability() {
        let table = default_grade_table();
        let q = ModifiableQuantity::HazardProbability;
        let hard = table.build_scalar(DifficultyPreset::Hard);
        let easy = table.build_scalar(DifficultyPreset::Easy);
        // Hard difficulty should have a higher hazard probability scalar than Easy.
        assert!(
            hard.scalar_for(&q) > easy.scalar_for(&q),
            "Hard hazard probability scalar ({}) should exceed Easy ({})",
            hard.scalar_for(&q),
            easy.scalar_for(&q)
        );
    }

    #[test]
    fn hard_has_lower_growth_scalar_than_easy() {
        let table = default_grade_table();
        let q = ModifiableQuantity::PopulationGrowth;
        let hard = table.build_scalar(DifficultyPreset::Hard);
        let easy = table.build_scalar(DifficultyPreset::Easy);
        assert!(
            easy.scalar_for(&q) > hard.scalar_for(&q),
            "Easy growth scalar ({}) should exceed Hard ({})",
            easy.scalar_for(&q),
            hard.scalar_for(&q)
        );
    }

    #[test]
    fn hard_difficulty_increases_hazard_rate_in_pipeline() {
        use crate::colony::Colony;
        use crate::hazard::{HazardConfig, HazardEntry, HazardKindConfig};
        use crate::turn::{GameState, TurnProcessor};

        let base_prob = 0.3_f32;
        let kinds_entries: Vec<HazardEntry> = crate::hazard::HazardKind::ALL
            .iter()
            .map(|&kind| HazardEntry {
                kind,
                config: HazardKindConfig {
                    base_probability: base_prob,
                    severity_min: 0.5,
                    severity_max: 0.5,
                    stability_damage_per_severity: 0.01,
                    commodity_loss_per_severity: 0.0,
                    population_damage_per_severity: 0.0,
                },
                terrain_modifiers: Default::default(),
            })
            .collect();
        let hazard_cfg = HazardConfig {
            kinds: kinds_entries,
        };

        // Run many sols on Hard and count hazard occurrences.
        let count_hazards = |preset: DifficultyPreset| -> usize {
            let mut state = GameState::new();
            state.add_colony(Colony::new("Test"), 1000);
            state.hazard_config = Some(hazard_cfg.clone());
            let table = default_grade_table();
            state.difficulty_preset = preset;
            state.difficulty_scalar = table.build_scalar(preset);
            let mut proc = TurnProcessor::new(12345);
            let mut total = 0usize;
            for _ in 0..200 {
                let out = proc.advance(&mut state);
                total += out.hazard_outcomes.len();
            }
            total
        };

        let hard_count = count_hazards(DifficultyPreset::Hard);
        let easy_count = count_hazards(DifficultyPreset::Easy);
        assert!(
            hard_count > easy_count,
            "Hard difficulty should produce more hazards than Easy (hard={hard_count}, easy={easy_count})"
        );
    }

    #[test]
    fn easy_difficulty_produces_faster_growth() {
        use crate::population::PopulationPool;

        let table = default_grade_table();
        let easy_scalar = table
            .build_scalar(DifficultyPreset::Easy)
            .scalar_for(&ModifiableQuantity::PopulationGrowth);
        let hard_scalar = table
            .build_scalar(DifficultyPreset::Hard)
            .scalar_for(&ModifiableQuantity::PopulationGrowth);

        let mut easy_pop = PopulationPool::new(1000.0);
        let mut hard_pop = PopulationPool::new(1000.0);

        for _ in 0..10 {
            easy_pop.apply_growth_tick_with_scalar(easy_scalar);
            hard_pop.apply_growth_tick_with_scalar(hard_scalar);
        }

        assert!(
            easy_pop.count > hard_pop.count,
            "Easy difficulty should produce faster growth (easy={}, hard={})",
            easy_pop.count,
            hard_pop.count
        );
    }

    #[test]
    fn set_difficulty_accepted_after_first_turn() {
        use crate::{Command, GameEngine};

        // Post-#161: SetDifficulty is now allowed at any sol so the mid-game
        // custom-difficulty menu can retune the game without a fresh start.
        let mut engine = GameEngine::new();
        assert!(engine
            .apply(&Command::SetDifficulty {
                preset: DifficultyPreset::Hard
            })
            .is_ok());
        engine
            .apply(&Command::AdvanceColonySol)
            .expect("first turn should advance");
        assert!(
            engine
                .apply(&Command::SetDifficulty {
                    preset: DifficultyPreset::Easy
                })
                .is_ok(),
            "SetDifficulty must remain accepted after sol > 0"
        );
    }

    #[test]
    fn default_grade_table_includes_new_161_rows() {
        let table = default_grade_table();
        let normal = table.build_scalar(DifficultyPreset::Normal);
        // Every new row should resolve to 1.0 at Normal (baseline).
        for q in [
            ModifiableQuantity::SlotCapacity,
            ModifiableQuantity::ResourceConsumption,
            ModifiableQuantity::ResearchCost,
            ModifiableQuantity::PowerRequirement,
            ModifiableQuantity::MaintenanceConsumption,
        ] {
            let n = normal.scalar_for(&q);
            assert!(
                (n - 1.0).abs() < 1e-4,
                "{q:?} should be 1.0 at Normal, got {n}"
            );
        }
    }

    #[test]
    fn sandbox_deposit_abundance_is_more_generous_than_brutal() {
        let table = default_grade_table();
        let q = ModifiableQuantity::DepositAbundance;
        let sandbox = table.build_scalar(DifficultyPreset::Sandbox).scalar_for(&q);
        let brutal = table.build_scalar(DifficultyPreset::Brutal).scalar_for(&q);
        assert!(
            sandbox > brutal,
            "Sandbox deposit abundance ({sandbox}) must exceed Brutal ({brutal})"
        );
    }

    #[test]
    fn brutal_resource_consumption_is_harsher_than_easy() {
        let table = default_grade_table();
        let q = ModifiableQuantity::ResourceConsumption;
        let brutal = table.build_scalar(DifficultyPreset::Brutal).scalar_for(&q);
        let easy = table.build_scalar(DifficultyPreset::Easy).scalar_for(&q);
        assert!(
            brutal > easy,
            "Brutal consumption ({brutal}) must exceed Easy ({easy})"
        );
    }

    #[test]
    fn brutal_research_cost_is_higher_than_sandbox() {
        let table = default_grade_table();
        let q = ModifiableQuantity::ResearchCost;
        let brutal = table.build_scalar(DifficultyPreset::Brutal).scalar_for(&q);
        let sandbox = table.build_scalar(DifficultyPreset::Sandbox).scalar_for(&q);
        assert!(brutal > sandbox);
    }

    #[test]
    fn brutal_power_requirement_is_higher_than_normal() {
        let table = default_grade_table();
        let q = ModifiableQuantity::PowerRequirement;
        let brutal = table.build_scalar(DifficultyPreset::Brutal).scalar_for(&q);
        let normal = table.build_scalar(DifficultyPreset::Normal).scalar_for(&q);
        assert!(brutal > normal);
    }

    #[test]
    fn resource_consumption_scalar_scales_pool_drain() {
        use crate::colony::ColonyPool;
        use crate::needs::{apply_needs_check_scaled, NeedsConfig};

        let cfg = NeedsConfig::default_survival();
        let mut pool_normal = ColonyPool::new();
        pool_normal.deposit("food_ration", 1_000.0);
        pool_normal.deposit("water", 1_000.0);
        pool_normal.deposit("oxygen", 1_000.0);
        pool_normal.deposit("power", 1_000.0);
        pool_normal.deposit("housing", 1_000.0);

        let mut pool_brutal = pool_normal.clone();

        let report_normal = apply_needs_check_scaled(&mut pool_normal, 100.0, &cfg, 1.0);
        let report_brutal = apply_needs_check_scaled(&mut pool_brutal, 100.0, &cfg, 1.30);

        // Brutal consumption scalar must have drawn more food.
        let food_normal = report_normal
            .needs
            .iter()
            .find(|n| n.commodity_id == "food_ration")
            .map(|n| n.consumed)
            .unwrap();
        let food_brutal = report_brutal
            .needs
            .iter()
            .find(|n| n.commodity_id == "food_ration")
            .map(|n| n.consumed)
            .unwrap();
        assert!(
            food_brutal > food_normal,
            "brutal food consumption ({food_brutal}) must exceed normal ({food_normal})"
        );
        // Ratio should be ~1.30 to first order.
        let ratio = food_brutal / food_normal;
        assert!((ratio - 1.30).abs() < 0.02, "ratio {ratio} not ≈ 1.30");
    }

    #[test]
    fn research_cost_scalar_slows_completion() {
        use crate::research::SystemResearchPool;
        use crate::tech::{apply_research_turn_scaled, TechDef, TechRegistry, TechState};

        // Two identical setups, one with cost_scalar 1.0, one with 2.0.
        let def = TechDef {
            id: "t".into(),
            display_name: "t".into(),
            research_cost: 100.0,
            ..Default::default()
        };
        let reg = TechRegistry::build(vec![def]).expect("registry");

        let fresh = || {
            let mut s = TechState::new();
            s.current_project = Some("t".into());
            let mut pool = SystemResearchPool::new();
            pool.deposit(150.0);
            (s, pool)
        };

        let (mut s1, mut p1) = fresh();
        let r1 = apply_research_turn_scaled(&mut s1, &mut p1, &reg, 1.0);
        assert_eq!(
            r1.completed,
            vec!["t".to_string()],
            "cost 100 with 150 available should complete"
        );

        let (mut s2, mut p2) = fresh();
        let r2 = apply_research_turn_scaled(&mut s2, &mut p2, &reg, 2.0);
        assert!(
            r2.completed.is_empty(),
            "cost 200 with 150 available must NOT complete"
        );
    }

    #[test]
    fn power_requirement_scalar_raises_demand() {
        use crate::colony::{process_production_scaled, ColonyPool};
        use crate::content::types::{BuildingCategory, BuildingDef, Ingredient, RecipeDef};
        use crate::content::ContentRegistry;

        let bdef = BuildingDef {
            id: "smelter".into(),
            name: "Smelter".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            worker_slots: 1,
            power_delta: 50.0, // consumer
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        };
        let recipe = RecipeDef {
            id: "smelt".into(),
            name: "Smelt".into(),
            building: "smelter".into(),
            inputs: vec![Ingredient {
                id: "ore".into(),
                quantity: 1.0,
            }],
            outputs: vec![Ingredient {
                id: "metal".into(),
                quantity: 1.0,
            }],
            cycle_sols: 1,
            power_draw: 50.0,
        };

        let mut reg = ContentRegistry::default();
        reg.insert_building(bdef);
        reg.insert_recipe(recipe);

        let placed = vec![("smelter".to_string(), 1u32)];
        let mut pool_normal = ColonyPool::new();
        pool_normal.deposit("ore", 100.0);
        let mut pool_hard = pool_normal.clone();

        let normal = process_production_scaled(
            &mut pool_normal,
            &placed,
            10.0,
            &reg,
            1.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
        );
        let hard = process_production_scaled(
            &mut pool_hard,
            &placed,
            10.0,
            &reg,
            2.0,
            1.0,
            true,
            1.0,
            &std::collections::HashMap::new(),
            &[],
        );

        assert!(
            hard.power_grid.demand > normal.power_grid.demand,
            "hard demand ({}) should exceed normal ({})",
            hard.power_grid.demand,
            normal.power_grid.demand
        );
        // Ratio should be ~2.0.
        let ratio = hard.power_grid.demand / normal.power_grid.demand;
        assert!((ratio - 2.0).abs() < 0.05, "ratio {ratio} not ≈ 2.0");
    }

    #[test]
    fn set_custom_difficulty_installs_scalar_and_marks_preset_custom() {
        use crate::modifier::{DifficultyScalar, ModifiableQuantity};
        use crate::{Command, GameEngine};

        let mut scalars = DifficultyScalar::new();
        scalars.set(ModifiableQuantity::ResourceConsumption, 1.7);

        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::SetCustomDifficulty {
                scalars,
                menace_enabled: false,
                hazards_enabled: false,
                maintenance_enabled: true,
            })
            .expect("SetCustomDifficulty");
        assert!(matches!(
            events.first(),
            Some(crate::Event::DifficultyChanged {
                preset: DifficultyPreset::Custom
            })
        ));
        assert_eq!(engine.state.difficulty_preset, DifficultyPreset::Custom);
        assert!(!engine.state.hazards_enabled);
        assert!(
            (engine
                .state
                .difficulty_scalar
                .scalar_for(&ModifiableQuantity::ResourceConsumption)
                - 1.7)
                .abs()
                < 1e-4
        );
    }

    #[test]
    fn custom_difficulty_toggle_reattaches_last_menace() {
        use crate::menace::{FinalSemantics, MenaceDefinition};
        use crate::modifier::DifficultyScalar;
        use crate::{Command, GameEngine};

        let def = MenaceDefinition {
            id: "resource-crunch".into(),
            name: "Resource Crunch".into(),
            phases: vec![],
            final_semantics: FinalSemantics::ProductionCollapse,
        };
        let mut engine = GameEngine::new();
        engine
            .apply(&Command::ActivateMenace {
                definition: Some(def),
            })
            .expect("activate");
        assert!(engine.state.menace_state.is_some());

        // Turn menace off via custom-difficulty.
        engine
            .apply(&Command::SetCustomDifficulty {
                scalars: DifficultyScalar::new(),
                menace_enabled: false,
                hazards_enabled: true,
                maintenance_enabled: true,
            })
            .expect("custom off");
        assert!(engine.state.menace_state.is_none());

        // Turn it back on — should re-attach from last_menace_definition.
        engine
            .apply(&Command::SetCustomDifficulty {
                scalars: DifficultyScalar::new(),
                menace_enabled: true,
                hazards_enabled: true,
                maintenance_enabled: true,
            })
            .expect("custom on");
        assert!(
            engine.state.menace_state.is_some(),
            "menace should be re-attached from last_menace_definition"
        );
    }

    #[test]
    fn set_hazards_enabled_toggles_state() {
        use crate::modifier::DifficultyScalar;
        use crate::{Command, GameEngine};

        let mut engine = GameEngine::new();
        assert!(engine.state.hazards_enabled);
        engine
            .apply(&Command::SetHazardsEnabled { enabled: false })
            .expect("toggle");
        assert!(!engine.state.hazards_enabled);
        // Custom difficulty should also drive the toggle atomically.
        engine
            .apply(&Command::SetCustomDifficulty {
                scalars: DifficultyScalar::new(),
                menace_enabled: false,
                hazards_enabled: true,
                maintenance_enabled: true,
            })
            .expect("custom");
        assert!(engine.state.hazards_enabled);
    }

    #[test]
    fn set_custom_difficulty_propagates_maintenance_enabled_atomically() {
        // Issue #180: the atomic panel apply must carry maintenance_enabled
        // alongside menace/hazards toggles.
        use crate::modifier::DifficultyScalar;
        use crate::{Command, GameEngine};

        let mut engine = GameEngine::new();
        assert!(engine.state.maintenance_enabled);

        engine
            .apply(&Command::SetCustomDifficulty {
                scalars: DifficultyScalar::new(),
                menace_enabled: false,
                hazards_enabled: true,
                maintenance_enabled: false,
            })
            .expect("custom");
        assert!(!engine.state.maintenance_enabled);

        engine
            .apply(&Command::SetCustomDifficulty {
                scalars: DifficultyScalar::new(),
                menace_enabled: false,
                hazards_enabled: true,
                maintenance_enabled: true,
            })
            .expect("custom on");
        assert!(engine.state.maintenance_enabled);
    }

    #[test]
    fn set_maintenance_enabled_toggles_state() {
        // Issue #180: standalone toggle mirrors SetHazardsEnabled.
        use crate::{Command, GameEngine};

        let mut engine = GameEngine::new();
        assert!(engine.state.maintenance_enabled);
        engine
            .apply(&Command::SetMaintenanceEnabled { enabled: false })
            .expect("toggle off");
        assert!(!engine.state.maintenance_enabled);
        engine
            .apply(&Command::SetMaintenanceEnabled { enabled: true })
            .expect("toggle on");
        assert!(engine.state.maintenance_enabled);
    }

    #[test]
    fn brutal_maintenance_consumption_is_harsher_than_easy() {
        let table = default_grade_table();
        let q = ModifiableQuantity::MaintenanceConsumption;
        let brutal = table.build_scalar(DifficultyPreset::Brutal).scalar_for(&q);
        let easy = table.build_scalar(DifficultyPreset::Easy).scalar_for(&q);
        assert!(
            brutal > easy,
            "Brutal maintenance ({brutal}) must exceed Easy ({easy})"
        );
    }

    #[test]
    fn custom_preset_falls_back_to_normal_in_grade_row() {
        // The Custom variant is never resolved through the grade table in
        // practice, but if it is, it should map to the Normal column.
        let row = DifficultyGradeRow {
            quantity: ModifiableQuantity::LaborEfficiency,
            sandbox: 1.5,
            easy: 1.2,
            normal: 1.0,
            hard: 0.8,
            brutal: 0.6,
        };
        assert!((row.scalar_for(DifficultyPreset::Custom) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn yaml_grade_table_roundtrip() {
        let yaml = serde_yaml::to_string(&default_grade_table()).expect("should serialise");
        let back: DifficultyGradeTable = serde_yaml::from_str(&yaml).expect("should deserialise");
        assert_eq!(back.rows.len(), default_grade_table().rows.len());
    }
}
