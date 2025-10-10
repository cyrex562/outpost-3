use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StarSystemId(pub Ulid);
impl StarSystemId {
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProbeId(pub Ulid);
impl ProbeId {
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CelestialBodyId(pub Ulid);
impl CelestialBodyId {
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

pub fn generate_id() -> Ulid {
    Ulid::new()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub game_time: f64,
    pub systems: Vec<StarSystem>,
    pub probes_in_flight: Vec<ProbeInFlight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarSystem {
    pub id: StarSystemId,
    pub name: String,
    pub spectral_class: String,
    pub bodies: Vec<CelestialBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeInFlight {
    pub id: ProbeId,
    pub target_system_id: StarSystemId,
    pub launched_at: f64,
    pub arrival_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CelestialBody {
    pub id: CelestialBodyId,
    pub name: String,
    pub body_type: String,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            game_time: 0.0,
            systems: Vec::new(),
            probes_in_flight: Vec::new(),
        }
    }

    pub fn with_probe_launched(
        mut self,
        target_system_id: StarSystemId,
        arrival_time: f64,
    ) -> (Self, ProbeId) {
        let probe_id = ProbeId(Ulid::new());
        self.probes_in_flight.push(ProbeInFlight {
            id: probe_id,
            target_system_id: target_system_id,
            launched_at: self.game_time,
            arrival_time,
        });
        (self, probe_id)
    }

    pub fn with_system_discovered(mut self, system: StarSystem) -> Self {
        if !self.systems.iter().any(|s| s.id == system.id) {
            self.systems.push(system);
        }
        self
    }

    pub fn with_probes_removed(mut self, probe_ids: &[ProbeId]) -> Self {
        self.probes_in_flight
            .retain(|probe| !probe_ids.contains(&probe.id));
        self
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
