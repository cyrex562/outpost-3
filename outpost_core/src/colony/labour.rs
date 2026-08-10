//! Automatic labour allocation across a colony's buildings (issue #307).
//!
//! Labour used to be a single colony-wide ratio: total workers divided by total
//! worker slots, applied uniformly, so every building throttled together and a
//! shortage hurt the greenhouse exactly as much as the ore mine. This module
//! replaces that with an explicit per-building allocation the player can steer.
//!
//! # The rules
//!
//! 1. **Locked buildings are staffed first.** A [`PlacedBuilding::labour_lock`]
//!    pins an allocation the allocator hands out before considering anything
//!    else, and never reclaims.
//! 2. **Everything else is staffed in priority order** — `1` first,
//!    [`MAX_BUILDING_PRIORITY`] last — until the workforce runs out.
//!
//! There is deliberately **no category ranking**. An earlier design ranked
//! [`BuildingCategory`](crate::content::BuildingCategory) first and used
//! priority as a tiebreak, but categories cut across intent: a farm and an ore
//! mine are both `Extraction`, so a food crisis would have competed with a metal
//! shortage on equal footing. Authored per-building defaults
//! ([`BuildingDef::default_priority`]) express the same intent directly, and a
//! content author can retune them without touching Rust.
//!
//! [`BuildingDef::default_priority`]: crate::content::types::BuildingDef::default_priority
//! [`MAX_BUILDING_PRIORITY`]: crate::content::types::MAX_BUILDING_PRIORITY

use uuid::Uuid;

use super::building::PlacedBuilding;
use crate::content::ContentRegistry;

/// What one building got out of the allocation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LabourAllocation {
    /// The placed instance this applies to.
    pub building_id: Uuid,
    /// Content-pack key, carried for convenience in reporting.
    pub building_type: String,
    /// Workers the building wants — its `worker_slots`, or `0` if it has no
    /// recipe to run and therefore no job to offer.
    pub demand: u32,
    /// Workers it actually got. `< demand` means understaffed.
    pub assigned: u32,
    /// Whether this allocation came from a player lock rather than the
    /// automatic pass.
    pub locked: bool,
    /// The priority the allocator used.
    pub priority: u8,
}

impl LabourAllocation {
    /// `true` when the building wanted workers and didn't get all of them.
    #[must_use]
    pub fn understaffed(&self) -> bool {
        self.assigned < self.demand
    }

    /// Fraction of this building's labour demand that was met, in `[0.0, 1.0]`.
    ///
    /// A building with no demand is fully staffed by definition and returns
    /// `1.0` — it has no jobs to leave empty, so it must not be reported as
    /// labour-starved.
    #[must_use]
    pub fn ratio(&self) -> f64 {
        if self.demand == 0 {
            return 1.0;
        }
        (f64::from(self.assigned) / f64::from(self.demand)).min(1.0)
    }
}

/// The result of allocating a colony's workforce.
///
/// Serialisable because the colony stores the latest plan (`Colony::last_labour`)
/// so between-sol queries report what production actually did, rather than
/// recomputing an estimate that could disagree with it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LabourPlan {
    /// One entry per building, in the order they were considered.
    pub allocations: Vec<LabourAllocation>,
    /// Workers left over with nowhere to go.
    pub idle: u32,
    /// Jobs left unfilled across the colony.
    pub unfilled: u32,
}

impl LabourPlan {
    /// Look up one building's allocation.
    #[must_use]
    pub fn for_building(&self, building_id: Uuid) -> Option<&LabourAllocation> {
        self.allocations
            .iter()
            .find(|a| a.building_id == building_id)
    }

    /// Every building that wanted workers and didn't get all of them.
    pub fn understaffed(&self) -> impl Iterator<Item = &LabourAllocation> {
        self.allocations.iter().filter(|a| a.understaffed())
    }
}

