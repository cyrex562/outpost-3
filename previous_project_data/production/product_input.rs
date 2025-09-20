use serde::{Deserialize, Serialize};
use crate::resources::resource_type::ResourceType;

#[derive(Debug,Clone,Deserialize,Serialize)]
pub struct ProductInput {
    pub resource_type: ResourceType,
    pub amount: f32,
}
