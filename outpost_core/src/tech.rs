//! Tech DAG — authored technology definitions, state, and unlock application.
//!
//! Tech is **unlocks-first**: most techs are binary gates making a new
//! building, commodity type, or capability *available*.  The DAG shape emerges
//! from authored data (`{ id, prerequisites[], cost, effects[] }`).
//!
//! See `docs/DESIGN.md §7A`.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::research::SystemResearchPool;

// ─── Content types (pack-level authoring schema) ─────────────────────────────

/// Identifier for a tech node.
pub type TechId = String;

/// Identifier for a capability gate (string slug).
pub type CapabilityId = String;

/// One effect granted by completing a technology.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TechEffect {
    /// Makes a building type constructable.
    UnlockBuilding {
        /// Building definition id.
        building_id: String,
    },
    /// Makes a commodity type produceable / tradeable.
    UnlockCommodity {
        /// Commodity definition id.
        commodity_id: String,
    },
    /// Enables a named capability gate.
    UnlockCapability {
        /// Capability slug.
        capability_id: CapabilityId,
    },
    /// Applies an additive numeric bonus within a named category.
    Bonus {
        /// The quantity category being boosted (e.g. `"production_efficiency"`).
        category: String,
        /// Additive modifier value (e.g. `0.20` for +20 %).
        value: f32,
    },
    /// Cancels one [`crate::system::HabitabilityAttribute`]'s penalty across
    /// every colony linked to a body (issue #185).
    ///
    /// Applied retroactively: colonies founded before this tech completed are
    /// recomputed immediately, matching how [`TechEffect::UnlockCapability`]
    /// already applies globally rather than only to future colonies.
    MitigateAttribute {
        /// The habitability input this tech treats as its best band.
        attribute: crate::system::HabitabilityAttribute,
    },
    /// Improves survey-expedition reveal odds system-wide (issue #236,
    /// feeding #235's `expedition::resolve_survey`). Stacks additively
    /// across every researched tech carrying this effect.
    SurveyModifierBonus {
        /// Additive bonus to full-reveal probability.
        #[serde(default)]
        full_reveal_bonus: f32,
        /// Additive bonus to partial-reveal probability.
        #[serde(default)]
        partial_reveal_bonus: f32,
    },
    /// Reduces survey-expedition transit time system-wide (issue #236,
    /// feeding #235). `fraction` is the proportional reduction (e.g. `0.15`
    /// = 15% shorter transit); multiple researched techs stack
    /// multiplicatively on the remaining scalar (`scalar *= 1.0 -
    /// fraction`), floored so transit never reaches zero turns.
    ReduceTransitTime {
        /// Proportional reduction in transit turns, in `[0.0, 1.0)`.
        fraction: f32,
    },
    /// Extends the max range an outpost may be established from its parent
    /// colony's home body (issue #241). Stacks additively across every
    /// researched tech carrying this effect.
    ExtendOutpostRange {
        /// Additive bonus in AU.
        bonus_au: f32,
    },
}

/// Authored technology definition — stored in content pack files.
///
/// The YAML schema also accepts a nested `unlocks: { buildings: [...],
/// resources: [...], bonuses: [...], events: [...] }` block; `buildings` and
/// `resources` are translated into [`TechEffect::UnlockBuilding`] /
/// [`TechEffect::UnlockCommodity`] entries when the registry loads. Bonus
/// and event unlocks are accepted but not yet mapped to core effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechDef {
    /// Unique identifier across all packs.
    pub id: TechId,
    /// Human-readable name. Also accepts the YAML field `name`.
    #[serde(alias = "name")]
    pub display_name: String,
    /// Optional short description shown in the tech tree UI.
    #[serde(default)]
    pub description: String,
    /// Optional category tag used for UI grouping (e.g. `"engineering"`).
    #[serde(default)]
    pub category: String,
    /// Optional tier used purely for UI presentation. Defaults to 1.
    ///
    /// The engine derives dependency depth from the prerequisite DAG; `tier`
    /// is a hint for grouping, not a gate.
    #[serde(default = "default_tier")]
    pub tier: u32,
    /// IDs of techs that must be researched before this one is available.
    #[serde(default)]
    pub prerequisites: Vec<TechId>,
    /// Research points required to complete this tech.
    pub research_cost: f32,
    /// Effects applied when this tech is completed.
    #[serde(default)]
    pub effects: Vec<TechEffect>,
    /// Authored unlock spec sugar. Merged into `effects` by
    /// [`load_tech_registry`]; consumers should read `effects`.
    #[serde(default)]
    pub unlocks: TechUnlocks,
}

