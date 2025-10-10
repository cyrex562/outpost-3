use serde::{Deserialize, Serialize};
use crate::production::product::Product;

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Process {
    name: String,
    time: u32,
    product: Product,
}
