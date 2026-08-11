//! Building model: placed buildings and the construction queue.
//!
//! A colony owns a list of completed [`PlacedBuilding`]s and an ordered
//! [`ConstructionQueue`] of in-progress projects. Build slots limit how many
//! buildings a colony can support; slot capacity is increased by technology.

use uuid::Uuid;

/// Unique identifier for a construction project.
pub type ProjectId = Uuid;

/// A building that has been fully constructed and is operational.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlacedBuilding {
    /// Stable identifier for this placed instance.
    pub id: Uuid,
    /// Content-pack key identifying the building type.
    pub building_type: String,
    /// Number of build slots consumed by this building.
    pub slot_cost: u32,
    /// Staffing priority: `1` is staffed first, [`MAX_BUILDING_PRIORITY`] last
    /// (issue #307).
    ///
    /// Seeded from the building's [`BuildingDef::default_priority`] at
    /// placement, then adjustable by the player. `#[serde(default)]` — a
    /// pre-#307 save loads every building at
    /// [`DEFAULT_BUILDING_PRIORITY`], which is the same footing they all
    /// implicitly had under the old colony-wide labour ratio.
    ///
    /// [`BuildingDef::default_priority`]: crate::content::types::BuildingDef::default_priority
    /// [`MAX_BUILDING_PRIORITY`]: crate::content::types::MAX_BUILDING_PRIORITY
    /// [`DEFAULT_BUILDING_PRIORITY`]: crate::content::types::DEFAULT_BUILDING_PRIORITY
    #[serde(default = "default_placed_priority")]
    pub priority: u8,
    /// Player-pinned labour allocation that automatic assignment must not touch
    /// (issue #307).
    ///
    /// `None` — the default — leaves the building in the automatic pool.
    /// `Some(n)` pins exactly `n` workers to it: the allocator hands those out
    /// before it considers anything else, and never reclaims them, however
    /// short the colony gets.
    #[serde(default)]
    pub labour_lock: Option<u32>,
    /// Per-type instance number, `1`-based, assigned at construction (issue #307).
    ///
    /// Gives the player something to tell two mines apart by, which per-instance
    /// staffing needs — ranking three mines against each other is meaningless if
    /// they all read as "Mine".
    ///
    /// **Numbers are never reused.** Demolishing "Mine 2" of three leaves
    /// `1, 3`, and the next mine built is `4`. Gaps are deliberate: the
    /// alternative renumbers surviving buildings behind the player's back, so the
    /// building they set to priority 1 silently becomes a different label.
    ///
    /// `0` means unassigned — a pre-#307 save loads that way and is backfilled
    /// on load (see `snapshot`). Treat `0` as "no number to show".
    #[serde(default)]
    pub ordinal: u32,
    /// Player-chosen name, overriding the auto-numbered default (issue #307).
    ///
    /// `None` — the default — displays as `"<Type Name> <ordinal>"`. Naming is
    /// opt-in so there's no friction at build time, but a building that matters
    /// can be labelled ("North Vein Mine").
    #[serde(default)]
    pub name: Option<String>,
    /// Whether this building is mothballed (issue #309).
    ///
    /// A paused building is excluded from the production pass entirely —
    /// [`crate::ProductionInput`] never gets built for it — rather than
    /// included with each of its draws individually zeroed. That single
    /// exclusion is what makes every other release in the issue (labour,
    /// power, commodity inputs, maintenance, waste it never generates) a
    /// consequence instead of six separate special cases, and it is why
    /// pausing beats even a [`Self::labour_lock`]: an excluded building offers
    /// zero labour demand, so a lock on it can never claim anything.
    ///
    /// **Build slots are the one thing that must survive this exclusion.**
    /// Slot accounting ([`crate::colony::Colony::slots_used`]) sums
    /// `slot_cost` directly off the placed-building list and does not touch
    /// the production pass, so pausing never frees a slot — the building's
    /// physical footprint is still standing, only its utilities are off.
    ///
    /// Per-instance, not per-type — matching [`Self::priority`],
    /// [`Self::labour_lock`], and [`Self::name`], all of which are already
    /// per-`PlacedBuilding` despite only the *default* being authored per
    /// type. Pausing one of three mines and leaving the other two running is
    /// the whole point.
    #[serde(default)]
    pub paused: bool,
    /// Physical condition, `1.0` pristine down to `0.0` derelict (issue #384).
    ///
    /// Degrades while this building's maintenance goes unpaid and recovers
    /// slowly while it is paid — see [`crate::condition`] for the model and
    /// the rates. Below [`crate::condition::BREAKDOWN_THRESHOLD`] it also
    /// starts carrying a per-sol chance of [`Self::broken`].
    ///
    /// `#[serde(default = ...)]` to [`crate::condition::CONDITION_NEW`]: a
    /// save predating this loads every building pristine, which is the honest
    /// reading — those buildings were never subject to wear, so inventing a
    /// history of neglect for them would break games on load.
    #[serde(default = "default_condition")]
    pub condition: f32,
    /// Whether this building has broken down and is awaiting repair (#384).
    ///
    /// A broken building is excluded from the production pass exactly as a
    /// [`Self::paused`] one is, and for the same reason: one exclusion frees
    /// its labour, power, inputs, and upkeep together, rather than zeroing
    /// each by hand. It keeps occupying its build slot — the wreck is still
    /// standing — and only [`crate::Command::RepairBuilding`] clears it.
    ///
    /// Deliberately a flag rather than "condition == 0": breakdown is a
    /// discrete event that can strike at any condition below the threshold,
    /// so a building can be broken at `0.3` and a different one limping along
    /// unbroken at `0.05`. Collapsing the two would erase that distinction and
    /// make the roll meaningless.
    #[serde(default)]
    pub broken: bool,
}