/// One building offered to the allocator, with its labour demand already
/// resolved by the caller.
///
/// Exists so the production pipeline can gate demand on things the allocator
/// has no way to know about. A mine with empty ore veins, or one browned out to
/// a standstill, offers **no jobs this sol** — the workers it would otherwise
/// hold should go somewhere they can accomplish something. Production computes
/// that feasibility (see
/// [`process_production_scaled`](super::production::process_production_scaled))
/// and passes the gated demand in here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabourCandidate {
    /// The placed instance this describes.
    pub id: Uuid,
    /// Content-pack key, used for reporting and as a deterministic tiebreak.
    pub building_type: String,
    /// Staffing priority: `1` is staffed first.
    pub priority: u8,
    /// Player-pinned allocation, if any.
    pub labour_lock: Option<u32>,
    /// Workers this building wants **this sol**, already gated on whatever the
    /// caller knows about its ability to run.
    pub demand: u32,
}

/// Workers a building wants: its `worker_slots`, or `0` when it has no recipe.
///
/// Mirrors the filter the old colony-wide
/// [`labor_demanded`](super::production::labor_demanded) used, so a storage silo
/// or habitat doesn't soak up staff it has no use for.
fn demand_for(building: &PlacedBuilding, registry: &ContentRegistry) -> u32 {
    let Some(def) = registry.building(&building.building_type) else {
        return 0;
    };
    if super::production::has_recipe(&def.id, registry) {
        def.worker_slots
    } else {
        0
    }
}

/// Distribute `available` workers across `buildings`.
///
/// Locks are honoured first, then the rest in priority order. A building that
/// can only be partly staffed gets what remains rather than nothing, so labour
/// degrades smoothly instead of falling off a cliff at each boundary.
///
/// # Determinism
///
/// Ties are broken by `(priority, building_type, id)`, so the same colony always
/// produces the same plan — a requirement for reproducible replays and for
/// saved-state resumption to continue identically.
///
/// # Over-locking
///
/// A lock is clamped two ways: to the building's own demand (pinning 50 workers
/// to a 5-slot greenhouse would waste 45), and to whatever workforce is left
/// when its turn comes. Locks are satisfied in priority order among themselves,
/// so if the player pins more workers than the colony has, the lower-priority
/// locks are the ones that come up short — the same rule the automatic pass
/// follows, rather than a separate failure mode.
#[must_use]
pub fn allocate_labour(
    buildings: &[PlacedBuilding],
    available: u32,
    registry: &ContentRegistry,
) -> LabourPlan {
    let candidates: Vec<LabourCandidate> = buildings
        .iter()
        .map(|b| LabourCandidate {
            id: b.id,
            building_type: b.building_type.clone(),
            priority: b.priority,
            labour_lock: b.labour_lock,
            demand: demand_for(b, registry),
        })
        .collect();
    allocate_from(&candidates, available)
}

