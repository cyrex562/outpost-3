use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Default,Debug,Serialize,Deserialize)]
pub struct Tile {
    pub id: Uuid,
}