fn default_condition() -> f32 {
    crate::condition::CONDITION_NEW
}

fn default_placed_priority() -> u8 {
    crate::content::types::DEFAULT_BUILDING_PRIORITY
}

/// Longest accepted player-supplied building name.
///
/// Generous enough for any reasonable label while keeping a pathological name out
/// of saves and UI layout.
pub const MAX_BUILDING_NAME_LEN: usize = 64;

impl PlacedBuilding {
    /// Create a new placed building instance at the default staffing priority.
    ///
    /// Prefer [`Self::with_priority`] where the building's
    /// [`BuildingDef`](crate::content::types::BuildingDef) is in hand, so the
    /// authored default is honoured.
    #[must_use]
    pub fn new(building_type: impl Into<String>, slot_cost: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            building_type: building_type.into(),
            slot_cost,
            priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            labour_lock: None,
            ordinal: 0,
            name: None,
            paused: false,
            condition: crate::condition::CONDITION_NEW,
            broken: false,
        }
    }

    /// Create a new placed building at an explicit staffing priority, clamped
    /// into `1..=MAX_BUILDING_PRIORITY`.
    ///
    /// Clamping rather than erroring is deliberate here: this takes an authored
    /// content value, and a pack with `default_priority: 0` should load and
    /// behave sensibly rather than fail the whole colony. Player-driven changes
    /// go through `Command::SetBuildingPriority`, which *does* reject
    /// out-of-range input.
    #[must_use]
    pub fn with_priority(building_type: impl Into<String>, slot_cost: u32, priority: u8) -> Self {
        Self {
            priority: priority.clamp(1, crate::content::types::MAX_BUILDING_PRIORITY),
            ..Self::new(building_type, slot_cost)
        }
    }

    /// The next unused instance number for `building_type` among `existing`.
    ///
    /// One past the highest already in use, so numbers are never reused — see
    /// [`Self::ordinal`] for why that matters. Returns `1` for the first of a type.
    #[must_use]
    pub fn next_ordinal(existing: &[Self], building_type: &str) -> u32 {
        existing
            .iter()
            .filter(|b| b.building_type == building_type)
            .map(|b| b.ordinal)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    /// Assign this building the next unused instance number for its type.
    #[must_use]
    pub fn numbered_within(mut self, existing: &[Self]) -> Self {
        self.ordinal = Self::next_ordinal(existing, &self.building_type);
        self
    }

    /// What to call this building in the UI.
    ///
    /// The player's name if they set one, else `"<type_name> <ordinal>"` — pass
    /// the authored [`BuildingDef::name`] as `type_name`. An unnumbered building
    /// (ordinal `0`, i.e. a pre-#307 save not yet backfilled) falls back to the
    /// bare type name rather than showing a meaningless "Mine 0".
    ///
    /// [`BuildingDef::name`]: crate::content::types::BuildingDef::name
    #[must_use]
    pub fn display_name(&self, type_name: &str) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }
        if self.ordinal == 0 {
            return type_name.to_owned();
        }
        format!("{type_name} {}", self.ordinal)
    }
}