fn default_tier() -> u32 {
    1
}

impl Default for TechDef {
    fn default() -> Self {
        Self {
            id: String::new(),
            display_name: String::new(),
            description: String::new(),
            category: String::new(),
            tier: default_tier(),
            prerequisites: Vec::new(),
            research_cost: 0.0,
            effects: Vec::new(),
            unlocks: TechUnlocks::default(),
        }
    }
}

/// Authored unlock groupings used by the YAML schema.
///
/// `buildings` and `resources` are converted into concrete [`TechEffect`]
/// entries by [`load_tech_registry`]. `bonuses` (e.g.
/// `"construction_speed_10_percent"`) and `events` are accepted so older
/// authored content still parses cleanly, but are **deliberately, permanently
/// decorative** (issue #248's recorded decision) — they are never converted
/// into [`TechEffect`] entries, so a tech relying only on `bonuses`/`events`
/// sugar (no explicit `effects:` block) produces zero runtime effect. A
/// slug like `"construction_speed_10_percent"` is ambiguous to parse back
/// into a real `{category, value}` pair (the naming convention isn't a
/// stable machine format, just human-readable flavor text), and every
/// numeric bonus that actually needs a real effect already has one
/// available via an explicit `effects: [{type: bonus, category, value}]`
/// entry (see [`TechEffect::Bonus`], wired to production output in
/// `colony/production.rs`, issue #248). Content authors should use
/// `effects:` for anything that must have a real numeric effect, and treat
/// `bonuses`/`events` purely as flavor-text hints; a follow-up content pass
/// (tracked alongside #249) can migrate the handful of pre-#236 techs still
/// relying only on the sugar fields to explicit `effects:` blocks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TechUnlocks {
    /// Building ids this tech makes constructable.
    #[serde(default)]
    pub buildings: Vec<String>,
    /// Commodity ids this tech makes produceable / tradeable.
    #[serde(default)]
    pub resources: Vec<String>,
    /// Named bonus slugs — decorative flavor text only (issue #248); never
    /// converted into a real [`TechEffect`]. Use an explicit `effects:`
    /// entry for anything that must actually change a number.
    #[serde(default)]
    pub bonuses: Vec<String>,
    /// Event ids surfaced by this tech — decorative flavor text only
    /// (issue #248); never converted into a real [`TechEffect`].
    #[serde(default)]
    pub events: Vec<String>,
}

// ─── Runtime state ────────────────────────────────────────────────────────────

/// System-wide tech research state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TechState {
    /// Set of completed tech IDs.
    pub researched: HashSet<TechId>,
    /// The tech currently being researched, if any.
    pub current_project: Option<TechId>,
    /// Research points accumulated toward `current_project`.
    pub progress: f32,
    /// Queued tech IDs (first = next after current project completes).
    pub research_queue: VecDeque<TechId>,
}

impl TechState {
    /// Create a new, empty tech state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the given tech has been researched.
    #[must_use]
    pub fn is_researched(&self, id: &str) -> bool {
        self.researched.contains(id)
    }

    /// Returns `true` if all prerequisites for `def` are satisfied.
    #[must_use]
    pub fn prerequisites_met(&self, def: &TechDef) -> bool {
        def.prerequisites
            .iter()
            .all(|p| self.researched.contains(p))
    }

    /// Start researching `tech_id`, replacing any current project.
    ///
    /// Does **not** validate prerequisites — call [`prerequisites_met`] first.
    pub fn set_current_project(&mut self, tech_id: TechId) {
        self.current_project = Some(tech_id);
        self.progress = 0.0;
    }

