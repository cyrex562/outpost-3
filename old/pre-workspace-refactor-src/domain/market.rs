use crate::domain::resource::ResourceType;
use crate::events::event::EventType;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MarketPrice {
    pub resource: ResourceType,
    pub price: f64,
}

impl MarketPrice {
    pub fn new(resource: ResourceType, price: f64) -> Self {
        Self { resource, price }
    }
}

/// Adjusts price based on demand signal, bounded to ±10% change.
/// Returns the new price and a MarketPriceChanged event.
pub fn market_price_update(
    resource: ResourceType,
    current_price: f64,
    demand_signal: f64,
) -> (f64, EventType) {
    let bounded = demand_signal.clamp(-0.1, 0.1);
    let multiplier = 1.0 + bounded;
    let new_price = (current_price * multiplier).max(0.0);
    info!(
        "Market price update: {:?} {} -> {}",
        resource, current_price, new_price
    );
    let event = EventType::MarketPriceChanged {
        resource,
        old_price: current_price,
        new_price,
    };
    (new_price, event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn price_increases_with_positive_demand() {
        let resource = ResourceType::Lithium;
        let current = 100.0;
        let (new_price, _ev) = market_price_update(resource, current, 0.05);
        assert!(new_price > current);
    }

    proptest! {
        #[test]
        fn price_change_bounded_within_ten_percent(demand in -1.0f64..1.0, base in 1.0f64..1000.0) {
            let (new_price, _event) = market_price_update(ResourceType::Steel, base, demand);
            let lower = base * 0.9 - 1e-6;
            let upper = base * 1.1 + 1e-6;
            prop_assert!(new_price >= lower && new_price <= upper);
        }
    }
}