/// An in-progress construction project.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstructionProject {
    /// Stable identifier for this project (used for cancellation).
    pub id: ProjectId,
    /// Content-pack key of the building being constructed.
    pub building_type: String,
    /// Number of build slots reserved while under construction.
    pub slot_cost: u32,
    /// Labor units consumed from the colony pool each turn.
    pub labor_per_turn: u32,
    /// Total commodity cost of the project, drawn from the owning pool in
    /// equal per-sol instalments as construction proceeds (issue #306).
    ///
    /// Also the basis for [`Self::cancel_refund`]. Costs are paid over time
    /// rather than up front so that refund-on-cancel is coherent: refunding
    /// half of the *progress-proportional* spend only means anything if spend
    /// tracks progress.
    pub construction_cost: Vec<(String, f64)>,
    /// Total number of turns required to complete construction.
    pub total_turns: u32,
    /// Turns of construction already completed.
    pub turns_completed: u32,
    /// The placed building this project repairs, if it is a repair (#451).
    ///
    /// `None` — the default, and every project before this existed — is an
    /// ordinary build that ends in a new [`PlacedBuilding`]. `Some(id)` ends
    /// instead by clearing that instance's [`PlacedBuilding::broken`] flag.
    ///
    /// Modelled as a project rather than as its own queue because a repair is
    /// the same shape of work as a build: it takes sols, draws labour every
    /// sol, and pays for materials in instalments — all of which
    /// [`ConstructionProject`] already does. A repair-specific mechanism would
    /// have duplicated every one of those, including the stall handling.
    ///
    /// A repair carries `slot_cost: 0`. The wreck already occupies its slot
    /// (#384 kept it standing), so reserving another would charge the colony
    /// twice for one building.
    #[serde(default)]
    pub repair_target: Option<Uuid>,
}

impl ConstructionProject {
    /// Create a new construction project.
    #[must_use]
    pub fn new(
        building_type: impl Into<String>,
        slot_cost: u32,
        labor_per_turn: u32,
        construction_cost: Vec<(String, f64)>,
        total_turns: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            building_type: building_type.into(),
            slot_cost,
            labor_per_turn,
            construction_cost,
            total_turns,
            turns_completed: 0,
            repair_target: None,
        }
    }

    /// Create a repair project for a broken placed building (issue #451).
    ///
    /// `slot_cost` is zero — the wreck already holds its slot — and completion
    /// returns the existing instance to service rather than placing a new one.
    #[must_use]
    pub fn repair(
        building_type: impl Into<String>,
        building_id: Uuid,
        labor_per_turn: u32,
        cost: Vec<(String, f64)>,
        total_turns: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            building_type: building_type.into(),
            slot_cost: 0,
            labor_per_turn,
            construction_cost: cost,
            total_turns,
            turns_completed: 0,
            repair_target: Some(building_id),
        }
    }

    /// Whether this project repairs an existing building rather than building
    /// a new one (issue #451).
    #[must_use]
    pub fn is_repair(&self) -> bool {
        self.repair_target.is_some()
    }

    /// Returns `true` if all construction turns are complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.turns_completed >= self.total_turns
    }

    /// Returns the proportion of construction complete, in `[0.0, 1.0]`.
    #[must_use]
    pub fn progress_fraction(&self) -> f64 {
        if self.total_turns == 0 {
            1.0
        } else {
            f64::from(self.turns_completed) / f64::from(self.total_turns)
        }
    }

    /// The commodity instalment owed for one sol of construction (issue #306).
    ///
    /// Total cost spread evenly across [`Self::total_turns`], so after `n` sols
    /// exactly `n / total_turns` of the cost has been drawn — which is what
    /// makes [`Self::cancel_refund`]'s use of [`Self::progress_fraction`] as a
    /// stand-in for "fraction spent" correct.
    ///
    /// A zero-turn project (completes on its first tick) owes the whole cost at
    /// once rather than dividing by zero.
    #[must_use]
    pub fn materials_draw_per_turn(&self) -> Vec<(String, f64)> {
        if self.total_turns == 0 {
            return self.construction_cost.clone();
        }
        let turns = f64::from(self.total_turns);
        self.construction_cost
            .iter()
            .map(|(id, qty)| (id.clone(), qty / turns))
            .collect()
    }

    /// Compute the 50 % commodity refund for cancelling this project.
    ///
    /// Only costs already spent (proportional to `turns_completed`) are
    /// considered; the refund is 50 % of that amount.
    #[must_use]
    pub fn cancel_refund(&self) -> Vec<(String, f64)> {
        let fraction_spent = self.progress_fraction();
        self.construction_cost
            .iter()
            .map(|(id, qty)| (id.clone(), qty * fraction_spent * 0.5))
            .collect()
    }
}

