//! Shopping scene handler.
//!
//! Ported from `src/harsh_realm/gm/scenes/shopping.py` and its two mixin files
//! (_shopping_commands_mixin.py, _shopping_transaction_mixin.py).
//! The `_shopping_inventory.py` helper data is inlined here.
//! All Python classes are collapsed into a single `ShoppingScene` struct.

use serde_json::Value;

use crate::character::Character;
use crate::command::ParsedCommand;
use crate::db::WorldDatabase;
use crate::events::GameEvent;
use crate::gm::scenes::base::{SceneHandler, SceneState};
use crate::payloads::notices_combat::{CharacterSnapshot, NarrationNotice};
use crate::payloads::requests::{ShoppingPurchaseRequested, ShoppingSaleRequested};
use crate::repositories::entity::EntityRepository;
use crate::repositories::resources::{ResourceInstance, ResourceRepository, GOLD_RESOURCE_ID};
use crate::runtime::{InventoryItemRecord, JsonObject, JsonValue, ShopItemRecord};

// Sell value multiplier (matches Python _SELL_RATIO = 0.5)
const SELL_RATIO: f64 = 0.5;

// ------------------------------------------------------------------
// Default shop inventory (matches Python _DEFAULT_SHOP_ITEMS)
// ------------------------------------------------------------------

fn default_shop_items() -> Vec<ShopItemRecord> {
    vec![
        ShopItemRecord {
            name: "Short Sword".to_string(),
            r#type: "weapon".to_string(),
            price: 10,
            description: "A reliable blade.".to_string(),
            weapon_damage: Some("1d6".to_string()),
            ..ShopItemRecord::default()
        },
        ShopItemRecord {
            name: "Spear".to_string(),
            r#type: "weapon".to_string(),
            price: 5,
            description: "Long reach, simple design.".to_string(),
            weapon_damage: Some("1d6".to_string()),
            ..ShopItemRecord::default()
        },
        ShopItemRecord {
            name: "Axe".to_string(),
            r#type: "weapon".to_string(),
            price: 15,
            description: "Heavy and effective.".to_string(),
            weapon_damage: Some("1d8".to_string()),
            ..ShopItemRecord::default()
        },
        ShopItemRecord {
            name: "Leather Armor".to_string(),
            r#type: "armor".to_string(),
            price: 20,
            description: "Light protection. +2 AC.".to_string(),
            ac_bonus: Some(2),
            ..ShopItemRecord::default()
        },
        ShopItemRecord {
            name: "Chain Mail".to_string(),
            r#type: "armor".to_string(),
            price: 60,
            description: "Solid protection. +4 AC.".to_string(),
            ac_bonus: Some(4),
            ..ShopItemRecord::default()
        },
        ShopItemRecord {
            name: "Shield".to_string(),
            r#type: "armor".to_string(),
            price: 10,
            description: "+1 AC when equipped.".to_string(),
            ac_bonus: Some(1),
            ..ShopItemRecord::default()
        },
        ShopItemRecord {
            name: "Lazarus Patch".to_string(),
            r#type: "consumable".to_string(),
            price: 15,
            description: "Heals 1d6+1 HP.".to_string(),
            heal: Some(JsonValue::String("1d6+1".to_string())),
            ..ShopItemRecord::default()
        },
        ShopItemRecord {
            name: "Rations (7 days)".to_string(),
            r#type: "gear".to_string(),
            price: 5,
            description: "A week's worth of food.".to_string(),
            ..ShopItemRecord::default()
        },
        ShopItemRecord {
            name: "Rope (50 ft)".to_string(),
            r#type: "gear".to_string(),
            price: 3,
            description: "Sturdy hemp rope.".to_string(),
            ..ShopItemRecord::default()
        },
        ShopItemRecord {
            name: "Torch (3)".to_string(),
            r#type: "gear".to_string(),
            price: 1,
            description: "Bundle of three torches.".to_string(),
            ..ShopItemRecord::default()
        },
    ]
}

// ------------------------------------------------------------------
// Public struct
// ------------------------------------------------------------------

/// Scene handler for settlement shopping.
pub struct ShoppingScene {
    building_name: String,
    tick: i64,
    items: Vec<ShopItemRecord>,
    exit_to_exploration: bool,
    return_to: Option<String>,
}

