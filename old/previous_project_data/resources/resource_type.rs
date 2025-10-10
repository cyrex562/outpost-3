use serde::{Deserialize, Serialize};

#[derive(Debug,Clone, Serialize,Deserialize,Eq, PartialEq,Hash)]
pub enum ResourceType {
    // Extracted
    Ice,
    Minerals,
    Gases,
    Hydrocarbons,
    Organics, // trees, plants, animal life
    // Refined/Grown/Made
    Water,
    Air,
    Metal,
    NonMetal,
    Energy,
    Food,
    BioMatter,
    // Future intermediates
    Waste,
}
