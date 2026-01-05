use outpost3::commands::{BuyResource, Command, SellResource, TradingError};
use outpost3::domain::market::MarketPrice;
use outpost3::domain::{ColonyId, ResourceType};
use outpost3::events::event::TradeSide;
use outpost3::events::EventType;

#[test]
fn buy_fails_with_insufficient_credits() {
    let cmd = BuyResource {
        colony_id: ColonyId(1),
        resource_type: ResourceType::Steel,
        quantity: 10,
        market_price: MarketPrice::new(ResourceType::Steel, 20.0),
        available_credits: 150, // Needs 200
    };

    let result = cmd.validate();
    assert_eq!(result, Err(TradingError::InsufficientCredits));
}

#[test]
fn sell_fails_with_insufficient_resource() {
    let cmd = SellResource {
        colony_id: ColonyId(1),
        resource_type: ResourceType::Lithium,
        quantity: 25,
        market_price: MarketPrice::new(ResourceType::Lithium, 50.0),
        available_quantity: 10,
    };

    let result = cmd.validate();
    assert_eq!(result, Err(TradingError::InsufficientResource));
}

#[test]
fn trade_roundtrip_emits_events() {
    let buy = BuyResource {
        colony_id: ColonyId(42),
        resource_type: ResourceType::CopperOre,
        quantity: 5,
        market_price: MarketPrice::new(ResourceType::CopperOre, 12.5),
        available_credits: 1000,
    };

    let buy_events = buy.execute().expect("buy execute");
    assert_eq!(buy_events.len(), 1);
    if let EventType::ResourceTraded {
        colony_id,
        resource_type,
        quantity,
        price,
        side,
    } = &buy_events[0]
    {
        assert_eq!(*colony_id, ColonyId(42));
        assert_eq!(*resource_type, ResourceType::CopperOre);
        assert_eq!(*quantity, 5);
        assert_eq!(*price, 12.5);
        assert_eq!(*side, TradeSide::Buy);
    } else {
        panic!("unexpected event type");
    }

    // Assume resources were added; now sell some portion
    let sell = SellResource {
        colony_id: ColonyId(42),
        resource_type: ResourceType::CopperOre,
        quantity: 3,
        market_price: MarketPrice::new(ResourceType::CopperOre, 11.0),
        available_quantity: 5,
    };

    let sell_events = sell.execute().expect("sell execute");
    assert_eq!(sell_events.len(), 1);
    if let EventType::ResourceTraded {
        colony_id,
        resource_type,
        quantity,
        price,
        side,
    } = &sell_events[0]
    {
        assert_eq!(*colony_id, ColonyId(42));
        assert_eq!(*resource_type, ResourceType::CopperOre);
        assert_eq!(*quantity, 3);
        assert_eq!(*price, 11.0);
        assert_eq!(*side, TradeSide::Sell);
    } else {
        panic!("unexpected event type");
    }
}
