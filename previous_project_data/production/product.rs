use serde::{Deserialize, Serialize};
use crate::production::product_input::ProductInput;
use crate::production::product_output::ProductOutput;

#[derive(Debug,Clone, Serialize, Deserialize)]
pub struct Product {
    pub name: String,
    pub inputs: Vec<ProductInput>,
    // pub maker: // TODO: made in installation, spacecraft, settlement
    pub outputs: Vec<ProductOutput>,
}
