use serde::{Deserialize, Serialize};

use crate::domain::ColonyId;
use crate::events::EventType;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EconomySnapshot {
    pub colony_id: ColonyId,
    pub gdp: i64,
    pub income: i64,
    pub expenses: i64,
    pub net_worth: i64,
}

pub fn calculate_snapshot(
    colony_id: ColonyId,
    credits_before: i64,
    credits_after: i64,
) -> (EconomySnapshot, EventType) {
    let delta = credits_after - credits_before;
    let income = delta.max(0);
    let expenses = (-delta).max(0);
    let gdp = income - expenses;
    let net_worth = credits_after;

    let snapshot = EconomySnapshot {
        colony_id,
        gdp,
        income,
        expenses,
        net_worth,
    };

    let event = EventType::EconomySnapshot {
        colony_id,
        gdp,
        income,
        expenses,
        net_worth,
    };

    (snapshot, event)
}
