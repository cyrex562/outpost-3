//! Pure faction turn action resolvers.
//!
//! Ported from `src/harsh_realm/faction/faction_turn.py`.
//!
//! Each action function takes pre-fetched faction/asset data and returns a
//! `FactionActionResult`. DB writes (update/delete assets and factions) are
//! performed by the caller (B9 async host). This keeps the resolution logic
//! pure and testable without a database.

use crate::admin::config::FactionAssetStat;
use crate::faction::{
    FactionActionResult, FactionAssetData, FactionAssetSpecialRule, FactionData,
};

/// Ticks per week (168 hours).
pub const TICKS_PER_WEEK: i32 = 168;

/// Actions that are considered significant (emit events).
pub use super::turn_support::SIGNIFICANT_ACTIONS;

// ── helpers ─────────────────────────────────────────────────────────────────

fn ok(faction: &FactionData, action: &str, narration: impl Into<String>) -> FactionActionResult {
    FactionActionResult {
        faction_id: faction.id.clone(),
        faction_name: faction.name.clone(),
        action: action.to_string(),
        success: true,
        narration: narration.into(),
        details: Default::default(),
    }
}

fn fail(faction: &FactionData, action: &str, narration: impl Into<String>) -> FactionActionResult {
    FactionActionResult {
        faction_id: faction.id.clone(),
        faction_name: faction.name.clone(),
        action: action.to_string(),
        success: false,
        narration: narration.into(),
        details: Default::default(),
    }
}

// ── action resolvers ────────────────────────────────────────────────────────

/// Outcome of the attack action including HP mutations the caller must apply.
#[derive(Debug, Clone)]
pub struct AttackOutcome {
    pub result: FactionActionResult,
    /// New HP for the target (or None if destroyed).
    pub target_new_hp: Option<i32>,
    /// New HP for the attacker (or None if destroyed).
    pub attacker_new_hp: Option<i32>,
}

/// Resolve an attack between two assets. The caller rolls damage via `roll_fn`.
pub fn resolve_attack<F>(
    faction: &FactionData,
    attacker: &FactionAssetData,
    target: &FactionAssetData,
    attacker_stats: Option<&FactionAssetStat>,
    defender_stats: Option<&FactionAssetStat>,
    mut roll_fn: F,
) -> AttackOutcome
where
    F: FnMut(&str) -> i32,
{
    let attacker_stats = match attacker_stats {
        Some(s) => s,
        None => {
            return AttackOutcome {
                result: fail(faction, "attack", format!("No stats found for {}.", attacker.asset_type)),
                target_new_hp: Some(target.hp),
                attacker_new_hp: Some(attacker.hp),
            };
        }
    };

    if attacker_stats.attack_roll.is_empty() {
        return AttackOutcome {
            result: fail(faction, "attack", format!("{} cannot attack.", attacker.asset_type)),
            target_new_hp: Some(target.hp),
            attacker_new_hp: Some(attacker.hp),
        };
    }

    let damage = roll_fn(&attacker_stats.attack_roll);
    let counter = defender_stats
        .filter(|s| !s.attack_roll.is_empty())
        .map(|s| roll_fn(&s.attack_roll))
        .unwrap_or(0);

    let new_target_hp = (target.hp - damage).max(0);
    let target_destroyed = new_target_hp <= 0;

    let new_attacker_hp = if counter > 0 {
        (attacker.hp - counter).max(0)
    } else {
        attacker.hp
    };
    let attacker_destroyed = counter > 0 && new_attacker_hp <= 0;

    let mut narration = format!(
        "{}'s {} attacks {} for {} damage",
        faction.name, attacker.asset_type, target.asset_type, damage
    );
    if target_destroyed {
        narration.push_str(". Target destroyed!");
    } else {
        narration.push_str(&format!(". Target HP: {new_target_hp}"));
    }
    if counter > 0 {
        narration.push_str(&format!(" Counter: {counter} damage back."));
    }

    let mut details = serde_json::Map::new();
    details.insert("attacker_type".into(), attacker.asset_type.clone().into());
    details.insert("target_type".into(), target.asset_type.clone().into());
    details.insert("target_faction_id".into(), target.faction_id.clone().into());
    details.insert("damage_dealt".into(), damage.into());
    details.insert("counter_damage".into(), counter.into());
    details.insert("target_destroyed".into(), target_destroyed.into());
    details.insert("attacker_destroyed".into(), attacker_destroyed.into());
    details.insert("target_hp".into(), new_target_hp.into());
    details.insert("attacker_hp".into(), new_attacker_hp.into());

    AttackOutcome {
        result: FactionActionResult {
            faction_id: faction.id.clone(),
            faction_name: faction.name.clone(),
            action: "attack".into(),
            success: true,
            details,
            narration,
        },
        target_new_hp: if target_destroyed { None } else { Some(new_target_hp) },
        attacker_new_hp: if attacker_destroyed { None } else { Some(new_attacker_hp) },
    }
}

