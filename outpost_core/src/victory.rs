//! Victory conditions and sandbox-continue mode (§11).
//!
//! **Capstone:** Launch the interstellar expedition (a system-scale megaproject).
//! **Alternates:** Economic, population, or scientific milestones for different play styles.
//! **Sandbox continue:** After any victory the player may keep playing (Factorio-style).
//!
//! Victory tracking is read-only from the perspective of external callers; the engine
//! checks conditions each strategic month and emits [`VictoryAchieved`] events.

use serde::{Deserialize, Serialize};

// ─── VictoryCondition ────────────────────────────────────────────────────────

/// One victory condition that can be satisfied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VictoryCondition {
    /// Primary capstone: launch the interstellar expedition megaproject.
    InterstellarExpeditionLaunched,
    /// Alternate: reach a system-wide economic output milestone.
    EconomicMilestone {
        /// Total commodity units per turn target.
        target_output: u64,
    },
    /// Alternate: reach a system-wide population milestone.
    PopulationMilestone {
        /// Target headcount across all colonies.
        target_population: u64,
    },
    /// Alternate: accumulate a total research points milestone.
    ScienceMilestone {
        /// Cumulative research points target.
        target_research: u64,
    },
    /// Alternate: research every non-gated tech node in the tech tree.
    TechTreeComplete,
    /// Alternate: control ≥ `share_pct` percent of traded volume for a commodity.
    TradeDominance {
        /// Commodity identifier to measure dominance for.
        commodity_id: String,
        /// Required share of total traded volume (0.0–100.0).
        share_pct: f32,
    },
}

impl VictoryCondition {
    /// Short human-readable description of this condition.
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            VictoryCondition::InterstellarExpeditionLaunched => {
                "Launch the interstellar expedition".to_string()
            }
            VictoryCondition::EconomicMilestone { target_output } => {
                format!("Reach {target_output} total commodity output per turn")
            }
            VictoryCondition::PopulationMilestone { target_population } => {
                format!("Reach {target_population} colonists across all colonies")
            }
            VictoryCondition::ScienceMilestone { target_research } => {
                format!("Accumulate {target_research} total research points")
            }
            VictoryCondition::TechTreeComplete => {
                "Complete the entire tech tree".to_string()
            }
            VictoryCondition::TradeDominance { commodity_id, share_pct } => {
                format!("Control {share_pct:.0}% of traded {commodity_id} volume")
            }
        }
    }
}

// ─── VictoryProgress ─────────────────────────────────────────────────────────

/// Snapshot of progress toward a single victory condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VictoryProgress {
    /// The condition being tracked.
    pub condition: VictoryCondition,
    /// Current value (units depend on condition type).
    pub current: u64,
    /// Required value (units depend on condition type).
    pub required: u64,
    /// Whether this condition has already been satisfied.
    pub achieved: bool,
}

impl VictoryProgress {
    /// Fraction complete in `[0.0, 1.0]`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn fraction(&self) -> f32 {
        if self.required == 0 {
            return 1.0;
        }
        (self.current as f32 / self.required as f32).clamp(0.0, 1.0)
    }
}

// ─── VictoryState ────────────────────────────────────────────────────────────

/// Tracks all active victory conditions and the sandbox-continue mode flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VictoryState {
    /// All conditions being tracked this campaign.
    pub conditions: Vec<VictoryProgress>,
    /// Whether the player has achieved at least one victory condition.
    pub any_achieved: bool,
    /// Whether the player chose to continue playing after victory (sandbox continue).
    pub sandbox_continue: bool,
}

impl VictoryState {
    /// Construct a new state tracking the given conditions.
    #[must_use]
    pub fn new(conditions: Vec<VictoryCondition>) -> Self {
        let tracked = conditions
            .into_iter()
            .map(|c| {
                let required = required_for(&c);
                VictoryProgress {
                    condition: c,
                    current: 0,
                    required,
                    achieved: false,
                }
            })
            .collect();
        Self {
            conditions: tracked,
            any_achieved: false,
            sandbox_continue: false,
        }
    }

    /// Default state tracking only the capstone expedition condition.
    #[must_use]
    pub fn capstone_only() -> Self {
        Self::new(vec![VictoryCondition::InterstellarExpeditionLaunched])
    }

    /// Update progress and return the set of conditions newly achieved this tick.
    ///
    /// `snapshot` provides current game metrics for evaluation.
    pub fn evaluate(&mut self, snapshot: &VictorySnapshot) -> Vec<VictoryCondition> {
        let mut newly_achieved = Vec::new();

        for progress in &mut self.conditions {
            if progress.achieved {
                continue;
            }
            let (current, satisfied) = evaluate_condition(&progress.condition, snapshot);
            progress.current = current;
            if satisfied {
                progress.achieved = true;
                self.any_achieved = true;
                newly_achieved.push(progress.condition.clone());
            }
        }

        newly_achieved
    }

