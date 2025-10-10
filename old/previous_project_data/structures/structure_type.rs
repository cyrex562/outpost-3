use crate::structures::{
    installation::Installation, settlement::Settlement, spacecraft::Spacecraft,
};

#[allow(dead_code)]
pub enum StructureType {
    Settlement(Settlement),
    Installation(Installation),
    Spacecraft(Spacecraft),
}