/// Resolve an expand action (marks asset as expanded; caller persists).
pub fn resolve_expand(faction: &FactionData, asset: &FactionAssetData) -> FactionActionResult {
    let mut r = ok(faction, "expand", format!("{} expands their {} influence.", faction.name, asset.asset_type));
    r.details.insert("asset_type".into(), asset.asset_type.clone().into());
    r.details.insert("asset_id".into(), asset.id.clone().into());
    r
}

/// Resolve a create action (caller checks requirements and persists).
pub fn resolve_create(
    faction: &FactionData,
    asset_type: &str,
    stats: Option<&FactionAssetStat>,
) -> Result<FactionActionResult, FactionActionResult> {
    let stats = match stats {
        Some(s) => s,
        None => return Err(fail(faction, "create", format!("Unknown asset type: {asset_type}"))),
    };

    let faction_attr = faction_attribute(faction, &stats.category);
    if faction_attr < stats.min_attribute {
        let mut r = fail(faction, "create", format!(
            "{} lacks the {} ({}) to create {} (needs {}).",
            faction.name, stats.category, faction_attr, asset_type, stats.min_attribute
        ));
        r.details.insert("asset_type".into(), asset_type.into());
        r.details.insert("required".into(), stats.min_attribute.into());
        r.details.insert("current".into(), faction_attr.into());
        return Err(r);
    }

    if faction.wealth < stats.cost {
        let mut r = fail(faction, "create", format!(
            "{} cannot afford {} (cost {}, wealth {}).",
            faction.name, asset_type, stats.cost, faction.wealth
        ));
        r.details.insert("asset_type".into(), asset_type.into());
        r.details.insert("cost".into(), stats.cost.into());
        r.details.insert("wealth".into(), faction.wealth.into());
        return Err(r);
    }

    let new_wealth = faction.wealth - stats.cost;
    let mut r = ok(faction, "create", format!(
        "{} creates {asset_type} for {} wealth.", faction.name, stats.cost
    ));
    r.details.insert("asset_type".into(), asset_type.into());
    r.details.insert("cost".into(), stats.cost.into());
    r.details.insert("new_wealth".into(), new_wealth.into());
    Ok(r)
}

/// Resolve a repair action (caller deducts wealth and restores HP).
pub fn resolve_repair(
    faction: &FactionData,
    asset: &FactionAssetData,
    stats: Option<&FactionAssetStat>,
) -> Result<FactionActionResult, FactionActionResult> {
    let stats = match stats {
        Some(s) => s,
        None => return Err(fail(faction, "repair", format!("No stats for {}.", asset.asset_type))),
    };
    let repair_cost = (stats.cost / 2).max(1);
    if faction.wealth < repair_cost {
        let mut r = fail(faction, "repair", format!("{} cannot afford to repair {}.", faction.name, asset.asset_type));
        r.details.insert("cost".into(), repair_cost.into());
        r.details.insert("wealth".into(), faction.wealth.into());
        return Err(r);
    }
    let new_wealth = faction.wealth - repair_cost;
    let mut r = ok(faction, "repair", format!(
        "{} repairs {} to full HP ({} -> {}) for {} wealth.",
        faction.name, asset.asset_type, asset.hp, asset.max_hp, repair_cost
    ));
    r.details.insert("asset_type".into(), asset.asset_type.clone().into());
    r.details.insert("old_hp".into(), asset.hp.into());
    r.details.insert("new_hp".into(), asset.max_hp.into());
    r.details.insert("cost".into(), repair_cost.into());
    r.details.insert("new_wealth".into(), new_wealth.into());
    Ok(r)
}

