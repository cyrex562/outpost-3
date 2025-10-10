use serde::{Deserialize, Serialize};

use crate::StarSystemId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
#[serde(tag = "type")]
pub enum Command {
    AdvanceTime { dt: f64 },
    LaunchProbe { target_system_id: StarSystemId},
}