/// Tolerance for "can this pool afford the instalment" checks.
///
/// An instalment is `total / turns`, so summing every instalment need not
/// reproduce the total bit-for-bit. Without slack, a pool stocked with exactly
/// the authored cost can stall on the final sol by a rounding crumb.
const AFFORDABILITY_EPSILON: f64 = 1e-9;

/// What one sol of construction did to the active project (issue #306).
#[derive(Debug, Clone)]
pub enum ConstructionTick {
    /// The queue is empty; nothing was charged.
    Idle,
    /// The pool could not cover this sol's instalment, so **nothing** was
    /// withdrawn and progress did not advance.
    ///
    /// All-or-nothing deliberately: a partial draw would consume materials
    /// while buying no progress, which reads to the player as theft.
    Stalled {
        /// The project that could not be funded.
        project_id: ProjectId,
        /// Content-pack key of the building it would produce.
        building_type: String,
        /// Per-commodity amount the colony is **genuinely** short.
        ///
        /// Measured against raw stock, so a commodity the player merely reserved
        /// never appears here — that is [`Self::StalledByReserve`]'s job, and
        /// mixing the two would tell the player to produce something they already
        /// have.
        missing: Vec<(String, f64)>,
    },
    /// The instalment was affordable from raw stock but not from *unreserved*
    /// stock, so the player's own commodity reserve is what stopped it
    /// (issue #355).
    ///
    /// Split from [`Self::Stalled`] because the two need opposite advice. A
    /// genuine shortage says "go get more"; this says "you asked me not to spend
    /// this" — and reporting it as a shortage would have the player hunting for
    /// metal while a full warehouse sits behind their own floor.
    StalledByReserve {
        /// The project that could not be funded.
        project_id: ProjectId,
        /// Content-pack key of the building it would produce.
        building_type: String,
        /// Per-commodity amount the reserve withheld from this sol's instalment.
        withheld: Vec<(String, f64)>,
    },
    /// The instalment was paid and the project advanced but is not finished.
    Progressed,
    /// The instalment was paid and the project finished; the caller should
    /// place the resulting building.
    Completed(ConstructionProject),
}

/// The ordered queue of in-progress construction projects for one colony.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ConstructionQueue {
    /// Projects in arrival order; the first element is the active project.
    pub projects: Vec<ConstructionProject>,
}

impl ConstructionQueue {
    /// Create a new empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Total build slots reserved by all queued projects.
    #[must_use]
    pub fn slots_reserved(&self) -> u32 {
        self.projects.iter().map(|p| p.slot_cost).sum()
    }

    /// Push a project onto the back of the queue.
    pub fn enqueue(&mut self, project: ConstructionProject) {
        self.projects.push(project);
    }

    /// Remove and return the project with the given id, if present.
    pub fn cancel(&mut self, project_id: ProjectId) -> Option<ConstructionProject> {
        if let Some(pos) = self.projects.iter().position(|p| p.id == project_id) {
            Some(self.projects.remove(pos))
        } else {
            None
        }
    }

