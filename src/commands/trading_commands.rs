use thiserror::Error;

use super::Command;
use crate::domain::market::MarketPrice;
use crate::domain::{ColonyId, ResourceType};
use crate::events::event::TradeSide;
use crate::events::EventType;
use tracing::info;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TradingError {
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
    #[error("Insufficient credits for purchase")]
    InsufficientCredits,
    #[error("Insufficient resource quantity for sale")]
    InsufficientResource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuyResource {
    pub colony_id: ColonyId,
    pub resource_type: ResourceType,
    pub quantity: i64,
    pub market_price: MarketPrice,
    pub available_credits: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SellResource {
    pub colony_id: ColonyId,
    pub resource_type: ResourceType,
    pub quantity: i64,
    pub market_price: MarketPrice,
    pub available_quantity: i64,
}

impl BuyResource {
    fn cost(&self) -> f64 {
        self.market_price.price * self.quantity as f64
    }
}

impl Command for BuyResource {
    type Error = TradingError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.quantity <= 0 {
            return Err(TradingError::InvalidCommand(
                "Quantity must be positive".to_string(),
            ));
        }

        if self.market_price.price <= 0.0 {
            return Err(TradingError::InvalidCommand(
                "Market price must be positive".to_string(),
            ));
        }

        if self.market_price.resource != self.resource_type {
            return Err(TradingError::InvalidCommand(
                "Price resource mismatch".to_string(),
            ));
        }

        if (self.available_credits as f64) + f64::EPSILON < self.cost() {
            return Err(TradingError::InsufficientCredits);
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;
        info!(
            colony_id = ?self.colony_id,
            resource_type = ?self.resource_type,
            quantity = self.quantity,
            price = self.market_price.price,
            side = "BUY",
            "Trade executed"
        );
        Ok(vec![EventType::ResourceTraded {
            colony_id: self.colony_id,
            resource_type: self.resource_type,
            quantity: self.quantity,
            price: self.market_price.price,
            side: TradeSide::Buy,
        }])
    }
}

impl Command for SellResource {
    type Error = TradingError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.quantity <= 0 {
            return Err(TradingError::InvalidCommand(
                "Quantity must be positive".to_string(),
            ));
        }

        if self.market_price.price <= 0.0 {
            return Err(TradingError::InvalidCommand(
                "Market price must be positive".to_string(),
            ));
        }

        if self.market_price.resource != self.resource_type {
            return Err(TradingError::InvalidCommand(
                "Price resource mismatch".to_string(),
            ));
        }

        if self.available_quantity < self.quantity {
            return Err(TradingError::InsufficientResource);
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;
        info!(
            colony_id = ?self.colony_id,
            resource_type = ?self.resource_type,
            quantity = self.quantity,
            price = self.market_price.price,
            side = "SELL",
            "Trade executed"
        );
        Ok(vec![EventType::ResourceTraded {
            colony_id: self.colony_id,
            resource_type: self.resource_type,
            quantity: self.quantity,
            price: self.market_price.price,
            side: TradeSide::Sell,
        }])
    }
}