/// Resolve a seize action (placeholder — consolidates territory).
pub fn resolve_seize(faction: &FactionData) -> FactionActionResult {
    ok(faction, "seize", format!("{} consolidates control over their territory.", faction.name))
}

/// Resolve a sell action (caller deletes asset and updates wealth).
pub fn resolve_sell(
    faction: &FactionData,
    asset: &FactionAssetData,
    stats: Option<&FactionAssetStat>,
) -> Result<FactionActionResult, FactionActionResult> {
    let stats = match stats {
        Some(s) => s,
        None => return Err(fail(faction, "sell", format!("No stats for {}.", asset.asset_type))),
    };
    let recovery = (stats.cost / 2).max(1);
    let new_wealth = faction.wealth + recovery;
    let mut r = ok(faction, "sell", format!("{} sells {} for {} wealth.", faction.name, asset.asset_type, recovery));
    r.details.insert("asset_type".into(), asset.asset_type.clone().into());
    r.details.insert("recovery".into(), recovery.into());
    r.details.insert("new_wealth".into(), new_wealth.into());
    Ok(r)
}

/// Resolve a refit action (heal 1 HP; caller persists).
pub fn resolve_refit(faction: &FactionData) -> FactionActionResult {
    if faction.hp >= faction.max_hp {
        let mut r = ok(faction, "refit", format!("{} is already at full strength.", faction.name));
        r.details.insert("hp".into(), faction.hp.into());
        r.details.insert("max_hp".into(), faction.max_hp.into());
        return r;
    }
    let new_hp = (faction.hp + 1).min(faction.max_hp);
    let mut r = ok(faction, "refit", format!("{} refits, recovering HP ({} -> {}).", faction.name, faction.hp, new_hp));
    r.details.insert("old_hp".into(), faction.hp.into());
    r.details.insert("new_hp".into(), new_hp.into());
    r.details.insert("max_hp".into(), faction.max_hp.into());
    r
}

/// Resolve a harvest action. Returns `(result, total_income)`.
pub fn resolve_harvest(
    faction: &FactionData,
    assets: &[FactionAssetData],
    stat_lookup: &dyn Fn(&str) -> Option<FactionAssetStat>,
) -> (FactionActionResult, i32) {
    let mut total_income = 0i32;
    for asset in assets {
        if let Some(stats) = stat_lookup(&asset.asset_type) {
            if let Some(special) = FactionAssetSpecialRule::from_raw(&stats.special) {
                if special.ability == "generate_wealth" {
                    total_income += special.amount;
                }
            }
        }
    }
    let new_wealth = faction.wealth + total_income;
    let narration = if total_income > 0 {
        format!("{} harvests {} wealth from their assets.", faction.name, total_income)
    } else {
        format!("{} has no wealth-generating assets.", faction.name)
    };
    let mut r = ok(faction, "harvest", narration);
    r.details.insert("income".into(), total_income.into());
    r.details.insert("new_wealth".into(), new_wealth.into());
    (r, total_income)
}