    /// Activate sandbox-continue mode (called when the player chooses to keep playing).
    pub fn activate_sandbox_continue(&mut self) {
        self.sandbox_continue = true;
    }

    /// Whether the game should show the victory screen (won but not yet continued).
    #[must_use]
    pub fn victory_screen_pending(&self) -> bool {
        self.any_achieved && !self.sandbox_continue
    }
}

/// Snapshot of current game metrics used to evaluate victory conditions.
#[derive(Debug, Clone, Default)]
pub struct VictorySnapshot {
    /// Whether the interstellar expedition megaproject has been launched.
    pub expedition_launched: bool,
    /// Total commodity output across all colonies this turn.
    pub total_output: u64,
    /// Total population across all colonies.
    pub total_population: u64,
    /// Cumulative research points earned this campaign.
    pub cumulative_research: u64,
    /// Whether all non-gated tech nodes have been researched.
    pub tech_tree_complete: bool,
    /// Total traded volume for each commodity across all routes this turn (commodity id → units).
    pub total_traded_volume: std::collections::HashMap<String, f64>,
    /// Volume traded by player-controlled colonies for each commodity (commodity id → units).
    pub player_traded_volume: std::collections::HashMap<String, f64>,
}

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Default required value for a condition (used during construction).
fn required_for(condition: &VictoryCondition) -> u64 {
    match condition {
        // Binary conditions — 0 = not done, 1 = done.
        VictoryCondition::InterstellarExpeditionLaunched | VictoryCondition::TechTreeComplete => 1,
        VictoryCondition::EconomicMilestone { target_output } => *target_output,
        VictoryCondition::PopulationMilestone { target_population } => *target_population,
        VictoryCondition::ScienceMilestone { target_research } => *target_research,
        // Percentage stored as integer 0–100.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        VictoryCondition::TradeDominance { share_pct, .. } => *share_pct as u64,
    }
}

/// Evaluate one condition against a snapshot. Returns `(current, satisfied)`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
fn evaluate_condition(condition: &VictoryCondition, snapshot: &VictorySnapshot) -> (u64, bool) {
    match condition {
        VictoryCondition::InterstellarExpeditionLaunched => {
            let v = u64::from(snapshot.expedition_launched);
            (v, snapshot.expedition_launched)
        }
        VictoryCondition::EconomicMilestone { target_output } => (
            snapshot.total_output,
            snapshot.total_output >= *target_output,
        ),
        VictoryCondition::PopulationMilestone { target_population } => (
            snapshot.total_population,
            snapshot.total_population >= *target_population,
        ),
        VictoryCondition::ScienceMilestone { target_research } => (
            snapshot.cumulative_research,
            snapshot.cumulative_research >= *target_research,
        ),
        VictoryCondition::TechTreeComplete => {
            let v = u64::from(snapshot.tech_tree_complete);
            (v, snapshot.tech_tree_complete)
        }
        VictoryCondition::TradeDominance { commodity_id, share_pct } => {
            let total = snapshot.total_traded_volume.get(commodity_id.as_str()).copied().unwrap_or(0.0);
            let player = snapshot.player_traded_volume.get(commodity_id.as_str()).copied().unwrap_or(0.0);
            let actual_pct = if total > 0.0 { (player / total * 100.0) as u64 } else { 0 };
            let required_pct = *share_pct as u64;
            (actual_pct, actual_pct >= required_pct)
        }
    }
}

// ─── YAML loading ─────────────────────────────────────────────────────────────

/// Raw YAML schema for a single victory condition entry.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum VictoryConditionYaml {
    InterstellarExpeditionLaunched,
    EconomicMilestone { target_output: u64 },
    PopulationMilestone { target_population: u64 },
    ScienceMilestone { target_research: u64 },
    TechTreeComplete,
    TradeDominance { commodity_id: String, share_pct: f32 },
}

impl From<VictoryConditionYaml> for VictoryCondition {
    fn from(y: VictoryConditionYaml) -> Self {
        match y {
            VictoryConditionYaml::InterstellarExpeditionLaunched => Self::InterstellarExpeditionLaunched,
            VictoryConditionYaml::EconomicMilestone { target_output } => Self::EconomicMilestone { target_output },
            VictoryConditionYaml::PopulationMilestone { target_population } => Self::PopulationMilestone { target_population },
            VictoryConditionYaml::ScienceMilestone { target_research } => Self::ScienceMilestone { target_research },
            VictoryConditionYaml::TechTreeComplete => Self::TechTreeComplete,
            VictoryConditionYaml::TradeDominance { commodity_id, share_pct } => Self::TradeDominance { commodity_id, share_pct },
        }
    }
}

