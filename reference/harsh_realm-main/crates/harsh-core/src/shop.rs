//! Runtime shop models.
//!
//! Ported from `src/harsh_realm/models/shop.py`.

use serde::{Deserialize, Serialize};

use crate::item::ItemData;

/// A single item available for purchase in a shop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShopItem {
    /// The canonical item.
    pub item_data: ItemData,
    /// Quantity in stock.
    pub quantity: i32,
    /// Sale price.
    pub price: i32,
}
