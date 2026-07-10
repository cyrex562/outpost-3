//! Orbital infrastructure layer — stations, orbit types, satellite constellations.
//!
//! Implements §8.2 of `docs/DESIGN.md`.  Orbital infrastructure is represented
//! as a schematic coverage model, not a physical placement puzzle.
//!
//! # Structure
//!
//! - [`OrbitType`] — three discrete orbit bands with different tradeoffs.
//! - [`StationType`] — three specialization roles for orbital stations.
//! - [`SatelliteType`] — three coverage layers (comms / sensor / defense).
//! - [`OrbitalStation`] — a built station occupying slots in an orbit band.
//! - [`SatelliteConstellation`] — a deployed satellite array with coverage footprint.
//! - [`OrbitalRegistry`] — system-wide store for stations and constellations.
//! - [`coverage_for`] — pure function: coverage footprint for a given satellite type
//!   and orbit band.

use uuid::Uuid;

use crate::colony::ColonyId;

// ─── Orbit types ─────────────────────────────────────────────────────────────

/// The three discrete orbital bands available to stations and constellations.
///
/// Each band offers a distinct tradeoff between access cost, hazard exposure,
/// coverage footprint, and communication latency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum OrbitType {
    /// Close to the surface — cheap surface access, higher hazard exposure.
    ///
    /// Maximum slots: [`OrbitType::slot_capacity`] returns [`LOW_ORBIT_SLOTS`].
    Low,
    /// Fixed over one colony — dedicated throughput link, no coverage drift.
    ///
    /// Maximum slots: [`GEO_ORBIT_SLOTS`].
    Geostationary,
    /// Stable gravitational equilibrium — system-wide vantage, multi-body reach.
    ///
    /// Maximum slots: [`LAGRANGE_SLOTS`].
    Lagrange,
}

/// Slot capacity of the Low-orbit band.
pub const LOW_ORBIT_SLOTS: u32 = 12;
/// Slot capacity of the Geostationary-orbit band.
pub const GEO_ORBIT_SLOTS: u32 = 6;
/// Slot capacity of the Lagrange-orbit band (scarce, system-wide value).
pub const LAGRANGE_SLOTS: u32 = 3;

impl OrbitType {
    /// Maximum number of station slots available in this orbit band.
    #[must_use]
    pub fn slot_capacity(self) -> u32 {
        match self {
            OrbitType::Low => LOW_ORBIT_SLOTS,
            OrbitType::Geostationary => GEO_ORBIT_SLOTS,
            OrbitType::Lagrange => LAGRANGE_SLOTS,
        }
    }

    /// Relative hazard exposure scalar: higher means more vulnerable to debris /
    /// meteor strikes before defense satellites mitigate them.
    #[must_use]
    pub fn hazard_exposure(self) -> f32 {
        match self {
            OrbitType::Low => 1.5,
            OrbitType::Geostationary => 1.0,
            OrbitType::Lagrange => 0.6,
        }
    }

    /// Surface access cost multiplier: lower is cheaper.
    #[must_use]
    pub fn surface_access_cost(self) -> f32 {
        match self {
            OrbitType::Low => 1.0,
            OrbitType::Geostationary => 2.0,
            OrbitType::Lagrange => 4.0,
        }
    }
}

// ─── Station types ────────────────────────────────────────────────────────────

/// The three station specialization roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum StationType {
    /// Population capacity — no planetary surface needed.
    ///
    /// Also acts as a population gateway for immigration (§7).
    Habitat,
    /// Zero-g / vacuum production — strictly better for certain recipes.
    Industrial,
    /// Dock / hub connecting surface shipments to system traffic (§8.3).
    ///
    /// Acts as a port for external immigration waves.
    Logistics,
}

impl StationType {
    /// Slot cost consumed in the chosen orbit band when this station is built.
    #[must_use]
    pub fn slot_cost(self) -> u32 {
        match self {
            StationType::Industrial => 3,
            StationType::Habitat | StationType::Logistics => 2,
        }
    }

    /// Base number of colony-sol turns required to construct this station.
    #[must_use]
    pub fn base_construction_turns(self) -> u32 {
        match self {
            StationType::Habitat => 8,
            StationType::Industrial => 12,
            StationType::Logistics => 6,
        }
    }
}

