use serde::{Deserialize, Serialize};

use crate::{ProbeId, StarSystem, StarSystemId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
    TimeAdvanced {
        dt: f64,
        new_time: f64,
    },
    ProbeLaunched {
        probe_id: ProbeId,
        target_system_id: StarSystemId,
        eta: f64,
    },
    ProbeArrived {
        probe_id: ProbeId,
        system_id: StarSystemId,
    },
    SystemDiscovered {
        system: StarSystem,
    },
}
