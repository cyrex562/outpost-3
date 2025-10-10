use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Faction {
    id: Uuid,
    name: String,
}