    /// Charge one sol of construction materials to `pool`, then advance the
    /// active project (issue #306).
    ///
    /// This is the production path; [`Self::tick_active`] is the unfunded
    /// primitive it delegates to. Prefer this everywhere a real colony or
    /// outpost queue advances, so that `construction_cost` is actually paid.
    pub fn tick_active_charging(
        &mut self,
        pool: &mut super::ColonyPool,
        reserves: &std::collections::HashMap<String, f64>,
    ) -> ConstructionTick {
        let Some(active) = self.projects.first() else {
            return ConstructionTick::Idle;
        };
        let instalment = active.materials_draw_per_turn();
        // Spendable stock is what sits above the player's floor (issue #355).
        let spendable = |id: &str| {
            let floor = reserves.get(id).copied().unwrap_or(0.0).max(0.0);
            (pool.amount(id) - floor).max(0.0)
        };
        let missing: Vec<(String, f64)> = instalment
            .iter()
            .filter_map(|(id, qty)| {
                let short = qty - spendable(id);
                (short > AFFORDABILITY_EPSILON).then(|| (id.clone(), short))
            })
            .collect();
        if !missing.is_empty() {
            // Distinguish "you don't have it" from "you told me not to spend it"
            // (#355): if raw stock would have covered every commodity, the floor
            // is the only thing in the way.
            let raw_covers_everything = instalment
                .iter()
                .all(|(id, qty)| qty - pool.amount(id) <= AFFORDABILITY_EPSILON);
            if raw_covers_everything {
                return ConstructionTick::StalledByReserve {
                    project_id: active.id,
                    building_type: active.building_type.clone(),
                    withheld: missing,
                };
            }
            // A genuine shortage names only what is genuinely absent — measured
            // against **raw** stock, not the reserve-adjusted figure.
            //
            // Mixed case: an instalment needs glass the colony hasn't got and
            // steel it has but reserved. Listing the steel as "missing" here
            // would tell the player to go produce steel they are standing on.
            // Once the glass arrives the sol stalls again and reports
            // `StalledByReserve`, which is the accurate advice at that point —
            // each event says one true thing rather than one true and one
            // misleading.
            let genuinely_absent: Vec<(String, f64)> = instalment
                .iter()
                .filter_map(|(id, qty)| {
                    let short = qty - pool.amount(id);
                    (short > AFFORDABILITY_EPSILON).then(|| (id.clone(), short))
                })
                .collect();
            return ConstructionTick::Stalled {
                project_id: active.id,
                building_type: active.building_type.clone(),
                missing: genuinely_absent,
            };
        }
        for (id, qty) in &instalment {
            // `withdraw` clamps to what's held, which absorbs the epsilon of
            // slack the affordability check above allows.
            pool.withdraw(id, *qty);
        }
        match self.tick_active() {
            Some(done) => ConstructionTick::Completed(done),
            None => ConstructionTick::Progressed,
        }
    }