/// Distribute `available` workers across candidates whose demand the caller has
/// already resolved.
///
/// The allocation rules are identical to [`allocate_labour`] — which is a thin
/// wrapper over this — and documented there. Use this directly when demand
/// depends on something the registry alone can't answer, such as whether a
/// building has any input left to work on.
#[must_use]
pub fn allocate_from(candidates: &[LabourCandidate], available: u32) -> LabourPlan {
    // Stable consideration order, independent of the order buildings were built.
    let mut order: Vec<&LabourCandidate> = candidates.iter().collect();
    order.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.building_type.cmp(&b.building_type))
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut remaining = available;
    let mut allocations: Vec<LabourAllocation> = Vec::with_capacity(order.len());

    // Pass 1: locked buildings, in priority order.
    for b in &order {
        let Some(lock) = b.labour_lock else { continue };
        let assigned = lock.min(b.demand).min(remaining);
        remaining -= assigned;
        allocations.push(LabourAllocation {
            building_id: b.id,
            building_type: b.building_type.clone(),
            demand: b.demand,
            assigned,
            locked: true,
            priority: b.priority,
        });
    }

    // Pass 2: everything else, in priority order, filling as far as it goes.
    for b in &order {
        if b.labour_lock.is_some() {
            continue;
        }
        let assigned = b.demand.min(remaining);
        remaining -= assigned;
        allocations.push(LabourAllocation {
            building_id: b.id,
            building_type: b.building_type.clone(),
            demand: b.demand,
            assigned,
            locked: false,
            priority: b.priority,
        });
    }

    let unfilled = allocations.iter().map(|a| a.demand - a.assigned).sum();

    LabourPlan {
        allocations,
        idle: remaining,
        unfilled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{
        BuildingCategory, BuildingDef, Ingredient, RecipeDef, DEFAULT_BUILDING_PRIORITY,
    };

    /// `staffed` buildings have a recipe (so they offer jobs); `unstaffed` ones
    /// don't (storage/habitat) and must offer none.
    fn registry() -> ContentRegistry {
        let mut reg = ContentRegistry::default();
        let building = |id: &str, worker_slots: u32| BuildingDef {
            hazard_susceptibility: None,
            repair_cost: vec![],
            id: id.into(),
            name: id.into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
            default_priority: DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
            storage: vec![],
            contamination_reduction: 0.0,
            max_instances: None,
            site_requirements: Vec::new(),
            output_scaling: None,
        };
        for (id, slots) in [("farm", 5), ("mine", 5), ("lab", 3), ("scrubber", 2)] {
            reg.insert_building(building(id, slots));
            reg.insert_recipe(RecipeDef {
                id: format!("{id}_run"),
                name: format!("{id}_run"),
                building: id.into(),
                cycle_sols: 1,
                inputs: vec![],
                outputs: vec![Ingredient {
                    id: "stuff".into(),
                    quantity: 1.0,
                }],
                concurrent: false,
                line: None,
                power_draw: 0.0,
            });
        }
        // A silo with worker slots authored but no recipe — it must still be
        // treated as offering no jobs.
        reg.insert_building(building("silo", 4));
        reg
    }

    fn placed(building_type: &str, priority: u8) -> PlacedBuilding {
        PlacedBuilding::with_priority(building_type, 1, priority)
    }

    #[test]
    fn plenty_of_labour_staffs_everything_and_reports_the_remainder_idle() {
        let reg = registry();
        let buildings = vec![placed("farm", 5), placed("mine", 5)];
        let plan = allocate_labour(&buildings, 20, &reg);

        assert!(plan.allocations.iter().all(|a| a.assigned == a.demand));
        assert_eq!(plan.idle, 10, "20 workers, 10 slots");
        assert_eq!(plan.unfilled, 0);
        assert_eq!(plan.understaffed().count(), 0);
    }

    /// The core of #307: a shortage is *not* spread evenly. The better priority
    /// runs at full rate and the worse one absorbs the whole shortfall.
    ///
    /// The names deliberately **oppose** the priority order — `farm` sorts
    /// before `mine` alphabetically, but here the mine has the better priority.
    /// Without that, the alphabetical tiebreak would produce the expected
    /// result on its own and the test would pass even with priority ignored.
    #[test]
    fn a_shortage_starves_the_worse_priority_first() {
        let reg = registry();
        let buildings = vec![placed("farm", 7), placed("mine", 2)];
        // 5 workers, 10 slots demanded — exactly enough for one of them.
        let plan = allocate_labour(&buildings, 5, &reg);

        let farm = plan
            .allocations
            .iter()
            .find(|a| a.building_type == "farm")
            .expect("farm allocated");
        let mine = plan
            .allocations
            .iter()
            .find(|a| a.building_type == "mine")
            .expect("mine allocated");

        assert_eq!(mine.assigned, 5, "priority 2 is staffed first");
        assert_eq!(
            farm.assigned, 0,
            "priority 7 gets what's left, which is none"
        );
        assert!((mine.ratio() - 1.0).abs() < 1e-9);
        assert!(farm.ratio().abs() < 1e-9);
        assert_eq!(plan.idle, 0);
        assert_eq!(plan.unfilled, 5);

        // This is precisely what the old colony-wide ratio could not do: it
        // would have run both at 50%.
        assert_ne!(farm.assigned, mine.assigned);
    }

    #[test]
    fn a_building_that_can_only_be_partly_staffed_gets_what_is_left() {
        let reg = registry();
        // `mine` has the better priority despite sorting later by name.
        let buildings = vec![placed("farm", 2), placed("mine", 1)];
        // 7 workers: mine takes 5, farm wants 5 and gets the remaining 2.
        let plan = allocate_labour(&buildings, 7, &reg);

        let farm = plan
            .allocations
            .iter()
            .find(|a| a.building_type == "farm")
            .expect("farm allocated");
        assert_eq!(farm.assigned, 2, "partial staffing, not zero");
        assert!((farm.ratio() - 0.4).abs() < 1e-9);
        assert!(farm.understaffed());
        assert_eq!(plan.idle, 0);
    }

    #[test]
    fn a_building_with_no_recipe_offers_no_jobs() {
        let reg = registry();
        // The silo authored 4 worker slots but has no recipe.
        let buildings = vec![placed("silo", 1), placed("farm", 9)];
        // silo sorts after farm, and has the better priority, so if demand were
        // not gated on "has a recipe" it would take workers from the farm.
        let plan = allocate_labour(&buildings, 5, &reg);

        let silo = plan
            .allocations
            .iter()
            .find(|a| a.building_type == "silo")
            .expect("silo present");
        assert_eq!(silo.demand, 0, "no recipe means no jobs");
        assert_eq!(silo.assigned, 0);
        assert!(!silo.understaffed(), "a silo is never labour-short");
        assert!(
            (silo.ratio() - 1.0).abs() < 1e-9,
            "no demand means fully staffed, not starved"
        );

        // Despite its better priority, the silo took nothing from the farm.
        let farm = plan
            .allocations
            .iter()
            .find(|a| a.building_type == "farm")
            .expect("farm present");
        assert_eq!(farm.assigned, 5);
    }

    #[test]
    fn a_lock_is_honoured_ahead_of_a_better_unlocked_priority() {
        let reg = registry();
        let mut locked = placed("mine", 9);
        locked.labour_lock = Some(4);
        let buildings = vec![locked, placed("farm", 1)];

        // 5 workers. Unlocked, the farm (priority 1) would take all 5.
        let plan = allocate_labour(&buildings, 5, &reg);

        let mine = plan.for_building(buildings[0].id).expect("mine allocated");
        assert!(mine.locked);
        assert_eq!(mine.assigned, 4, "the lock wins over a better priority");
        let farm = plan.for_building(buildings[1].id).expect("farm allocated");
        assert_eq!(farm.assigned, 1, "the farm gets only what the lock left");
    }

    #[test]
    fn a_lock_larger_than_the_building_can_use_is_clamped_to_its_demand() {
        let reg = registry();
        let mut greedy = placed("scrubber", 5);
        greedy.labour_lock = Some(50);
        let buildings = vec![greedy, placed("farm", 6)];
        let plan = allocate_labour(&buildings, 10, &reg);

        let scrubber = plan.for_building(buildings[0].id).expect("scrubber");
        assert_eq!(scrubber.assigned, 2, "clamped to its 2 worker slots");
        // The 48 surplus workers were not swallowed.
        let farm = plan.for_building(buildings[1].id).expect("farm");
        assert_eq!(farm.assigned, 5);
        assert_eq!(plan.idle, 3);
    }

    #[test]
    fn locks_beyond_the_workforce_starve_the_worse_priority_lock() {
        let reg = registry();
        // Again the better priority is the alphabetically-later name, so the
        // tiebreak cannot stand in for the priority comparison.
        let mut good = placed("mine", 2);
        good.labour_lock = Some(5);
        let mut bad = placed("farm", 8);
        bad.labour_lock = Some(5);
        let buildings = vec![bad, good];

        // Only 6 workers for 10 pinned.
        let plan = allocate_labour(&buildings, 6, &reg);

        let mine = plan.for_building(buildings[1].id).expect("mine");
        let farm = plan.for_building(buildings[0].id).expect("farm");
        assert_eq!(mine.assigned, 5, "better-priority lock is satisfied first");
        assert_eq!(
            farm.assigned, 1,
            "worse-priority lock absorbs the shortfall"
        );
        assert_eq!(plan.idle, 0);
    }

    #[test]
    fn zero_workforce_leaves_everything_unstaffed_without_panicking() {
        let reg = registry();
        let buildings = vec![placed("farm", 1), placed("mine", 2)];
        let plan = allocate_labour(&buildings, 0, &reg);
        assert!(plan.allocations.iter().all(|a| a.assigned == 0));
        assert_eq!(plan.idle, 0);
        assert_eq!(plan.unfilled, 10);
    }

    #[test]
    fn an_unknown_building_type_offers_no_jobs_rather_than_panicking() {
        let reg = registry();
        let buildings = vec![placed("not_in_registry", 1)];
        let plan = allocate_labour(&buildings, 5, &reg);
        assert_eq!(plan.allocations[0].demand, 0);
        assert_eq!(plan.idle, 5);
    }

    /// Replays and save-resumption depend on this: the same colony must always
    /// produce the same plan, whatever order the buildings happen to sit in.
    #[test]
    fn allocation_is_independent_of_the_order_buildings_were_built() {
        let reg = registry();
        let a = placed("farm", 3);
        let b = placed("mine", 3);
        let c = placed("lab", 3);

        let forward = allocate_labour(&[a.clone(), b.clone(), c.clone()], 8, &reg);
        let reversed = allocate_labour(&[c, b, a], 8, &reg);

        let key = |p: &LabourPlan| {
            let mut v: Vec<(Uuid, u32)> = p
                .allocations
                .iter()
                .map(|x| (x.building_id, x.assigned))
                .collect();
            v.sort();
            v
        };
        assert_eq!(key(&forward), key(&reversed), "same colony, same plan");
    }

    #[test]
    fn equal_priorities_are_broken_deterministically_by_type_then_id() {
        let reg = registry();
        // farm < lab < mine alphabetically, all at the same priority.
        let buildings = vec![placed("mine", 4), placed("lab", 4), placed("farm", 4)];
        let plan = allocate_labour(&buildings, 6, &reg);

        let order: Vec<&str> = plan
            .allocations
            .iter()
            .map(|a| a.building_type.as_str())
            .collect();
        assert_eq!(order, vec!["farm", "lab", "mine"]);
        // farm takes 5, lab takes the last 1, mine gets nothing.
        assert_eq!(plan.allocations[0].assigned, 5);
        assert_eq!(plan.allocations[1].assigned, 1);
        assert_eq!(plan.allocations[2].assigned, 0);
    }

    #[test]
    fn a_placed_building_takes_the_authored_default_priority() {
        let mut reg = registry();
        let mut farm = reg.building("farm").expect("farm def").clone();
        farm.default_priority = 2;
        reg.insert_building(farm);

        let def = reg.building("farm").expect("farm def");
        let b = PlacedBuilding::with_priority("farm", 1, def.default_priority);
        assert_eq!(b.priority, 2);
        assert_eq!(b.labour_lock, None, "not locked until the player says so");
    }

    #[test]
    fn an_out_of_range_authored_priority_is_clamped_not_honoured() {
        let b = PlacedBuilding::with_priority("farm", 1, 0);
        assert_eq!(b.priority, 1, "0 clamps up into range");
        let b = PlacedBuilding::with_priority("farm", 1, 250);
        assert_eq!(b.priority, 9, "past the max clamps down");
    }
}
