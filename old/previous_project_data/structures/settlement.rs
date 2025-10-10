use crate::buildings::building_type::BuildingType;
use crate::population::person_type::PersonType;
use crate::resources::resource_type::ResourceType;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Settlement {
    id: Uuid,
    name: String,
    population: HashMap<PersonType, u32>,
    buildings: HashMap<BuildingType, u32>,
    // production queues
    // unit production queue
    // buidling production queue
    resources: HashMap<ResourceType, u64>,
    // local market
}