/// Load a list of [`VictoryCondition`]s from YAML text.
///
/// The YAML must contain a top-level `conditions:` sequence of tagged objects.
///
/// # Errors
///
/// Returns a [`serde_yaml::Error`] if the YAML is malformed or has unknown condition types.
pub fn load_victory_conditions(yaml: &str) -> Result<Vec<VictoryCondition>, serde_yaml::Error> {
    #[derive(Deserialize)]
    struct Root {
        conditions: Vec<VictoryConditionYaml>,
    }
    let root: Root = serde_yaml::from_str(yaml)?;
    Ok(root.conditions.into_iter().map(Into::into).collect())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capstone_not_achieved_before_expedition() {
        let mut vs = VictoryState::capstone_only();
        let snap = VictorySnapshot {
            expedition_launched: false,
            ..Default::default()
        };
        let achieved = vs.evaluate(&snap);
        assert!(achieved.is_empty());
        assert!(!vs.any_achieved);
    }

    #[test]
    fn capstone_achieved_when_expedition_launched() {
        let mut vs = VictoryState::capstone_only();
        let snap = VictorySnapshot {
            expedition_launched: true,
            ..Default::default()
        };
        let achieved = vs.evaluate(&snap);
        assert_eq!(achieved.len(), 1);
        assert_eq!(
            achieved[0],
            VictoryCondition::InterstellarExpeditionLaunched
        );
        assert!(vs.any_achieved);
    }

    #[test]
    fn sandbox_continue_suppresses_victory_screen() {
        let mut vs = VictoryState::capstone_only();
        let snap = VictorySnapshot {
            expedition_launched: true,
            ..Default::default()
        };
        vs.evaluate(&snap);
        assert!(vs.victory_screen_pending());
        vs.activate_sandbox_continue();
        assert!(!vs.victory_screen_pending());
        assert!(vs.sandbox_continue);
    }

    #[test]
    fn economic_milestone_tracks_progress() {
        let mut vs = VictoryState::new(vec![VictoryCondition::EconomicMilestone {
            target_output: 1000,
        }]);
        let snap_low = VictorySnapshot {
            total_output: 500,
            ..Default::default()
        };
        vs.evaluate(&snap_low);
        assert!(!vs.any_achieved);
        assert!((vs.conditions[0].fraction() - 0.5).abs() < 1e-3);

        let snap_high = VictorySnapshot {
            total_output: 1000,
            ..Default::default()
        };
        let achieved = vs.evaluate(&snap_high);
        assert_eq!(achieved.len(), 1);
    }

    #[test]
    fn population_milestone_achieved() {
        let mut vs = VictoryState::new(vec![VictoryCondition::PopulationMilestone {
            target_population: 5000,
        }]);
        let snap = VictorySnapshot {
            total_population: 5001,
            ..Default::default()
        };
        let achieved = vs.evaluate(&snap);
        assert!(!achieved.is_empty());
    }

    #[test]
    fn science_milestone_achieved() {
        let mut vs = VictoryState::new(vec![VictoryCondition::ScienceMilestone {
            target_research: 10_000,
        }]);
        let snap = VictorySnapshot {
            cumulative_research: 10_000,
            ..Default::default()
        };
        let achieved = vs.evaluate(&snap);
        assert!(!achieved.is_empty());
    }

    #[test]
    fn already_achieved_condition_not_re_emitted() {
        let mut vs = VictoryState::capstone_only();
        let snap = VictorySnapshot {
            expedition_launched: true,
            ..Default::default()
        };
        vs.evaluate(&snap);
        let second = vs.evaluate(&snap);
        assert!(
            second.is_empty(),
            "should not re-emit already achieved condition"
        );
    }

    #[test]
    fn multiple_conditions_each_tracked_independently() {
        let mut vs = VictoryState::new(vec![
            VictoryCondition::InterstellarExpeditionLaunched,
            VictoryCondition::PopulationMilestone {
                target_population: 1000,
            },
        ]);
        // Only expedition
        let snap1 = VictorySnapshot {
            expedition_launched: true,
            total_population: 500,
            ..Default::default()
        };
        let a1 = vs.evaluate(&snap1);
        assert_eq!(a1.len(), 1);
        assert_eq!(a1[0], VictoryCondition::InterstellarExpeditionLaunched);

        // Now population too
        let snap2 = VictorySnapshot {
            expedition_launched: true,
            total_population: 1000,
            ..Default::default()
        };
        let a2 = vs.evaluate(&snap2);
        assert_eq!(a2.len(), 1);
        assert_eq!(
            a2[0],
            VictoryCondition::PopulationMilestone {
                target_population: 1000
            }
        );
    }

    #[test]
    fn fraction_clamps_at_one() {
        let p = VictoryProgress {
            condition: VictoryCondition::EconomicMilestone { target_output: 100 },
            current: 200,
            required: 100,
            achieved: false,
        };
        assert!((p.fraction() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn condition_descriptions_non_empty() {
        let conditions = vec![
            VictoryCondition::InterstellarExpeditionLaunched,
            VictoryCondition::EconomicMilestone { target_output: 1 },
            VictoryCondition::PopulationMilestone { target_population: 1 },
            VictoryCondition::ScienceMilestone { target_research: 1 },
            VictoryCondition::TechTreeComplete,
            VictoryCondition::TradeDominance {
                commodity_id: "food".into(),
                share_pct: 51.0,
            },
        ];
        for c in conditions {
            assert!(!c.description().is_empty());
        }
    }

    #[test]
    fn tech_tree_complete_not_achieved_when_incomplete() {
        let mut vs = VictoryState::new(vec![VictoryCondition::TechTreeComplete]);
        let snap = VictorySnapshot {
            tech_tree_complete: false,
            ..Default::default()
        };
        let achieved = vs.evaluate(&snap);
        assert!(achieved.is_empty());
        assert!(!vs.any_achieved);
    }

    #[test]
    fn tech_tree_complete_achieved_when_all_researched() {
        let mut vs = VictoryState::new(vec![VictoryCondition::TechTreeComplete]);
        let snap = VictorySnapshot {
            tech_tree_complete: true,
            ..Default::default()
        };
        let achieved = vs.evaluate(&snap);
        assert_eq!(achieved.len(), 1);
        assert_eq!(achieved[0], VictoryCondition::TechTreeComplete);
        assert!(vs.any_achieved);
    }

    #[test]
    fn trade_dominance_not_achieved_below_threshold() {
        let mut vs = VictoryState::new(vec![VictoryCondition::TradeDominance {
            commodity_id: "iron".into(),
            share_pct: 60.0,
        }]);
        let mut total = std::collections::HashMap::new();
        total.insert("iron".to_string(), 100.0);
        let mut player = std::collections::HashMap::new();
        player.insert("iron".to_string(), 50.0); // 50%, below 60%
        let snap = VictorySnapshot {
            total_traded_volume: total,
            player_traded_volume: player,
            ..Default::default()
        };
        let achieved = vs.evaluate(&snap);
        assert!(achieved.is_empty());
        assert!(!vs.any_achieved);
    }

    #[test]
    fn trade_dominance_achieved_at_threshold() {
        let mut vs = VictoryState::new(vec![VictoryCondition::TradeDominance {
            commodity_id: "iron".into(),
            share_pct: 60.0,
        }]);
        let mut total = std::collections::HashMap::new();
        total.insert("iron".to_string(), 100.0);
        let mut player = std::collections::HashMap::new();
        player.insert("iron".to_string(), 60.0); // exactly 60%
        let snap = VictorySnapshot {
            total_traded_volume: total,
            player_traded_volume: player,
            ..Default::default()
        };
        let achieved = vs.evaluate(&snap);
        assert_eq!(achieved.len(), 1);
        assert!(matches!(
            achieved[0],
            VictoryCondition::TradeDominance { .. }
        ));
    }

    #[test]
    fn trade_dominance_zero_total_volume_not_achieved() {
        let mut vs = VictoryState::new(vec![VictoryCondition::TradeDominance {
            commodity_id: "food".into(),
            share_pct: 51.0,
        }]);
        let snap = VictorySnapshot::default();
        let achieved = vs.evaluate(&snap);
        assert!(achieved.is_empty());
    }

    #[test]
    fn load_victory_conditions_from_yaml() {
        let yaml = r#"
conditions:
  - type: interstellar_expedition_launched
  - type: population_milestone
    target_population: 10000
  - type: tech_tree_complete
  - type: trade_dominance
    commodity_id: food
    share_pct: 51.0
"#;
        let conditions = super::load_victory_conditions(yaml).unwrap();
        assert_eq!(conditions.len(), 4);
        assert!(matches!(
            conditions[0],
            VictoryCondition::InterstellarExpeditionLaunched
        ));
        assert!(matches!(
            conditions[1],
            VictoryCondition::PopulationMilestone { target_population: 10000 }
        ));
        assert!(matches!(conditions[2], VictoryCondition::TechTreeComplete));
        assert!(matches!(
            conditions[3],
            VictoryCondition::TradeDominance { .. }
        ));
    }
}
