use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::units::unit_type::UnitType;

#[derive(Clone, Debug,Deserialize,Serialize)]
pub enum InstallationPurpose {
    Mine,
    Refinery,
    Factory,
    Military,
    Research,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Installation {
    id: Uuid,
    name: String,
    purpose: InstallationPurpose,
    crew: Option<UnitType>,
}