    /// Enqueue a tech for research after the current project.
    pub fn enqueue(&mut self, tech_id: TechId) {
        self.research_queue.push_back(tech_id);
    }
}

// ─── Turn drain logic ─────────────────────────────────────────────────────────

/// Result of applying one turn's worth of research spending.
#[derive(Debug, Clone)]
pub struct ResearchTurnResult {
    /// Tech IDs completed this turn (normally 0 or 1).
    pub completed: Vec<TechId>,
    /// Effects that should be applied to game state (one vec per completed tech).
    pub new_effects: Vec<Vec<TechEffect>>,
}

/// Drain research from `pool` into `state`, completing techs when cost is met.
///
/// On completion the next queued tech (if any) becomes `current_project`.
/// Returns which techs (if any) completed and their effects.
pub fn apply_research_turn(
    state: &mut TechState,
    pool: &mut SystemResearchPool,
    registry: &TechRegistry,
) -> ResearchTurnResult {
    apply_research_turn_scaled(state, pool, registry, 1.0)
}

/// Same as [`apply_research_turn`] but multiplies each tech's authored
/// `research_cost` by `cost_scalar` (the resolved `ResearchCost` difficulty
/// scalar, #161) when checking for completion.
pub fn apply_research_turn_scaled(
    state: &mut TechState,
    pool: &mut SystemResearchPool,
    registry: &TechRegistry,
    cost_scalar: f32,
) -> ResearchTurnResult {
    let mut completed = Vec::new();
    let mut new_effects = Vec::new();

    // Drain all available research this turn toward the current project.
    let available = pool.total();
    if available <= 0.0 {
        return ResearchTurnResult {
            completed,
            new_effects,
        };
    }

    let mut remaining = available;
    let cost_mul = cost_scalar.max(0.0);

    loop {
        let project_id = match state.current_project.clone() {
            Some(id) => id,
            None => {
                // Try to advance from queue.
                match state.research_queue.pop_front() {
                    Some(next) => {
                        state.current_project = Some(next.clone());
                        state.progress = 0.0;
                        next
                    }
                    None => break,
                }
            }
        };

        let Some(def) = registry.get(&project_id) else {
            break; // Unknown tech — stop silently.
        };

        let effective_cost = def.research_cost * cost_mul;
        let needed = (effective_cost - state.progress).max(0.0);
        if remaining >= needed {
            // Tech completes.
            remaining -= needed;
            state.progress = 0.0;
            state.current_project = None;
            state.researched.insert(project_id.clone());
            completed.push(project_id);
            new_effects.push(def.effects.clone());

            // Advance to next in queue if any research remains.
            if remaining <= 0.0 {
                break;
            }
        } else {
            state.progress += remaining;
            remaining = 0.0;
            break;
        }
    }

    // Spend from pool.
    let spent = available - remaining;
    if spent > 0.0 {
        // Pool only has `deposit`; we simulate spend by reconstructing.
        pool.total -= spent;
    }

    ResearchTurnResult {
        completed,
        new_effects,
    }
}

// ─── Tech registry (validated in-memory index) ───────────────────────────────

/// In-memory registry of tech definitions, built from content pack files.
#[derive(Debug, Clone, Default)]
pub struct TechRegistry {
    techs: HashMap<TechId, TechDef>,
}

/// Error produced during tech registry construction or pack validation.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TechRegistryError {
    /// A tech references a prerequisite that does not exist in the pack.
    #[error("tech '{id}' references unknown prerequisite '{prereq}'")]
    UnknownPrerequisite {
        /// Tech that has the bad reference.
        id: TechId,
        /// The missing prerequisite id.
        prereq: TechId,
    },
    /// The prerequisites form a cycle; the cycle is described in the message.
    #[error("tech DAG contains a cycle involving: {cycle}")]
    CycleDetected {
        /// A human-readable description of the cycle members.
        cycle: String,
    },
}

