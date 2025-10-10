use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::maps::location::Location;
use crate::population::person_type::PersonType;
use crate::resources::resource_type::ResourceType;
use crate::units::unit_type::UnitType;
use uuid::Uuid;

#[derive(Debug,Clone,Serialize,Deserialize,Hash,PartialEq,Eq)]
pub enum SpacecraftModuleType {
    Mine,
    Refinery,
    Factory,
    Laboratory,
    Military,
    Administrative,

}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Spacecraft {
    id: Uuid,
    location: Location,
    destination: Option<Location>,
    modules: HashMap<SpacecraftModuleType, u32>,
    cargo: HashMap<ResourceType, u32>,
    population: Option<HashMap<PersonType, u32>>,
    crew: Option<UnitType>,
    fleed_it: Option<Uuid>,
}