// ─── Satellite types ──────────────────────────────────────────────────────────

/// The three satellite constellation roles, each with its own coverage layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum SatelliteType {
    /// Communications relay — boosts colony throughput and trade route bandwidth.
    Comms,
    /// Sensor array — early warning for hazards and raiders (if adversary toggle enabled).
    Sensor,
    /// Defense battery — reduces hazard damage; intercepts raiders if adversary toggle enabled.
    Defense,
}

// ─── Coverage ─────────────────────────────────────────────────────────────────

/// Coverage footprint produced by one satellite constellation unit.
///
/// All values are in `[0.0, 1.0]` coverage fractions relative to the body /
/// system the orbit band is anchored to.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CoverageFootprint {
    /// Fraction of the surface / body footprint covered by this constellation unit.
    pub local_coverage: f32,
    /// Fraction of the star system covered (meaningful only in Lagrange orbits).
    pub system_coverage: f32,
}

/// Compute the coverage footprint for a single constellation unit of `satellite`
/// placed in `orbit`.
///
/// Pure function — no state dependencies; safe to call in tests and UI layers.
#[must_use]
pub fn coverage_for(satellite: SatelliteType, orbit: OrbitType) -> CoverageFootprint {
    match (satellite, orbit) {
        // Comms: low orbit gives tight local patch; geo gives full single-body coverage;
        // Lagrange gives moderate system-wide reach.
        (SatelliteType::Comms, OrbitType::Low) => CoverageFootprint {
            local_coverage: 0.25,
            system_coverage: 0.0,
        },
        (SatelliteType::Comms, OrbitType::Geostationary) => CoverageFootprint {
            local_coverage: 1.0,
            system_coverage: 0.0,
        },
        (SatelliteType::Comms, OrbitType::Lagrange) => CoverageFootprint {
            local_coverage: 0.6,
            system_coverage: 0.5,
        },

        // Sensor: low orbit has fine resolution but limited arc; geo watches one body
        // continuously; Lagrange has the system-wide vantage.
        (SatelliteType::Sensor, OrbitType::Low) => CoverageFootprint {
            local_coverage: 0.2,
            system_coverage: 0.0,
        },
        (SatelliteType::Sensor, OrbitType::Geostationary) => CoverageFootprint {
            local_coverage: 0.85,
            system_coverage: 0.0,
        },
        (SatelliteType::Sensor, OrbitType::Lagrange) => CoverageFootprint {
            local_coverage: 0.5,
            system_coverage: 0.9,
        },

        // Defense: low orbit intercepts close threats best; geo defends one body;
        // Lagrange has reduced local efficacy but some system-wide deterrence.
        (SatelliteType::Defense, OrbitType::Low) => CoverageFootprint {
            local_coverage: 0.6,
            system_coverage: 0.0,
        },
        (SatelliteType::Defense, OrbitType::Geostationary) => CoverageFootprint {
            local_coverage: 0.9,
            system_coverage: 0.0,
        },
        (SatelliteType::Defense, OrbitType::Lagrange) => CoverageFootprint {
            local_coverage: 0.3,
            system_coverage: 0.4,
        },
    }
}

/// Aggregate coverage produced by `count` units of a constellation.
///
/// Coverage caps at `1.0` — redundant satellites don't exceed full coverage.
#[must_use]
pub fn aggregate_coverage(
    satellite: SatelliteType,
    orbit: OrbitType,
    count: u32,
) -> CoverageFootprint {
    let unit = coverage_for(satellite, orbit);
    #[allow(clippy::cast_precision_loss)]
    let factor = (count as f32).min(1.0 / unit.local_coverage.max(f32::EPSILON));
    CoverageFootprint {
        local_coverage: (unit.local_coverage * factor).min(1.0),
        system_coverage: (unit.system_coverage * factor).min(1.0),
    }
}

// ─── Orbital station ──────────────────────────────────────────────────────────

/// A built orbital station occupying slots in one orbit band.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrbitalStation {
    /// Stable unique identifier.
    pub id: Uuid,
    /// Station specialization role.
    pub station_type: StationType,
    /// Orbit band this station occupies.
    pub orbit_type: OrbitType,
    /// Colony that built and operates this station (may be `None` for independent
    /// Lagrange stations in future content).
    pub colony_id: ColonyId,
    /// Slots consumed in the orbit band.
    pub slot_cost: u32,
}