impl TechRegistry {
    /// Build and validate a [`TechRegistry`] from a list of [`TechDef`]s.
    ///
    /// Validates that all prerequisite IDs exist and that the DAG is acyclic.
    ///
    /// # Errors
    ///
    /// Returns [`TechRegistryError::UnknownPrerequisite`] if a referenced
    /// prerequisite is not found, or [`TechRegistryError::CycleDetected`] if
    /// the prerequisite graph contains a cycle.
    ///
    /// # Panics
    ///
    /// Panics if internal state is inconsistent (should not occur if the
    /// prerequisite validation above passes).
    pub fn build(defs: Vec<TechDef>) -> Result<Self, TechRegistryError> {
        let techs: HashMap<TechId, TechDef> = defs.into_iter().map(|d| (d.id.clone(), d)).collect();

        // Validate all prerequisites exist.
        for def in techs.values() {
            for prereq in &def.prerequisites {
                if !techs.contains_key(prereq) {
                    return Err(TechRegistryError::UnknownPrerequisite {
                        id: def.id.clone(),
                        prereq: prereq.clone(),
                    });
                }
            }
        }

        // Cycle detection via Kahn's algorithm (topological sort).
        let mut in_degree: HashMap<&str, usize> = techs.keys().map(|id| (id.as_str(), 0)).collect();

        for def in techs.values() {
            for _prereq in &def.prerequisites {
                // Each tech is a dependent of its prerequisites, so increment
                // the in-degree of the dependent (this tech).
                *in_degree.entry(def.id.as_str()).or_insert(0) += 1;
            }
        }

        // Actually we need: for each edge (prereq → tech), in_degree[tech]++.
        // Re-compute correctly.
        let mut in_degree: HashMap<&str, usize> =
            techs.keys().map(|id| (id.as_str(), 0usize)).collect();
        for def in techs.values() {
            for _prereq in &def.prerequisites {
                *in_degree.get_mut(def.id.as_str()).unwrap() += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut visited = 0usize;
        while let Some(node) = queue.pop_front() {
            visited += 1;
            // Find all techs that list `node` as a prerequisite.
            for def in techs.values() {
                if def.prerequisites.iter().any(|p| p == node) {
                    let deg = in_degree.get_mut(def.id.as_str()).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(def.id.as_str());
                    }
                }
            }
        }

        if visited != techs.len() {
            // Some nodes were never drained → cycle.
            let cycle_members: Vec<_> = in_degree
                .iter()
                .filter(|(_, &d)| d > 0)
                .map(|(id, _)| *id)
                .collect();
            return Err(TechRegistryError::CycleDetected {
                cycle: cycle_members.join(", "),
            });
        }

        Ok(Self { techs })
    }

    /// Look up a tech definition by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&TechDef> {
        self.techs.get(id)
    }

    /// Iterate over all tech definitions.
    pub fn all(&self) -> impl Iterator<Item = &TechDef> {
        self.techs.values()
    }

    /// Returns the set of tech IDs whose prerequisites are all in `researched`.
    #[must_use]
    pub fn available_techs<'a>(&'a self, researched: &'a HashSet<TechId>) -> Vec<&'a TechDef> {
        self.techs
            .values()
            .filter(|def| {
                !researched.contains(&def.id)
                    && def.prerequisites.iter().all(|p| researched.contains(p))
            })
            .collect()
    }
}

// ─── Colony unlock helpers ────────────────────────────────────────────────────

/// Returns the building IDs unlocked by a set of researched techs.
///
/// A building is available if it has no `tech_prerequisite` OR its prerequisite
/// is in `researched`.
#[must_use]
pub fn unlocked_buildings<'a, S: ::std::hash::BuildHasher>(
    all_buildings: impl Iterator<Item = &'a crate::content::types::BuildingDef>,
    researched: &HashSet<TechId, S>,
) -> Vec<&'a crate::content::types::BuildingDef> {
    all_buildings
        .filter(|b| match &b.tech_prerequisite {
            None => true,
            Some(req) => researched.contains(req),
        })
        .collect()
}

// ─── Pack loading ────────────────────────────────────────────────────────────

