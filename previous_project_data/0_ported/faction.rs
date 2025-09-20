use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Faction {
    pub id: Uuid,
    pub name: String,
}

impl Faction {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
        }
    }
}

// Marker component for faction-controlled entities
#[derive(Component)]
pub struct FactionControlled {
    pub faction_id: Uuid,
}