    /// Advance the active (first) project by one turn; return it if complete.
    ///
    /// Returns `Some(project)` when the project just finished (caller should
    /// add a [`PlacedBuilding`] and pop the queue), or `None` if not yet done.
    ///
    /// Charges nothing — see [`Self::tick_active_charging`] for the funded
    /// version used by the turn processor.
    pub fn tick_active(&mut self) -> Option<ConstructionProject> {
        let active = self.projects.first_mut()?;
        active.turns_completed += 1;
        if active.is_complete() {
            Some(self.projects.remove(0))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No player reserves — the default for tests that aren't about #355.
    fn no_reserves() -> std::collections::HashMap<String, f64> {
        std::collections::HashMap::new()
    }

    fn sample_project(total_turns: u32) -> ConstructionProject {
        ConstructionProject::new(
            "greenhouse",
            1,
            5,
            vec![("steel".into(), 100.0), ("glass".into(), 50.0)],
            total_turns,
        )
    }

    #[test]
    fn project_starts_incomplete() {
        let p = sample_project(3);
        assert!(!p.is_complete());
        assert!((p.progress_fraction() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn project_is_complete_after_all_turns() {
        let mut p = sample_project(2);
        p.turns_completed = 2;
        assert!(p.is_complete());
        assert!((p.progress_fraction() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cancel_refund_is_50_pct_of_spent_cost() {
        let mut p = sample_project(4);
        p.turns_completed = 2; // 50 % done
        let refund = p.cancel_refund();
        // 50 % spent × 50 % refund = 25 % of total cost
        let steel = refund.iter().find(|(id, _)| id == "steel").unwrap();
        assert!((steel.1 - 25.0).abs() < 1e-9);
    }

    #[test]
    fn cancel_refund_zero_when_just_started() {
        let p = sample_project(4); // turns_completed = 0
        let refund = p.cancel_refund();
        for (_, qty) in &refund {
            assert!(*qty < 1e-9);
        }
    }

    #[test]
    fn materials_draw_splits_cost_across_turns() {
        let p = sample_project(4);
        let draw = p.materials_draw_per_turn();
        let steel = draw.iter().find(|(id, _)| id == "steel").unwrap();
        let glass = draw.iter().find(|(id, _)| id == "glass").unwrap();
        assert!((steel.1 - 25.0).abs() < 1e-9);
        assert!((glass.1 - 12.5).abs() < 1e-9);
    }

    #[test]
    fn materials_draw_for_zero_turn_project_is_the_whole_cost() {
        let p = sample_project(0);
        let draw = p.materials_draw_per_turn();
        let steel = draw.iter().find(|(id, _)| id == "steel").unwrap();
        assert!((steel.1 - 100.0).abs() < 1e-9);
    }

    /// A pool stocked with exactly the authored cost must fund every sol,
    /// including the last — the instalments need not re-sum to the total
    /// bit-for-bit, which is what [`AFFORDABILITY_EPSILON`] absorbs.
    #[test]
    fn exact_stock_funds_construction_to_completion() {
        let mut pool = super::super::ColonyPool::new();
        pool.deposit("steel", 100.0);
        pool.deposit("glass", 50.0);
        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(3)); // 3 turns → 33.333… steel per sol

        for turn in 1..=2 {
            match q.tick_active_charging(&mut pool, &no_reserves()) {
                ConstructionTick::Progressed => {}
                other => panic!("turn {turn} should progress, got {other:?}"),
            }
        }
        match q.tick_active_charging(&mut pool, &no_reserves()) {
            ConstructionTick::Completed(done) => assert_eq!(done.building_type, "greenhouse"),
            other => panic!("final turn should complete, got {other:?}"),
        }
        assert!(
            pool.amount("steel") < 1e-6,
            "steel: {}",
            pool.amount("steel")
        );
        assert!(q.projects.is_empty());
    }

    /// An unaffordable sol must withdraw **nothing** — a partial draw would
    /// consume materials while buying no progress.
    #[test]
    fn stalled_tick_withdraws_nothing_and_makes_no_progress() {
        let mut pool = super::super::ColonyPool::new();
        pool.deposit("steel", 100.0); // plenty of steel …
        pool.deposit("glass", 1.0); // … but nowhere near enough glass
        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(4)); // needs 25 steel + 12.5 glass per sol

        match q.tick_active_charging(&mut pool, &no_reserves()) {
            ConstructionTick::Stalled {
                building_type,
                missing,
                ..
            } => {
                assert_eq!(building_type, "greenhouse");
                let glass = missing.iter().find(|(id, _)| id == "glass").unwrap();
                assert!((glass.1 - 11.5).abs() < 1e-9, "missing glass: {}", glass.1);
                assert!(
                    !missing.iter().any(|(id, _)| id == "steel"),
                    "steel is in stock and must not be reported missing"
                );
            }
            other => panic!("expected a stall, got {other:?}"),
        }
        assert!(
            (pool.amount("steel") - 100.0).abs() < 1e-9,
            "steel was spent"
        );
        assert!((pool.amount("glass") - 1.0).abs() < 1e-9, "glass was spent");
        assert_eq!(q.projects[0].turns_completed, 0);
    }

    #[test]
    fn charging_tick_on_empty_queue_is_idle() {
        let mut pool = super::super::ColonyPool::new();
        let mut q = ConstructionQueue::new();
        assert!(matches!(
            q.tick_active_charging(&mut pool, &no_reserves()),
            ConstructionTick::Idle
        ));
    }

    /// Mixed case (issue #355): one commodity genuinely absent, another merely
    /// reserved. The stall must name only the absent one.
    #[test]
    fn a_mixed_shortage_names_only_what_is_genuinely_absent() {
        let mut pool = super::super::ColonyPool::new();
        pool.deposit("steel", 100.0); // held, but reserved below
                                      // No glass at all.
        let mut reserves = std::collections::HashMap::new();
        reserves.insert("steel".to_string(), 100.0);

        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(4)); // needs 25 steel + 12.5 glass per sol

        match q.tick_active_charging(&mut pool, &reserves) {
            ConstructionTick::Stalled { missing, .. } => {
                assert!(
                    missing.iter().any(|(id, _)| id == "glass"),
                    "glass is genuinely absent and must be reported, got {missing:?}"
                );
                assert!(
                    !missing.iter().any(|(id, _)| id == "steel"),
                    "steel is in stock behind the player's own floor — reporting it                      as missing would send them producing what they already have;                      got {missing:?}"
                );
            }
            other => panic!("a genuine shortage outranks the reserve, got {other:?}"),
        }
        assert!((pool.amount("steel") - 100.0).abs() < 1e-9, "nothing spent");
    }

    /// A project that stalls at 0 % must refund nothing, so cancelling it
    /// cannot mint materials that were never paid.
    #[test]
    fn stalled_project_refunds_nothing_on_cancel() {
        let mut pool = super::super::ColonyPool::new();
        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(4));
        let project_id = q.projects[0].id;

        for _ in 0..4 {
            assert!(matches!(
                q.tick_active_charging(&mut pool, &no_reserves()),
                ConstructionTick::Stalled { .. }
            ));
        }
        let cancelled = q.cancel(project_id).unwrap();
        for (_, qty) in cancelled.cancel_refund() {
            assert!(qty < 1e-9, "refund from an unpaid project: {qty}");
        }
    }

    #[test]
    fn queue_slots_reserved_sums_all_projects() {
        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(3));
        let mut p2 = sample_project(2);
        p2.slot_cost = 2;
        q.enqueue(p2);
        assert_eq!(q.slots_reserved(), 3);
    }

    #[test]
    fn tick_active_returns_none_before_completion() {
        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(3));
        assert!(q.tick_active().is_none()); // turn 1/3
        assert!(q.tick_active().is_none()); // turn 2/3
    }

    #[test]
    fn tick_active_returns_project_on_completion() {
        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(2));
        q.tick_active(); // turn 1
        let done = q.tick_active(); // turn 2 — complete
        assert!(done.is_some());
        assert!(q.projects.is_empty());
    }

    #[test]
    fn cancel_removes_project_from_queue() {
        let mut q = ConstructionQueue::new();
        let p = sample_project(5);
        let pid = p.id;
        q.enqueue(p);
        let removed = q.cancel(pid);
        assert!(removed.is_some());
        assert!(q.projects.is_empty());
    }

    #[test]
    fn cancel_unknown_id_returns_none() {
        let mut q = ConstructionQueue::new();
        q.enqueue(sample_project(3));
        assert!(q.cancel(Uuid::new_v4()).is_none());
    }

    // ── Instance identity (issue #307) ───────────────────────────────────────

    #[test]
    fn instance_numbers_start_at_one_and_count_up_per_type() {
        let mut placed: Vec<PlacedBuilding> = Vec::new();
        for expected in 1..=3 {
            let b = PlacedBuilding::new("mine", 1).numbered_within(&placed);
            assert_eq!(b.ordinal, expected);
            placed.push(b);
        }
        // A different type numbers independently, not continuing the mine count.
        let farm = PlacedBuilding::new("farm", 1).numbered_within(&placed);
        assert_eq!(farm.ordinal, 1, "each type counts from one");
    }

    /// Numbers are never recycled. Renumbering survivors would silently relabel
    /// the building the player set to priority 1.
    #[test]
    fn a_demolished_buildings_number_is_not_reused() {
        let mut placed: Vec<PlacedBuilding> = Vec::new();
        for _ in 0..3 {
            let b = PlacedBuilding::new("mine", 1).numbered_within(&placed);
            placed.push(b);
        }
        assert_eq!(
            placed.iter().map(|b| b.ordinal).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );

        // Demolish "Mine 2".
        placed.retain(|b| b.ordinal != 2);
        assert_eq!(
            placed.iter().map(|b| b.ordinal).collect::<Vec<_>>(),
            vec![1, 3],
            "survivors keep their numbers rather than closing the gap"
        );

        let next = PlacedBuilding::new("mine", 1).numbered_within(&placed);
        assert_eq!(
            next.ordinal, 4,
            "continues past the highest, not into the gap"
        );
    }

    #[test]
    fn display_name_falls_back_to_the_type_name_and_number() {
        let b = PlacedBuilding::new("mine", 1).numbered_within(&[]);
        assert_eq!(b.display_name("Ore Mine"), "Ore Mine 1");
    }

    #[test]
    fn a_player_name_overrides_the_numbered_default() {
        let mut b = PlacedBuilding::new("mine", 1).numbered_within(&[]);
        b.name = Some("North Vein".into());
        assert_eq!(b.display_name("Ore Mine"), "North Vein");
    }

    /// A pre-#307 save loads unnumbered. Showing "Ore Mine 0" would be worse than
    /// showing no number, and the backfill on load fixes it properly.
    #[test]
    fn an_unnumbered_building_shows_the_bare_type_name() {
        let b = PlacedBuilding::new("mine", 1);
        assert_eq!(b.ordinal, 0);
        assert_eq!(b.display_name("Ore Mine"), "Ore Mine");
    }
}