/// Load and validate a [`TechRegistry`] from raw YAML bytes.
///
/// The YAML should be a sequence of [`TechDef`] records.
/// Validates all prerequisite references and detects cycles.
///
/// # Errors
///
/// Returns a [`TechRegistryError`] if the YAML is malformed, prerequisites
/// are unknown, or the DAG contains a cycle.
pub fn load_tech_registry(yaml: &str) -> Result<TechRegistry, TechLoadError> {
    let mut defs: Vec<TechDef> =
        serde_yaml::from_str(yaml).map_err(|e| TechLoadError::Parse(e.to_string()))?;

    // Translate the YAML `unlocks` sugar into concrete `TechEffect` entries.
    // `bonuses` and `events` are accepted by the parser but not yet mapped;
    // the fields remain on `unlocks` so downstream tooling can surface them.
    for def in &mut defs {
        for building_id in def.unlocks.buildings.drain(..) {
            def.effects.push(TechEffect::UnlockBuilding { building_id });
        }
        for commodity_id in def.unlocks.resources.drain(..) {
            def.effects
                .push(TechEffect::UnlockCommodity { commodity_id });
        }
    }

    TechRegistry::build(defs).map_err(TechLoadError::Registry)
}

/// Error type returned by [`load_tech_registry`].
#[derive(Debug, thiserror::Error)]
pub enum TechLoadError {
    /// YAML parse error.
    #[error("tech pack parse error: {0}")]
    Parse(String),
    /// DAG validation error.
    #[error("{0}")]
    Registry(#[from] TechRegistryError),
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_tech(id: &str, prereqs: Vec<&str>, cost: f32) -> TechDef {
        TechDef {
            id: id.to_string(),
            display_name: id.to_string(),
            prerequisites: prereqs.into_iter().map(str::to_string).collect(),
            research_cost: cost,
            effects: vec![TechEffect::UnlockBuilding {
                building_id: format!("{id}_building"),
            }],
            ..TechDef::default()
        }
    }

    fn make_pool(amount: f32) -> SystemResearchPool {
        let mut p = SystemResearchPool::new();
        p.deposit(amount);
        p
    }

    // ── DAG validation ────────────────────────────────────────────────────

    #[test]
    fn build_empty_registry() {
        let reg = TechRegistry::build(vec![]).unwrap();
        assert_eq!(reg.all().count(), 0);
    }

    #[test]
    fn build_linear_chain() {
        let defs = vec![
            simple_tech("alpha", vec![], 10.0),
            simple_tech("beta", vec!["alpha"], 20.0),
            simple_tech("gamma", vec!["beta"], 30.0),
        ];
        assert!(TechRegistry::build(defs).is_ok());
    }

    #[test]
    fn unknown_prerequisite_rejected() {
        let defs = vec![simple_tech("beta", vec!["nonexistent"], 10.0)];
        let err = TechRegistry::build(defs).unwrap_err();
        assert!(matches!(err, TechRegistryError::UnknownPrerequisite { .. }));
    }

    #[test]
    fn cycle_detected_simple() {
        let defs = vec![
            simple_tech("a", vec!["b"], 10.0),
            simple_tech("b", vec!["a"], 10.0),
        ];
        let err = TechRegistry::build(defs).unwrap_err();
        assert!(matches!(err, TechRegistryError::CycleDetected { .. }));
    }

    #[test]
    fn cycle_detected_three_node() {
        let defs = vec![
            simple_tech("a", vec!["c"], 10.0),
            simple_tech("b", vec!["a"], 10.0),
            simple_tech("c", vec!["b"], 10.0),
        ];
        let err = TechRegistry::build(defs).unwrap_err();
        assert!(matches!(err, TechRegistryError::CycleDetected { .. }));
    }

    #[test]
    fn self_referential_cycle_detected() {
        let defs = vec![simple_tech("a", vec!["a"], 10.0)];
        let err = TechRegistry::build(defs).unwrap_err();
        assert!(matches!(err, TechRegistryError::CycleDetected { .. }));
    }

    // ── Prerequisite enforcement ──────────────────────────────────────────

    #[test]
    fn prerequisites_met_empty() {
        let def = simple_tech("alpha", vec![], 10.0);
        let state = TechState::new();
        assert!(state.prerequisites_met(&def));
    }

    #[test]
    fn prerequisites_not_met_when_prereq_missing() {
        let def = simple_tech("beta", vec!["alpha"], 10.0);
        let state = TechState::new();
        assert!(!state.prerequisites_met(&def));
    }

    #[test]
    fn prerequisites_met_when_prereq_researched() {
        let def = simple_tech("beta", vec!["alpha"], 10.0);
        let mut state = TechState::new();
        state.researched.insert("alpha".to_string());
        assert!(state.prerequisites_met(&def));
    }

    // ── Research turn drain ───────────────────────────────────────────────

    #[test]
    fn research_completes_tech_on_sufficient_pool() {
        let defs = vec![simple_tech("alpha", vec![], 10.0)];
        let reg = TechRegistry::build(defs).unwrap();
        let mut state = TechState::new();
        state.set_current_project("alpha".to_string());
        let mut pool = make_pool(15.0);

        let result = apply_research_turn(&mut state, &mut pool, &reg);

        assert_eq!(result.completed, vec!["alpha"]);
        assert!(state.is_researched("alpha"));
        // Surplus 5.0 remains.
        assert!((pool.total() - 5.0).abs() < 1e-4);
    }

    #[test]
    fn research_partial_progress_no_completion() {
        let defs = vec![simple_tech("alpha", vec![], 20.0)];
        let reg = TechRegistry::build(defs).unwrap();
        let mut state = TechState::new();
        state.set_current_project("alpha".to_string());
        let mut pool = make_pool(5.0);

        let result = apply_research_turn(&mut state, &mut pool, &reg);

        assert!(result.completed.is_empty());
        assert!(!state.is_researched("alpha"));
        assert!((state.progress - 5.0).abs() < 1e-4);
        assert!((pool.total()).abs() < 1e-4);
    }

    #[test]
    fn research_chains_to_next_queued_tech() {
        let defs = vec![
            simple_tech("alpha", vec![], 10.0),
            simple_tech("beta", vec!["alpha"], 10.0),
        ];
        let reg = TechRegistry::build(defs).unwrap();
        let mut state = TechState::new();
        state.researched.insert("alpha".to_string()); // prereq already done
        state.set_current_project("beta".to_string());
        let mut pool = make_pool(10.0);

        let result = apply_research_turn(&mut state, &mut pool, &reg);

        assert_eq!(result.completed, vec!["beta"]);
        assert!(state.is_researched("beta"));
    }

    #[test]
    fn load_tech_registry_parses_placeholder_yaml_schema() {
        // Sanity check the alias + unlocks-translation path used by the base
        // pack. Fields the loader has to tolerate: `name` (alias for
        // display_name), `description`, `category`, `tier`, `research_time_ticks`
        // (unknown), `resource_costs` (unknown), `unlocks` sugar.
        let yaml = "\
- id: basic_construction
  name: Basic Construction
  description: Foundational construction techniques.
  category: engineering
  tier: 1
  research_cost: 100.0
  research_time_ticks: 50
  prerequisites: []
  unlocks:
    buildings:
      - basic_habitat
      - warehouse
    resources:
      - concrete
    bonuses:
      - construction_speed_10_percent
";
        let reg = load_tech_registry(yaml).expect("yaml must parse");
        let def = reg.get("basic_construction").unwrap();
        assert_eq!(def.display_name, "Basic Construction");
        assert_eq!(def.description, "Foundational construction techniques.");
        assert_eq!(def.category, "engineering");
        assert_eq!(def.tier, 1);
        // buildings + resources unlocks should have been rewritten as effects.
        let has_building = def
            .effects
            .iter()
            .any(|e| matches!(e, TechEffect::UnlockBuilding { building_id } if building_id == "basic_habitat"));
        let has_commodity = def
            .effects
            .iter()
            .any(|e| matches!(e, TechEffect::UnlockCommodity { commodity_id } if commodity_id == "concrete"));
        assert!(has_building, "expected UnlockBuilding for basic_habitat");
        assert!(has_commodity, "expected UnlockCommodity for concrete");
        // buildings should have been drained into effects.
        assert!(def.unlocks.buildings.is_empty());
        // bonuses are accepted but preserved on unlocks (not yet mapped).
        assert_eq!(def.unlocks.bonuses.len(), 1);
    }

    #[test]
    fn unlock_effects_returned_on_completion() {
        let defs = vec![TechDef {
            id: "alpha".to_string(),
            display_name: "Alpha".to_string(),
            prerequisites: vec![],
            research_cost: 5.0,
            effects: vec![
                TechEffect::UnlockBuilding {
                    building_id: "lab".to_string(),
                },
                TechEffect::UnlockCapability {
                    capability_id: "warp".to_string(),
                },
            ],
            ..TechDef::default()
        }];
        let reg = TechRegistry::build(defs).unwrap();
        let mut state = TechState::new();
        state.set_current_project("alpha".to_string());
        let mut pool = make_pool(10.0);

        let result = apply_research_turn(&mut state, &mut pool, &reg);

        assert_eq!(result.new_effects.len(), 1);
        assert_eq!(result.new_effects[0].len(), 2);
    }

    // ── available_techs helper ────────────────────────────────────────────

    #[test]
    fn available_techs_excludes_researched() {
        let defs = vec![
            simple_tech("alpha", vec![], 10.0),
            simple_tech("beta", vec!["alpha"], 10.0),
        ];
        let reg = TechRegistry::build(defs).unwrap();
        let mut researched = HashSet::new();
        researched.insert("alpha".to_string());

        let avail: Vec<_> = reg.available_techs(&researched);
        assert_eq!(avail.len(), 1);
        assert_eq!(avail[0].id, "beta");
    }

    #[test]
    fn available_techs_requires_prerequisite() {
        let defs = vec![
            simple_tech("alpha", vec![], 10.0),
            simple_tech("beta", vec!["alpha"], 10.0),
        ];
        let reg = TechRegistry::build(defs).unwrap();
        let researched = HashSet::new();

        let avail: Vec<_> = reg.available_techs(&researched);
        // Only alpha is available (beta requires alpha).
        assert_eq!(avail.len(), 1);
        assert_eq!(avail[0].id, "alpha");
    }

    // ── unlocked_buildings helper ─────────────────────────────────────────

    #[test]
    fn unlocked_buildings_no_prereq_always_available() {
        use crate::content::types::{BuildingCategory, BuildingDef};
        let buildings = vec![BuildingDef {
            id: "shelter".to_string(),
            name: "Shelter".to_string(),
            description: String::new(),
            category: BuildingCategory::Housing,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        }];
        let researched = HashSet::new();
        let result = unlocked_buildings(buildings.iter(), &researched);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn unlocked_buildings_gated_by_tech() {
        use crate::content::types::{BuildingCategory, BuildingDef};
        let buildings = vec![BuildingDef {
            id: "adv_lab".to_string(),
            name: "Advanced Lab".to_string(),
            description: String::new(),
            category: BuildingCategory::Research,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 2,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 3,
            tech_prerequisite: Some("science_tier2".to_string()),
            maintenance: vec![],
        }];

        let empty: HashSet<TechId> = HashSet::new();
        assert!(unlocked_buildings(buildings.iter(), &empty).is_empty());

        let mut with_tech = HashSet::new();
        with_tech.insert("science_tier2".to_string());
        assert_eq!(unlocked_buildings(buildings.iter(), &with_tech).len(), 1);
    }

    // ── Real content/base/tech.yaml (issue #236) ──────────────────────────────

    fn read_real_tech_yaml() -> Option<String> {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
        let root = std::path::Path::new(&manifest).parent()?.to_path_buf();
        let path = root.join("content").join("base").join("tech.yaml");
        std::fs::read_to_string(path).ok()
    }

    /// Done-when: the real authored tree parses cleanly (no cycles, no
    /// unknown-prerequisite refs) and is a genuine multi-category,
    /// multi-tier DAG well beyond the pre-#236 12-entry placeholder set.
    ///
    /// Thresholds raised in #249 (wave 2, ~48 → ~81 entries, tier 6 → 7) to
    /// reflect the larger tree's actual shape rather than staying pinned to
    /// #236's original "solid start" numbers.
    #[test]
    fn real_tech_yaml_parses_and_forms_a_wide_multi_tier_dag() {
        let Some(yaml) = read_real_tech_yaml() else {
            return; // content/ not present in this checkout layout; skip.
        };
        let registry = load_tech_registry(&yaml).expect("content/base/tech.yaml must parse");

        let all_ids: Vec<&TechId> = registry.techs.keys().collect();
        assert!(
            all_ids.len() >= 75,
            "expected a wave-2 expansion well beyond #236's ~48-entry solid \
             start, got {} techs",
            all_ids.len()
        );

        let categories: std::collections::HashSet<&str> = registry
            .techs
            .values()
            .map(|d| d.category.as_str())
            .collect();
        for expected in [
            "engineering",
            "physical_sciences",
            "life_sciences",
            "computing",
            "materials_science",
            "astronautics",
        ] {
            assert!(
                categories.contains(expected),
                "expected category '{expected}' to be represented in the tree"
            );
        }

        let max_tier = registry.techs.values().map(|d| d.tier).max().unwrap_or(0);
        assert!(
            max_tier >= 7,
            "expected the tree to span at least 7 tiers (wave 2's ascension \
             capstones), got max tier {max_tier}"
        );

        // Genuine DAG, not a single-file line: several techs must have 2+
        // direct prerequisites (convergence points) — wave 2 deliberately
        // added many cross-category convergences, not just one token point.
        let convergence_count = registry
            .techs
            .values()
            .filter(|d| d.prerequisites.len() >= 2)
            .count();
        assert!(
            convergence_count >= 10,
            "expected at least 10 DAG convergence points (techs with 2+ \
             prerequisites), got {convergence_count}"
        );

        // Several techs must have 2+ direct dependents (fan-out points).
        let mut dependents: HashMap<&str, u32> = HashMap::new();
        for def in registry.techs.values() {
            for prereq in &def.prerequisites {
                *dependents.entry(prereq.as_str()).or_insert(0) += 1;
            }
        }
        let fan_out_count = dependents.values().filter(|&&count| count >= 2).count();
        assert!(
            fan_out_count >= 5,
            "expected at least 5 DAG fan-out points (techs with 2+ direct \
             dependents), got {fan_out_count}"
        );
    }

    /// Done-when (pacing check, issue #236's DoD): at the baseline
    /// `research_lab`-only output rate (5 research/sol, no `physics_lab`),
    /// clearing the entire authored tree takes far more than a token number
    /// of strategic months — i.e. tree size is scarce relative to research
    /// output, not trivially beelinable.
    #[test]
    fn real_tech_yaml_is_not_trivially_beelinable_at_baseline_research_rate() {
        let Some(yaml) = read_real_tech_yaml() else {
            return;
        };
        let registry = load_tech_registry(&yaml).expect("content/base/tech.yaml must parse");

        let total_cost: f32 = registry.techs.values().map(|d| d.research_cost).sum();

        // Baseline: one starter research_lab, 5 research/sol, 30 sols/month
        // (DEFAULT_SOLS_PER_MONTH) = 150 research/month, no physics_lab or
        // AI/quantum bonuses yet (those are themselves deep in the tree).
        let baseline_research_per_month = 5.0 * 30.0;
        let months_to_clear_tree = total_cost / baseline_research_per_month;

        assert!(
            months_to_clear_tree > 300.0,
            "expected clearing the whole tree to take well over 300 \
             strategic months at baseline output (scarcity, not a beeline); \
             got {months_to_clear_tree:.1} months for {total_cost:.0} total cost"
        );
    }
}
