use serde::{Deserialize, Serialize};

#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq,Hash)]
pub enum PersonType {
    Colonist,
    Worker,
    Scientist,
    Soldier,
    Administrator,
    Child,
}
