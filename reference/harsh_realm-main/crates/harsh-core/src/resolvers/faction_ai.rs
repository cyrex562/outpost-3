//! Faction-turn AI decision (R5-R6).
//!
//! The priority-based action chooser ported from `faction/faction_ai.py`. It is
//! pure: the host gathers the faction's state (own assets, alliance-filtered enemy
//! assets, the asset-stat catalog with parsed specials) and applies the chosen
//! action's DB writes / dice rolls afterward.
//!
//! Tie-breaking matches Python exactly: `min`/`max` return the *first* extreme.
//! Rust's `min_by_key` already returns the first; `max_by_key` returns the *last*,
//! so max selections iterate `.rev()` to recover first-wins.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const ASSET_CAP: i64 = 8;
const CREATE_WEALTH_THRESHOLD: i64 = 3;

#[derive(Debug, Clone, Deserialize)]
pub struct FactionState {
    pub id: String,
    pub hp: i64,
    pub max_hp: i64,
    pub force: i64,
    pub cunning: i64,
    pub wealth: i64,
}

impl FactionState {
    fn attribute(&self, category: &str) -> i64 {
        match category {
            "force" => self.force,
            "cunning" => self.cunning,
            "wealth" => self.wealth,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub id: String,
    pub asset_type: String,
    pub hp: i64,
    pub max_hp: i64,
    #[serde(default)]
    pub expanded: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetStat {
    pub asset_type: String,
    pub category: String,
    pub min_attribute: i64,
    pub cost: i64,
    #[serde(default)]
    pub max_hp: i64,
    #[serde(default)]
    pub attack_roll: String,
    #[serde(default)]
    pub special_ability: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FactionTurnInput {
    pub faction: FactionState,
    #[serde(default)]
    pub own_assets: Vec<Asset>,
    /// Already filtered: not ours, not allied.
    #[serde(default)]
    pub enemy_assets: Vec<Asset>,
    /// The asset-stat catalog, in repository order (create scans it in order).
    #[serde(default)]
    pub asset_stats: Vec<AssetStat>,
}

/// The chosen action; ids reference assets the host already gathered.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ActionChoice {
    pub action_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attacker_asset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_asset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_type: Option<String>,
}

fn empty(action: &str) -> ActionChoice {
    ActionChoice {
        action_name: action.to_string(),
        ..Default::default()
    }
}

/// Choose this faction's action for the turn.
pub fn choose_action(input: &FactionTurnInput) -> ActionChoice {
    let stats: BTreeMap<&str, &AssetStat> = input
        .asset_stats
        .iter()
        .map(|s| (s.asset_type.as_str(), s))
        .collect();
    let faction = &input.faction;

    // 1. Attack if HP > 50%.
    if faction.hp > faction.max_hp / 2 {
        if let Some(choice) = find_attack_target(&input.own_assets, &input.enemy_assets, &stats) {
            return choice;
        }
    }

    // 2. Repair a badly-damaged asset we can afford.
    if let Some(choice) = find_repair_target(faction, &input.own_assets, &stats) {
        return choice;
    }

    // 3. Create a new asset if wealthy enough and below the cap.
    if faction.wealth >= CREATE_WEALTH_THRESHOLD && (input.own_assets.len() as i64) < ASSET_CAP {
        if let Some(choice) = find_create_option(faction, &input.asset_stats) {
            return choice;
        }
    }

    // 4. Expand the first asset that hasn't expanded.
    for asset in &input.own_assets {
        if !asset.expanded {
            return ActionChoice {
                action_name: "expand".to_string(),
                asset_id: Some(asset.id.clone()),
                ..Default::default()
            };
        }
    }

    // 5. Harvest if any asset generates wealth.
    let has_wealth_gen = input.own_assets.iter().any(|a| {
        stats
            .get(a.asset_type.as_str())
            .is_some_and(|s| s.special_ability == "generate_wealth")
    });
    if has_wealth_gen {
        return empty("harvest");
    }

    // 6. Refit if below max HP.
    if faction.hp < faction.max_hp {
        return empty("refit");
    }

    // Default: harvest (a no-op without generators).
    empty("harvest")
}

fn find_attack_target(
    own: &[Asset],
    enemies: &[Asset],
    stats: &BTreeMap<&str, &AssetStat>,
) -> Option<ActionChoice> {
    let attackers: Vec<&Asset> = own
        .iter()
        .filter(|a| {
            stats
                .get(a.asset_type.as_str())
                .is_some_and(|s| !s.attack_roll.is_empty())
        })
        .collect();
    if attackers.is_empty() || enemies.is_empty() {
        return None;
    }
    // Target the weakest enemy (first on ties — matches Python `min`).
    let target = enemies.iter().min_by_key(|a| a.hp)?;
    // Strongest attacker by its stat max_hp (first on ties — `.rev()` recovers
    // first-wins from Rust's last-wins `max_by_key`).
    let attacker = attackers
        .iter()
        .rev()
        .max_by_key(|a| stats.get(a.asset_type.as_str()).map_or(0, |s| s.max_hp))?;
    Some(ActionChoice {
        action_name: "attack".to_string(),
        attacker_asset_id: Some(attacker.id.clone()),
        target_asset_id: Some(target.id.clone()),
        ..Default::default()
    })
}

fn find_repair_target(
    faction: &FactionState,
    assets: &[Asset],
    stats: &BTreeMap<&str, &AssetStat>,
) -> Option<ActionChoice> {
    // Damaged = below half HP (float comparison, matching Python `max_hp * 0.5`).
    let damaged: Vec<&Asset> = assets
        .iter()
        .filter(|a| (a.hp as f64) < (a.max_hp as f64) * 0.5)
        .collect();
    let target = damaged.iter().min_by_key(|a| a.hp)?;
    let stat = stats.get(target.asset_type.as_str())?;
    let repair_cost = (stat.cost / 2).max(1);
    if faction.wealth < repair_cost {
        return None;
    }
    Some(ActionChoice {
        action_name: "repair".to_string(),
        asset_id: Some(target.id.clone()),
        ..Default::default()
    })
}

fn find_create_option(faction: &FactionState, asset_stats: &[AssetStat]) -> Option<ActionChoice> {
    for stat in asset_stats {
        if faction.attribute(&stat.category) >= stat.min_attribute && faction.wealth >= stat.cost {
            return Some(ActionChoice {
                action_name: "create".to_string(),
                asset_type: Some(stat.asset_type.clone()),
                ..Default::default()
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stat(asset_type: &str, category: &str, cost: i64, min_attr: i64) -> AssetStat {
        AssetStat {
            asset_type: asset_type.to_string(),
            category: category.to_string(),
            min_attribute: min_attr,
            cost,
            max_hp: 4,
            attack_roll: String::new(),
            special_ability: String::new(),
        }
    }

    fn faction(hp: i64, wealth: i64) -> FactionState {
        FactionState { id: "f1".into(), hp, max_hp: 10, force: 3, cunning: 1, wealth }
    }

    #[test]
    fn attacks_weakest_enemy_with_strongest_attacker() {
        let mut gun = stat("gun", "force", 2, 0);
        gun.attack_roll = "1d6".into();
        gun.max_hp = 6;
        let input = FactionTurnInput {
            faction: faction(10, 0),
            own_assets: vec![
                Asset { id: "a1".into(), asset_type: "gun".into(), hp: 4, max_hp: 6, expanded: true },
            ],
            enemy_assets: vec![
                Asset { id: "e1".into(), asset_type: "x".into(), hp: 5, max_hp: 5, expanded: false },
                Asset { id: "e2".into(), asset_type: "x".into(), hp: 2, max_hp: 5, expanded: false },
            ],
            asset_stats: vec![gun],
        };
        let c = choose_action(&input);
        assert_eq!(c.action_name, "attack");
        assert_eq!(c.attacker_asset_id.as_deref(), Some("a1"));
        assert_eq!(c.target_asset_id.as_deref(), Some("e2")); // weakest
    }

    #[test]
    fn low_hp_skips_attack_and_repairs() {
        let input = FactionTurnInput {
            faction: faction(4, 10), // hp <= max/2 → no attack
            own_assets: vec![Asset {
                id: "a1".into(), asset_type: "fort".into(), hp: 1, max_hp: 6, expanded: true,
            }],
            enemy_assets: vec![],
            asset_stats: vec![stat("fort", "force", 4, 0)],
        };
        let c = choose_action(&input);
        assert_eq!(c.action_name, "repair");
        assert_eq!(c.asset_id.as_deref(), Some("a1"));
    }

    #[test]
    fn creates_first_affordable_in_catalog_order() {
        let input = FactionTurnInput {
            faction: faction(10, 5),
            own_assets: vec![],
            enemy_assets: vec![],
            asset_stats: vec![
                stat("expensive", "force", 9, 0), // unaffordable
                stat("cheap", "force", 2, 0),      // first affordable
                stat("cheaper", "force", 1, 0),
            ],
        };
        let c = choose_action(&input);
        assert_eq!(c.action_name, "create");
        assert_eq!(c.asset_type.as_deref(), Some("cheap"));
    }

    #[test]
    fn falls_through_to_refit_then_harvest() {
        // No assets, can't create (wealth 0) → not expand/harvest → refit (hp < max).
        let refit = choose_action(&FactionTurnInput {
            faction: faction(5, 0),
            own_assets: vec![],
            enemy_assets: vec![],
            asset_stats: vec![],
        });
        assert_eq!(refit.action_name, "refit");
        // Full HP → default harvest.
        let harvest = choose_action(&FactionTurnInput {
            faction: faction(10, 0),
            own_assets: vec![],
            enemy_assets: vec![],
            asset_stats: vec![],
        });
        assert_eq!(harvest.action_name, "harvest");
    }
}
