use serde::{Deserialize, Serialize};
use crate::resources::resource_type::ResourceType;

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct ProductOutput {
    resource_type: ResourceType,
    amount: f32,
}