fn faction_attribute(faction: &FactionData, category: &str) -> i32 {
    match category {
        "force" => faction.force,
        "cunning" => faction.cunning,
        "wealth" => faction.wealth,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::config::FactionAssetStat;
    use crate::faction::{FactionAssetData, FactionData};

    fn faction() -> FactionData {
        serde_json::from_value(serde_json::json!({
            "id": "f1", "name": "Reavers",
            "hp": 5, "max_hp": 7,
            "force": 3, "cunning": 2, "wealth": 10,
            "xp": 0, "home_q": 0, "home_r": 0, "goal": "expand", "description": ""
        })).unwrap()
    }

    fn asset(id: &str, asset_type: &str, hp: i32, max_hp: i32) -> FactionAssetData {
        serde_json::from_value(serde_json::json!({
            "id": id, "faction_id": "f1", "asset_type": asset_type,
            "category": "force", "hp": hp, "max_hp": max_hp,
            "location_q": 0, "location_r": 0, "data": {}
        })).unwrap()
    }

    fn stat(asset_type: &str, cost: i32, max_hp: i32, category: &str, min_attr: i32, attack_roll: &str) -> FactionAssetStat {
        FactionAssetStat {
            asset_type: asset_type.into(),
            category: category.into(),
            min_attribute: min_attr,
            cost,
            max_hp,
            attack_roll: attack_roll.into(),
            special: String::new(),
            upkeep: 0,
            attack_stat: String::new(),
            counter_stat: String::new(),
            description: String::new(),
        }
    }

    #[test]
    fn attack_deals_damage() {
        let f = faction();
        let attacker = asset("a1", "infantry", 4, 4);
        let target = asset("t1", "cavalry", 5, 5); // hp 5, damage 3 → hp 2
        let a_stats = stat("infantry", 2, 4, "force", 1, "1d6");
        let d_stats = stat("cavalry", 2, 4, "force", 1, "");
        let outcome = resolve_attack(&f, &attacker, &target, Some(&a_stats), Some(&d_stats), |_| 3);
        assert!(outcome.result.success);
        assert_eq!(outcome.target_new_hp, Some(2)); // 5 - 3 = 2
    }

    #[test]
    fn attack_missing_stats_fails() {
        let f = faction();
        let attacker = asset("a1", "unknown", 4, 4);
        let target = asset("t1", "cavalry", 3, 4);
        let outcome = resolve_attack(&f, &attacker, &target, None, None, |_| 3);
        assert!(!outcome.result.success);
    }

    #[test]
    fn expand_succeeds() {
        let f = faction();
        let a = asset("a1", "spy", 2, 2);
        let r = resolve_expand(&f, &a);
        assert!(r.success);
        assert_eq!(r.action, "expand");
    }

    #[test]
    fn create_insufficient_wealth_fails() {
        let mut f = faction();
        f.wealth = 0;
        let s = stat("spy_network", 5, 4, "cunning", 1, "");
        let res = resolve_create(&f, "spy_network", Some(&s));
        assert!(res.is_err());
    }

    #[test]
    fn create_sufficient_wealth_succeeds() {
        let f = faction();
        let s = stat("raider_band", 3, 4, "force", 1, "");
        let res = resolve_create(&f, "raider_band", Some(&s));
        assert!(res.is_ok());
    }

    #[test]
    fn repair_insufficient_wealth_fails() {
        let mut f = faction();
        f.wealth = 0;
        let a = asset("a1", "infantry", 1, 4);
        let s = stat("infantry", 4, 4, "force", 1, "1d6");
        let res = resolve_repair(&f, &a, Some(&s));
        assert!(res.is_err());
    }

    #[test]
    fn refit_at_max_hp() {
        let mut f = faction();
        f.hp = f.max_hp;
        let r = resolve_refit(&f);
        assert!(r.success);
        assert!(r.narration.contains("full strength"));
    }

    #[test]
    fn refit_below_max_hp() {
        let f = faction();
        let r = resolve_refit(&f);
        assert!(r.success);
        assert_eq!(r.details["new_hp"], 6);
    }

    #[test]
    fn harvest_no_generators() {
        let f = faction();
        let (r, income) = resolve_harvest(&f, &[], &|_| None);
        assert!(r.success);
        assert_eq!(income, 0);
    }
}