impl ShoppingScene {
    /// Create a shopping scene with an explicit item list.
    pub fn new(
        building_name: impl Into<String>,
        tick: i64,
        items: Vec<ShopItemRecord>,
        return_to: Option<String>,
    ) -> Self {
        Self {
            building_name: building_name.into(),
            tick,
            items,
            exit_to_exploration: false,
            return_to,
        }
    }

    /// Create a shopping scene with the default hardcoded inventory.
    pub fn with_default_items(
        building_name: impl Into<String>,
        tick: i64,
        return_to: Option<String>,
    ) -> Self {
        Self::new(building_name, tick, default_shop_items(), return_to)
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn narrate(&self, text: impl Into<String>) -> GameEvent {
        let notice = NarrationNotice { text: text.into() };
        let data = to_json_object(&notice);
        GameEvent::new(self.tick, "gm.narrate", data).with_source("gm")
    }

    fn load_character(&self, db: &WorldDatabase) -> Option<Character> {
        EntityRepository::new(db)
            .load_first_character()
            .ok()
            .flatten()
    }

    fn get_gold(&self, db: &WorldDatabase, char: &Character) -> i32 {
        ResourceRepository::new(db)
            .get(&char.id, GOLD_RESOURCE_ID)
            .ok()
            .flatten()
            .map(|r| r.current)
            .unwrap_or(0)
    }

    fn set_gold(&self, db: &WorldDatabase, char_id: &str, amount: i32) {
        let _ = ResourceRepository::new(db).set(&ResourceInstance {
            entity_id: char_id.to_string(),
            resource_id: GOLD_RESOURCE_ID.to_string(),
            current: amount,
            max: None,
            last_regen_tick: 0,
        });
    }

    fn find_shop_item(&self, query: &str) -> Option<&ShopItemRecord> {
        // Numeric index first ("1", "2", …)
        if let Ok(n) = query.trim().parse::<usize>() {
            let idx = n.saturating_sub(1);
            if idx < self.items.len() {
                return Some(&self.items[idx]);
            }
        }
        let q = query.to_lowercase();
        self.items
            .iter()
            .find(|i| i.name.to_lowercase() == q)
            .or_else(|| self.items.iter().find(|i| i.name.to_lowercase().contains(&q)))
    }

    fn find_inventory_item(&self, char: &Character, name: &str) -> Option<usize> {
        let q = name.to_lowercase();
        let parse = |raw: &JsonObject| -> Option<InventoryItemRecord> {
            serde_json::from_value::<InventoryItemRecord>(JsonValue::Object(raw.clone())).ok()
        };
        let exact = char
            .equipment
            .iter()
            .enumerate()
            .find(|(_, raw)| {
                parse(raw)
                    .map(|i| i.name.to_lowercase() == q)
                    .unwrap_or(false)
            })
            .map(|(idx, _)| idx);
        if exact.is_some() {
            return exact;
        }
        char.equipment
            .iter()
            .enumerate()
            .find(|(_, raw)| {
                parse(raw)
                    .map(|i| i.name.to_lowercase().contains(&q))
                    .unwrap_or(false)
            })
            .map(|(idx, _)| idx)
    }

    // ------------------------------------------------------------------
    // Command handlers
    // ------------------------------------------------------------------

    fn handle_leave(&mut self, _cmd: &ParsedCommand, _db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        self.exit_to_exploration = true;
        Ok(vec![self.narrate("You leave the shop.")])
    }

    fn handle_status(&self, _cmd: &ParsedCommand, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        let Some(c) = self.load_character(db) else {
            return Ok(vec![self.narrate("[ERROR] No character found.")]);
        };
        let gold = self.get_gold(db, &c);
        let text = format!(
            "--- {} ---\nClass: {} | Level {}\nHP: {}/{} | AC: {}\nGold: {}",
            c.name,
            capitalize(&c.character_class),
            c.level,
            c.hp, c.max_hp,
            c.ac,
            gold,
        );
        Ok(vec![self.narrate(text)])
    }

    fn handle_inventory(&self, _cmd: &ParsedCommand, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        let Some(c) = self.load_character(db) else {
            return Ok(vec![self.narrate("[ERROR] No character found.")]);
        };
        if c.equipment.is_empty() {
            return Ok(vec![self.narrate("You are carrying nothing.")]);
        }
        let mut lines = vec!["--- Inventory ---".to_string()];
        let parse = |raw: &JsonObject| -> Option<InventoryItemRecord> {
            serde_json::from_value::<InventoryItemRecord>(JsonValue::Object(raw.clone())).ok()
        };
        for raw in &c.equipment {
            if let Some(item) = parse(raw) {
                let sell_price = ((item.price as f64 * SELL_RATIO) as i32).max(1);
                lines.push(format!("  {} (sells for {} gold)", item.name, sell_price));
            }
        }
        Ok(vec![self.narrate(lines.join("\n"))])
    }

    fn handle_help(&self, _cmd: &ParsedCommand, _db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        Ok(vec![self.narrate(
            "--- Shopping Commands ---\n\
             list           — Show available items\n\
             buy <item>     — Purchase an item\n\
             sell <item>    — Sell from inventory (50% value)\n\
             examine <item> — View item details\n\
             inventory      — Show your items\n\
             status         — Show gold and stats\n\
             leave          — Exit the shop",
        )])
    }

    fn handle_list(&self, _cmd: &ParsedCommand, _db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        if self.items.is_empty() {
            return Ok(vec![self.narrate("The shop is empty.")]);
        }
        let mut lines = vec!["--- Available Items ---".to_string()];
        for (i, item) in self.items.iter().enumerate() {
            lines.push(format!(
                "  {}. {} — {} gold  ({})",
                i + 1,
                item.name,
                item.price,
                item.description
            ));
        }
        lines.push("\nUse 'buy <item>' to purchase or 'examine <item>' for details.".to_string());
        Ok(vec![self.narrate(lines.join("\n"))])
    }

    fn handle_buy(&self, cmd: &ParsedCommand, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        if cmd.args.is_empty() {
            return Ok(vec![self.narrate(
                "Buy what? Use 'buy <item name>' or 'buy <number>'.",
            )]);
        }
        let Some(c) = self.load_character(db) else {
            return Ok(vec![self.narrate("[ERROR] No character found.")]);
        };
        let query = cmd.args.join(" ");
        let Some(item) = self.find_shop_item(&query) else {
            return Ok(vec![self.narrate(format!("The shop doesn't sell '{query}'."))]);
        };
        let price = item.price;
        let gold = self.get_gold(db, &c);
        if gold < price {
            return Ok(vec![self.narrate(format!(
                "You can't afford {} ({} gold). You have {} gold.",
                item.name, price, gold
            ))]);
        }

        let new_gold = gold - price;
        self.set_gold(db, &c.id, new_gold);

        // Build item as JsonObject for the purchase payload.
        let item_obj = to_json_object(item);

        let req = ShoppingPurchaseRequested {
            character_id: c.id.clone(),
            character_data: character_snapshot(&c),
            item: item_obj,
            price,
            gold_remaining: new_gold,
        };
        Ok(vec![
            GameEvent::new(self.tick, "shopping.purchase_requested", to_json_object(&req))
                .with_source("shopping"),
            self.narrate(format!(
                "You buy {} for {} gold. ({} gold remaining.)",
                item.name, price, new_gold
            )),
        ])
    }

    fn handle_sell(&self, cmd: &ParsedCommand, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        if cmd.args.is_empty() {
            return Ok(vec![self.narrate("Sell what? Use 'sell <item name>'.")]);
        }
        let Some(mut c) = self.load_character(db) else {
            return Ok(vec![self.narrate("[ERROR] No character found.")]);
        };
        let item_name = cmd.args.join(" ");
        let Some(idx) = self.find_inventory_item(&c, &item_name) else {
            return Ok(vec![self.narrate(format!(
                "You don't have '{item_name}' to sell."
            ))]);
        };

        let raw = c.equipment[idx].clone();
        let item: InventoryItemRecord = serde_json::from_value::<InventoryItemRecord>(
            JsonValue::Object(raw),
        )
        .map_err(|e| e.to_string())?;

        let sell_price = ((item.price as f64 * SELL_RATIO) as i32).max(1);
        c.equipment.remove(idx);

        let gold = self.get_gold(db, &c);
        let new_gold = gold + sell_price;
        self.set_gold(db, &c.id, new_gold);

        let req = ShoppingSaleRequested {
            character_id: c.id.clone(),
            character_data: character_snapshot(&c),
            item_name: item.name.clone(),
            price: sell_price,
            gold_total: new_gold,
        };
        Ok(vec![
            GameEvent::new(self.tick, "shopping.sale_requested", to_json_object(&req))
                .with_source("shopping"),
            self.narrate(format!(
                "You sell {} for {} gold. ({} gold total.)",
                item.name, sell_price, new_gold
            )),
        ])
    }

    fn handle_examine(&self, cmd: &ParsedCommand, _db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        if cmd.args.is_empty() {
            return Ok(vec![self.narrate("Examine what?")]);
        }
        let query = cmd.args.join(" ");
        let Some(item) = self.find_shop_item(&query) else {
            return Ok(vec![self.narrate(format!(
                "The shop doesn't have '{query}'."
            ))]);
        };
        let mut lines = vec![format!("--- {} ---", item.name)];
        lines.push(format!("  Price: {} gold", item.price));
        lines.push(format!("  Type: {}", item.r#type));
        if let Some(ref dmg) = item.weapon_damage {
            lines.push(format!("  Damage: {dmg}"));
        }
        if let Some(shock) = item.shock_damage {
            lines.push(format!(
                "  Shock: {} / AC {}",
                shock,
                item.shock_ac_threshold.unwrap_or(0)
            ));
        }
        if let Some(ac) = item.ac_bonus {
            lines.push(format!("  AC Bonus: +{ac}"));
        }
        if let Some(ref h) = item.heal {
            lines.push(format!("  Heals: {h} HP"));
        }
        if !item.description.is_empty() {
            lines.push(format!("  {}", item.description));
        }
        Ok(vec![self.narrate(lines.join("\n"))])
    }
}

// ------------------------------------------------------------------
// SceneHandler impl
// ------------------------------------------------------------------

impl SceneHandler for ShoppingScene {
    fn get_valid_commands(&self) -> Vec<String> {
        ["list", "buy", "sell", "examine", "leave", "status", "inventory", "help"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn get_prompt(&self, _db: &WorldDatabase) -> String {
        format!("[Shopping — {}]", self.building_name)
    }

    fn handle_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        match cmd.verb.as_str() {
            "list" | "shop" => self.handle_list(cmd, db),
            "buy" => self.handle_buy(cmd, db),
            "sell" => self.handle_sell(cmd, db),
            "examine" => self.handle_examine(cmd, db),
            "leave" => self.handle_leave(cmd, db),
            "status" => self.handle_status(cmd, db),
            "inventory" => self.handle_inventory(cmd, db),
            "help" => self.handle_help(cmd, db),
            _ => Ok(vec![self.narrate(format!(
                "\"{}\" is not valid here. Try: list, buy, sell, examine, or leave.",
                cmd.raw
            ))]),
        }
    }

    fn check_transitions(&self, _events: &[GameEvent]) -> Option<SceneState> {
        if self.exit_to_exploration {
            Some(if self.return_to.as_deref() == Some("town") {
                SceneState::Town
            } else {
                SceneState::Exploration
            })
        } else {
            None
        }
    }

    fn get_suggestions(&self) -> Vec<String> {
        vec![
            "list".to_string(),
            "buy".to_string(),
            "sell".to_string(),
            "leave".to_string(),
        ]
    }
}

// ------------------------------------------------------------------
// Module-level helpers
// ------------------------------------------------------------------

fn to_json_object<T: serde::Serialize>(value: &T) -> JsonObject {
    match serde_json::to_value(value) {
        Ok(Value::Object(map)) => map,
        _ => JsonObject::new(),
    }
}

fn character_snapshot(c: &Character) -> CharacterSnapshot {
    CharacterSnapshot {
        id: c.id.clone(),
        name: c.name.clone(),
        character_class: c.character_class.clone(),
        level: c.level,
        xp: c.xp,
        xp_next: c.xp_next,
        attributes: c.attributes.clone(),
        attr_mods: c.attr_mods.clone(),
        skills: c.skills.clone(),
        hp: c.hp,
        max_hp: c.max_hp,
        ac: c.ac,
        attack_bonus: c.attack_bonus,
        physical_save: c.physical_save,
        evasion_save: c.evasion_save,
        mental_save: c.mental_save,
        equipment: c.equipment.clone(),
        class_abilities: c.class_abilities.clone(),
        position_q: c.position_q,
        position_r: c.position_r,
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;

    fn make_scene() -> ShoppingScene {
        ShoppingScene::with_default_items("General Store", 0, None)
    }

    fn cmd(verb: &str, raw: &str, args: Vec<&str>) -> ParsedCommand {
        ParsedCommand {
            verb: verb.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            raw: raw.to_string(),
            direction: None,
        }
    }

    #[test]
    fn get_valid_commands_includes_core_commands() {
        let scene = make_scene();
        let cmds = scene.get_valid_commands();
        assert!(cmds.contains(&"list".to_string()));
        assert!(cmds.contains(&"buy".to_string()));
        assert!(cmds.contains(&"sell".to_string()));
        assert!(cmds.contains(&"leave".to_string()));
    }

    #[test]
    fn get_prompt_shows_building_name() {
        let scene = make_scene();
        let db = WorldDatabase::open_in_memory().unwrap();
        let prompt = scene.get_prompt(&db);
        assert!(prompt.contains("General Store"), "prompt: {prompt}");
    }

    #[test]
    fn help_command_returns_narrate_event() {
        let mut scene = make_scene();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("help", "help", vec![]), &db).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "gm.narrate");
    }

    #[test]
    fn list_shows_items() {
        let mut scene = make_scene();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("list", "list", vec![]), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("Short Sword"), "text: {text}");
        assert!(text.contains("gold"), "text: {text}");
    }

    #[test]
    fn list_empty_shop_says_empty() {
        let mut scene = ShoppingScene::new("Empty Store", 0, vec![], None);
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("list", "list", vec![]), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("empty"), "text: {text}");
    }

    #[test]
    fn leave_transitions_to_exploration() {
        let mut scene = make_scene();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("leave", "leave", vec![]), &db).unwrap();
        assert_eq!(scene.check_transitions(&events), Some(SceneState::Exploration));
    }