impl OrbitalStation {
    /// Create a new station; `slot_cost` defaults to [`StationType::slot_cost`].
    #[must_use]
    pub fn new(station_type: StationType, orbit_type: OrbitType, colony_id: ColonyId) -> Self {
        Self {
            id: Uuid::new_v4(),
            slot_cost: station_type.slot_cost(),
            station_type,
            orbit_type,
            colony_id,
        }
    }
}

// ─── Satellite constellation ──────────────────────────────────────────────────

/// A deployed satellite array providing a coverage layer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SatelliteConstellation {
    /// Stable unique identifier.
    pub id: Uuid,
    /// What kind of coverage this constellation provides.
    pub satellite_type: SatelliteType,
    /// Orbit band the constellation is deployed in.
    pub orbit_type: OrbitType,
    /// Number of individual satellites in the array.
    pub count: u32,
    /// Whether this overlay is toggled on in the UI (no sim effect — presentation only).
    pub overlay_visible: bool,
}

impl SatelliteConstellation {
    /// Create a new constellation with the overlay visible by default.
    #[must_use]
    pub fn new(satellite_type: SatelliteType, orbit_type: OrbitType, count: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            satellite_type,
            orbit_type,
            count,
            overlay_visible: true,
        }
    }

    /// Compute the aggregate coverage footprint for this constellation.
    #[must_use]
    pub fn coverage(&self) -> CoverageFootprint {
        aggregate_coverage(self.satellite_type, self.orbit_type, self.count)
    }
}

// ─── Orbital registry ─────────────────────────────────────────────────────────

/// Error produced when an orbital operation cannot be completed.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrbitalError {
    /// The target orbit band has no remaining slot capacity.
    #[error("orbit slot capacity exceeded: need {needed} slot(s) but only {available} available in {orbit:?}")]
    SlotCapacityExceeded {
        /// Slots required by the new station.
        needed: u32,
        /// Slots remaining in the orbit band.
        available: u32,
        /// Orbit band that is full.
        orbit: OrbitType,
    },
    /// The referenced station does not exist.
    #[error("orbital station not found: {0}")]
    StationNotFound(Uuid),
    /// The referenced constellation does not exist.
    #[error("satellite constellation not found: {0}")]
    ConstellationNotFound(Uuid),
}

/// System-wide registry of orbital stations and satellite constellations.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OrbitalRegistry {
    /// All built stations across all orbit bands.
    pub stations: Vec<OrbitalStation>,
    /// All deployed satellite constellations.
    pub constellations: Vec<SatelliteConstellation>,
}

