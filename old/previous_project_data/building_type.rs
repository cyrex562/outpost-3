use std::fmt::{Debug, Formatter};
use serde::{Deserialize, Serialize};

#[derive(Clone,Serialize,Deserialize,Eq,Hash,PartialEq)]
pub enum BuildingType {
    Mine,
    Refinery,
    Factory,
    Laboratory,
}

impl Debug for BuildingType {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
