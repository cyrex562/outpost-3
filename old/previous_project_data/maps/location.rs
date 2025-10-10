use uuid::Uuid;
use hex2d::Coordinate as HexCoord;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug,Serialize,Deserialize, PartialEq)]
pub enum Location {
    Surface {
        body_id: Uuid,
        hex_coord: HexCoord,
    },
    Orbit {
        body_id: Uuid,
        orbital_slot_id: Uuid,
    }, // for stations and constellations
    DeepSpace {
        x: f32,
        y: f32,
    },
    Docked {
        structure_id: Uuid,
    }, // when docked to another structure/spacecraft
}