impl OrbitalRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Count slots in use in the given orbit band.
    #[must_use]
    pub fn slots_used(&self, orbit: OrbitType) -> u32 {
        self.stations
            .iter()
            .filter(|s| s.orbit_type == orbit)
            .map(|s| s.slot_cost)
            .sum()
    }

    /// Count slots remaining in the given orbit band.
    #[must_use]
    pub fn slots_available(&self, orbit: OrbitType) -> u32 {
        orbit.slot_capacity().saturating_sub(self.slots_used(orbit))
    }

    /// Attempt to add a station; fails if the orbit band is full.
    ///
    /// # Errors
    ///
    /// Returns [`OrbitalError::SlotCapacityExceeded`] when there are not enough
    /// free slots in `station.orbit_type`.
    pub fn add_station(&mut self, station: OrbitalStation) -> Result<(), OrbitalError> {
        let available = self.slots_available(station.orbit_type);
        if station.slot_cost > available {
            return Err(OrbitalError::SlotCapacityExceeded {
                needed: station.slot_cost,
                available,
                orbit: station.orbit_type,
            });
        }
        self.stations.push(station);
        Ok(())
    }

    /// Remove a station by id.
    ///
    /// # Errors
    ///
    /// Returns [`OrbitalError::StationNotFound`] if the id is unknown.
    pub fn remove_station(&mut self, id: Uuid) -> Result<OrbitalStation, OrbitalError> {
        let pos = self
            .stations
            .iter()
            .position(|s| s.id == id)
            .ok_or(OrbitalError::StationNotFound(id))?;
        Ok(self.stations.remove(pos))
    }

    /// Deploy a satellite constellation; no slot gating (constellations don't
    /// occupy station slots — they are separate infrastructure).
    pub fn deploy_constellation(&mut self, constellation: SatelliteConstellation) {
        self.constellations.push(constellation);
    }

    /// Remove a constellation by id.
    ///
    /// # Errors
    ///
    /// Returns [`OrbitalError::ConstellationNotFound`] if the id is unknown.
    pub fn remove_constellation(
        &mut self,
        id: Uuid,
    ) -> Result<SatelliteConstellation, OrbitalError> {
        let pos = self
            .constellations
            .iter()
            .position(|c| c.id == id)
            .ok_or(OrbitalError::ConstellationNotFound(id))?;
        Ok(self.constellations.remove(pos))
    }

    /// Toggle the overlay visibility of a constellation.
    ///
    /// # Errors
    ///
    /// Returns [`OrbitalError::ConstellationNotFound`] if the id is unknown.
    pub fn toggle_overlay(&mut self, id: Uuid) -> Result<bool, OrbitalError> {
        let c = self
            .constellations
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(OrbitalError::ConstellationNotFound(id))?;
        c.overlay_visible = !c.overlay_visible;
        Ok(c.overlay_visible)
    }

    /// Return the combined coverage for a given satellite type across all
    /// constellations of that type currently deployed.
    ///
    /// Sums unit coverage across all constellations in all orbit bands,
    /// then clamps to `[0, 1]`.
    #[must_use]
    pub fn combined_coverage(&self, satellite: SatelliteType) -> CoverageFootprint {
        let mut local = 0.0f32;
        let mut system = 0.0f32;

        for c in &self.constellations {
            if c.satellite_type == satellite {
                let fp = c.coverage();
                local += fp.local_coverage;
                system += fp.system_coverage;
            }
        }

        CoverageFootprint {
            local_coverage: local.min(1.0),
            system_coverage: system.min(1.0),
        }
    }
}

// ─── Construction queue ───────────────────────────────────────────────────────

/// An in-progress orbital station construction project.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrbitalConstructionProject {
    /// Content-pack blueprint id that was used to start this project.
    pub blueprint_id: String,
    /// Colony funding and operating this build.
    pub colony_id: ColonyId,
    /// Orbit band the finished station will occupy.
    pub orbit_type: OrbitType,
    /// Station specialisation that will be built.
    pub station_type: StationType,
    /// Strategic months remaining until completion.
    pub months_remaining: u32,
    /// Whether commodity costs have already been deducted from the colony pool.
    pub costs_paid: bool,
}

impl OrbitalConstructionProject {
    /// Create a new project.  `costs_paid` starts as `false`; the engine sets it
    /// to `true` after successfully deducting commodity costs.
    #[must_use]
    pub fn new(
        blueprint_id: String,
        colony_id: ColonyId,
        orbit_type: OrbitType,
        station_type: StationType,
        build_months: u32,
    ) -> Self {
        Self {
            blueprint_id,
            colony_id,
            orbit_type,
            station_type,
            months_remaining: build_months,
            costs_paid: false,
        }
    }