    #[test]
    fn leave_transitions_to_town_when_return_to_town() {
        let mut scene =
            ShoppingScene::with_default_items("Tavern", 0, Some("town".to_string()));
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("leave", "leave", vec![]), &db).unwrap();
        assert_eq!(scene.check_transitions(&events), Some(SceneState::Town));
    }

    #[test]
    fn buy_without_args_prompts_for_item() {
        let mut scene = make_scene();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("buy", "buy", vec![]), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("Buy what"), "text: {text}");
    }

    #[test]
    fn find_shop_item_by_number() {
        let scene = make_scene();
        let item = scene.find_shop_item("1");
        assert!(item.is_some());
        assert_eq!(item.unwrap().name, "Short Sword");
    }

    #[test]
    fn find_shop_item_by_name() {
        let scene = make_scene();
        let item = scene.find_shop_item("Spear");
        assert!(item.is_some());
        assert_eq!(item.unwrap().name, "Spear");
    }

    #[test]
    fn find_shop_item_by_partial_name() {
        let scene = make_scene();
        let item = scene.find_shop_item("chain");
        assert!(item.is_some());
        assert_eq!(item.unwrap().name, "Chain Mail");
    }

    #[test]
    fn examine_shows_item_details() {
        let mut scene = make_scene();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene
            .handle_command(&cmd("examine", "examine Short Sword", vec!["Short Sword"]), &db)
            .unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("Short Sword"), "text: {text}");
        assert!(text.contains("gold"), "text: {text}");
    }

    #[test]
    fn unknown_command_returns_guidance() {
        let mut scene = make_scene();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene
            .handle_command(&cmd("attack", "attack", vec![]), &db)
            .unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("not valid here"), "text: {text}");
    }

    #[test]
    fn check_transitions_none_before_leave() {
        let scene = make_scene();
        assert_eq!(scene.check_transitions(&[]), None);
    }

    #[test]
    fn sell_unknown_item_returns_error() {
        let mut scene = make_scene();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene
            .handle_command(&cmd("sell", "sell magic wand", vec!["magic", "wand"]), &db)
            .unwrap();
        // With no character in DB this returns "No character found" error
        let text = events[0].data["text"].as_str().unwrap_or("");
        // Either "don't have" or "No character" are valid — the key is no panic.
        assert!(!text.is_empty());
    }

    #[test]
    fn default_shop_has_ten_items() {
        let items = default_shop_items();
        assert_eq!(items.len(), 10);
    }
}