    /// Decrement the countdown by one strategic month.
    ///
    /// Returns `true` when the station is ready to be placed (months hit zero).
    pub fn tick(&mut self) -> bool {
        self.months_remaining = self.months_remaining.saturating_sub(1);
        self.months_remaining == 0
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── slot capacity constants ───────────────────────────────────────────────

    #[test]
    fn orbit_slot_capacities_match_design() {
        assert_eq!(OrbitType::Low.slot_capacity(), LOW_ORBIT_SLOTS);
        assert_eq!(OrbitType::Geostationary.slot_capacity(), GEO_ORBIT_SLOTS);
        assert_eq!(OrbitType::Lagrange.slot_capacity(), LAGRANGE_SLOTS);
        // Low has the most slots; Lagrange the fewest.
        assert!(OrbitType::Low.slot_capacity() > OrbitType::Lagrange.slot_capacity());
    }

    // ── slot gating ──────────────────────────────────────────────────────────

    #[test]
    fn station_blocked_when_orbit_slots_full() {
        let mut reg = OrbitalRegistry::new();
        let colony = ColonyId::new_v4();

        // Fill all Lagrange slots (capacity = 3; Logistics = 2 slots each).
        // First station (2 slots) fits.
        reg.add_station(OrbitalStation::new(
            StationType::Logistics,
            OrbitType::Lagrange,
            colony,
        ))
        .expect("first station should fit");
        assert_eq!(reg.slots_used(OrbitType::Lagrange), 2);

        // Second station (2 slots) — only 1 slot left → must fail.
        let err = reg
            .add_station(OrbitalStation::new(
                StationType::Logistics,
                OrbitType::Lagrange,
                colony,
            ))
            .unwrap_err();

        assert!(
            matches!(
                err,
                OrbitalError::SlotCapacityExceeded {
                    orbit: OrbitType::Lagrange,
                    ..
                }
            ),
            "expected SlotCapacityExceeded for Lagrange, got {err:?}"
        );
    }

    #[test]
    fn low_orbit_accepts_more_stations_than_lagrange() {
        let mut reg = OrbitalRegistry::new();
        let colony = ColonyId::new_v4();

        // Fill Lagrange fully with one Habitat (2 slots) — 1 slot left.
        reg.add_station(OrbitalStation::new(
            StationType::Habitat,
            OrbitType::Lagrange,
            colony,
        ))
        .unwrap();
        // Now only 1 slot left; Habitat costs 2 → blocked.
        assert!(reg
            .add_station(OrbitalStation::new(
                StationType::Habitat,
                OrbitType::Lagrange,
                colony,
            ))
            .is_err());

        // Low orbit can still accept many more.
        for _ in 0..4 {
            reg.add_station(OrbitalStation::new(
                StationType::Habitat,
                OrbitType::Low,
                colony,
            ))
            .expect("low orbit should have plenty of slots");
        }
        assert!(reg.slots_available(OrbitType::Low) > 0);
    }

    #[test]
    fn removing_station_frees_slots() {
        let mut reg = OrbitalRegistry::new();
        let colony = ColonyId::new_v4();

        let station = OrbitalStation::new(StationType::Logistics, OrbitType::Lagrange, colony);
        let id = station.id;
        reg.add_station(station).unwrap();
        assert_eq!(reg.slots_used(OrbitType::Lagrange), 2);

        reg.remove_station(id).unwrap();
        assert_eq!(reg.slots_used(OrbitType::Lagrange), 0);
        assert_eq!(reg.slots_available(OrbitType::Lagrange), LAGRANGE_SLOTS);
    }

    // ── coverage computation ─────────────────────────────────────────────────

    #[test]
    fn lagrange_sensor_has_high_system_coverage() {
        let fp = coverage_for(SatelliteType::Sensor, OrbitType::Lagrange);
        assert!(
            fp.system_coverage > 0.5,
            "Lagrange sensor should cover >50% of system; got {}",
            fp.system_coverage
        );
    }

    #[test]
    fn low_orbit_sensor_has_zero_system_coverage() {
        let fp = coverage_for(SatelliteType::Sensor, OrbitType::Low);
        assert_eq!(
            fp.system_coverage, 0.0,
            "Low-orbit sensor must not have system coverage"
        );
    }

    #[test]
    fn geo_comms_achieves_full_local_coverage() {
        let fp = coverage_for(SatelliteType::Comms, OrbitType::Geostationary);
        assert!(
            (fp.local_coverage - 1.0).abs() < 1e-6,
            "Geostationary comms should cover 100% of local body; got {}",
            fp.local_coverage
        );
    }

    #[test]
    fn aggregate_coverage_caps_at_one() {
        // 100 Low-orbit defense units — should saturate local_coverage at 1.0.
        let fp = aggregate_coverage(SatelliteType::Defense, OrbitType::Low, 100);
        assert!(
            (fp.local_coverage - 1.0).abs() < 1e-6,
            "aggregate coverage must cap at 1.0; got {}",
            fp.local_coverage
        );
    }

    #[test]
    fn combined_coverage_sums_across_constellations() {
        let mut reg = OrbitalRegistry::new();

        // Two small Comms constellations in different orbit bands.
        reg.deploy_constellation(SatelliteConstellation::new(
            SatelliteType::Comms,
            OrbitType::Low,
            2,
        ));
        reg.deploy_constellation(SatelliteConstellation::new(
            SatelliteType::Comms,
            OrbitType::Lagrange,
            1,
        ));

        let combined = reg.combined_coverage(SatelliteType::Comms);
        // Lagrange comms gives system_coverage; combined should be > 0.
        assert!(
            combined.system_coverage > 0.0,
            "combined comms coverage should include Lagrange system_coverage"
        );
        assert!(
            combined.local_coverage > 0.0,
            "combined comms coverage should include Low-orbit local_coverage"
        );
    }

    // ── overlay toggle ────────────────────────────────────────────────────────

    #[test]
    fn overlay_toggle_flips_visibility() {
        let mut reg = OrbitalRegistry::new();
        let c = SatelliteConstellation::new(SatelliteType::Defense, OrbitType::Low, 3);
        let id = c.id;
        reg.deploy_constellation(c);

        assert!(reg.constellations[0].overlay_visible);
        reg.toggle_overlay(id).unwrap();
        assert!(!reg.constellations[0].overlay_visible);
        reg.toggle_overlay(id).unwrap();
        assert!(reg.constellations[0].overlay_visible);
    }

    // ── orbit-type tradeoffs ─────────────────────────────────────────────────

    #[test]
    fn low_orbit_cheapest_surface_access() {
        assert!(
            OrbitType::Low.surface_access_cost() < OrbitType::Geostationary.surface_access_cost()
        );
        assert!(
            OrbitType::Geostationary.surface_access_cost()
                < OrbitType::Lagrange.surface_access_cost()
        );
    }

    #[test]
    fn low_orbit_highest_hazard_exposure() {
        assert!(OrbitType::Low.hazard_exposure() > OrbitType::Geostationary.hazard_exposure());
        assert!(OrbitType::Lagrange.hazard_exposure() < OrbitType::Geostationary.hazard_exposure());
    }

    // ── station type properties ───────────────────────────────────────────────

    #[test]
    fn industrial_station_costs_more_slots_than_logistics() {
        assert!(StationType::Industrial.slot_cost() > StationType::Logistics.slot_cost());
    }

    #[test]
    fn station_types_have_positive_construction_turns() {
        for st in [
            StationType::Habitat,
            StationType::Industrial,
            StationType::Logistics,
        ] {
            assert!(st.base_construction_turns() > 0);
        }
    }

    // ── serde round-trip ──────────────────────────────────────────────────────

    #[test]
    fn orbit_type_serde_round_trip() {
        for orbit in [
            OrbitType::Low,
            OrbitType::Geostationary,
            OrbitType::Lagrange,
        ] {
            let json = serde_json::to_string(&orbit).unwrap();
            let back: OrbitType = serde_json::from_str(&json).unwrap();
            assert_eq!(orbit, back);
        }
    }

    #[test]
    fn orbital_registry_serde_round_trip() {
        let mut reg = OrbitalRegistry::new();
        let colony = ColonyId::new_v4();
        reg.add_station(OrbitalStation::new(
            StationType::Habitat,
            OrbitType::Low,
            colony,
        ))
        .unwrap();
        reg.deploy_constellation(SatelliteConstellation::new(
            SatelliteType::Comms,
            OrbitType::Geostationary,
            4,
        ));

        let json = serde_json::to_string(&reg).unwrap();
        let back: OrbitalRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.stations.len(), 1);
        assert_eq!(back.constellations.len(), 1);
    }

    // ── OrbitalConstructionProject ────────────────────────────────────────────

    #[test]
    fn construction_project_tick_counts_down() {
        let colony = ColonyId::new_v4();
        let mut project = OrbitalConstructionProject::new(
            "habitat_bp".into(),
            colony,
            OrbitType::Low,
            StationType::Habitat,
            3,
        );
        assert_eq!(project.months_remaining, 3);
        assert!(!project.tick()); // month 2 remaining
        assert!(!project.tick()); // month 1 remaining
        assert!(project.tick());  // completes at 0
        assert_eq!(project.months_remaining, 0);
    }

    #[test]
    fn construction_project_tick_returns_true_only_at_zero() {
        let colony = ColonyId::new_v4();
        let mut project = OrbitalConstructionProject::new(
            "bp".into(),
            colony,
            OrbitType::Geostationary,
            StationType::Logistics,
            1,
        );
        // Single month to build.
        assert!(project.tick());
    }
}
