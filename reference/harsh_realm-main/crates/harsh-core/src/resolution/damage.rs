//! The system-agnostic damage & health pipeline (R5-P4).
//!
//! A [`DamagePacket`] flows through fixed stages — wounding → mitigation →
//! routing → pool application — producing pool deltas the host applies. AC lives
//! only at the *avoidance* stage (the attack contest); it is not in this
//! pipeline. The same stages express XWN HP, Rifts SDC/MDC (tiered pools + MD
//! bypass), GURPS (type wounding + subtractive DR), and Savage (threshold).
//!
//! The damage *amount* is rolled by the host (dice stay out of the core); this
//! pipeline is a pure function of the packet + the entity's config.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Damage tier — Rifts' structural vs mega distinction. XWN damage is `Sd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DamageTier {
    #[default]
    Sd,
    Md,
}

/// An incoming damage packet (amount already rolled).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DamagePacket {
    pub amount: i64,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub tier: DamageTier,
}

/// A mitigation an entity applies to incoming damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MitigationKind {
    /// Subtract `value` from the amount (floored at 0). GURPS DR, XWN armor-as-DR.
    DamageResistance,
    /// If the amount is at or below `value`, it is negated. Savage Toughness-style.
    ArmorRating,
}

/// One mitigation entry. `applies_to_types` empty = applies to all damage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mitigation {
    pub kind: MitigationKind,
    pub value: i64,
    #[serde(default)]
    pub applies_to_types: Vec<String>,
}

/// A typed health pool on an entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Pool {
    pub id: String,
    /// Routing tags, e.g. `["sd"]`, `["md"]`, `["hp"]`. A pool tagged `md` only
    /// takes `Md` packets; others take `Sd`.
    #[serde(default)]
    pub tags: Vec<String>,
    pub current: i64,
    pub max: i64,
    #[serde(default)]
    pub can_go_negative: bool,
}

impl Pool {
    /// Whether this pool accepts a packet of the given tier (the MD-bypass rule).
    fn accepts(&self, tier: DamageTier) -> bool {
        let is_md_pool = self.tags.iter().any(|t| t == "md");
        match tier {
            DamageTier::Md => is_md_pool,
            DamageTier::Sd => !is_md_pool,
        }
    }
}

/// A change to apply to one pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolDelta {
    pub pool_id: String,
    pub delta: i64,
}

/// The result of pushing a packet through the pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageApplication {
    /// Amount after wounding + mitigation (what actually hit the pool).
    pub final_amount: i64,
    /// Pool changes the host should apply.
    pub deltas: Vec<PoolDelta>,
    /// Pools that reached or passed zero (depletion → the host fires effects).
    pub depleted: Vec<String>,
}

fn matches_types(applies_to: &[String], packet_types: &[String]) -> bool {
    applies_to.is_empty() || applies_to.iter().any(|t| packet_types.contains(t))
}

/// Wounding: multiply the amount by the product of per-type multipliers for the
/// packet's types (types absent from `wounding` count as ×1). Truncated to int.
fn apply_wounding(amount: i64, types: &[String], wounding: &BTreeMap<String, f64>) -> i64 {
    let mut mult = 1.0_f64;
    for t in types {
        if let Some(m) = wounding.get(t) {
            mult *= *m;
        }
    }
    (amount as f64 * mult) as i64
}

/// Mitigation: apply each matching mitigation in order.
fn apply_mitigation(mut amount: i64, types: &[String], mitigations: &[Mitigation]) -> i64 {
    for m in mitigations {
        if !matches_types(&m.applies_to_types, types) {
            continue;
        }
        match m.kind {
            MitigationKind::DamageResistance => amount = (amount - m.value).max(0),
            MitigationKind::ArmorRating => {
                if amount <= m.value {
                    amount = 0;
                }
            }
        }
    }
    amount
}

/// Push a packet through wounding → mitigation → routing → application.
///
/// Routing picks the first pool that accepts the packet's tier (MD bypasses SD).
/// If no pool accepts it, the damage is dropped (the Sd-vs-MDC ≈ 0 case) and the
/// application is empty.
pub fn apply_damage(
    packet: &DamagePacket,
    wounding: &BTreeMap<String, f64>,
    mitigations: &[Mitigation],
    pools: &[Pool],
) -> DamageApplication {
    let wounded = apply_wounding(packet.amount, &packet.types, wounding);
    let final_amount = apply_mitigation(wounded, &packet.types, mitigations);

    let mut deltas = Vec::new();
    let mut depleted = Vec::new();

    if let Some(pool) = pools.iter().find(|p| p.accepts(packet.tier)) {
        let target = pool.current - final_amount;
        let new_current = if pool.can_go_negative {
            target
        } else {
            target.max(0)
        };
        let delta = new_current - pool.current;
        if delta != 0 {
            deltas.push(PoolDelta {
                pool_id: pool.id.clone(),
                delta,
            });
        }
        if new_current <= 0 {
            depleted.push(pool.id.clone());
        }
    }

    DamageApplication {
        final_amount,
        deltas,
        depleted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hp(current: i64) -> Pool {
        Pool {
            id: "hp".into(),
            tags: vec!["hp".into()],
            current,
            max: current,
            can_go_negative: false,
        }
    }

    fn packet(amount: i64, types: &[&str], tier: DamageTier) -> DamagePacket {
        DamagePacket {
            amount,
            types: types.iter().map(|s| s.to_string()).collect(),
            tier,
        }
    }

    #[test]
    fn xwn_basic_damage_to_hp() {
        let app = apply_damage(
            &packet(6, &["physical"], DamageTier::Sd),
            &BTreeMap::new(),
            &[],
            &[hp(10)],
        );
        assert_eq!(app.final_amount, 6);
        assert_eq!(
            app.deltas,
            vec![PoolDelta {
                pool_id: "hp".into(),
                delta: -6
            }]
        );
        assert!(app.depleted.is_empty());
    }

    #[test]
    fn depletion_and_clamp_at_zero() {
        let app = apply_damage(
            &packet(15, &["physical"], DamageTier::Sd),
            &BTreeMap::new(),
            &[],
            &[hp(10)],
        );
        // Clamped: hp 10 → 0, so delta is -10 (not -15), and it's depleted.
        assert_eq!(
            app.deltas,
            vec![PoolDelta {
                pool_id: "hp".into(),
                delta: -10
            }]
        );
        assert_eq!(app.depleted, vec!["hp".to_string()]);
    }

    #[test]
    fn can_go_negative_pool() {
        let mut p = hp(10);
        p.can_go_negative = true;
        let app = apply_damage(
            &packet(15, &[], DamageTier::Sd),
            &BTreeMap::new(),
            &[],
            &[p],
        );
        assert_eq!(
            app.deltas,
            vec![PoolDelta {
                pool_id: "hp".into(),
                delta: -15
            }]
        );
        assert_eq!(app.depleted, vec!["hp".to_string()]);
    }

    #[test]
    fn gurps_style_wounding_and_dr() {
        // impaling ×2 then DR 3: 5 × 2 = 10, − 3 = 7.
        let mut wounding = BTreeMap::new();
        wounding.insert("impaling".to_string(), 2.0);
        let mits = vec![Mitigation {
            kind: MitigationKind::DamageResistance,
            value: 3,
            applies_to_types: vec![],
        }];
        let app = apply_damage(
            &packet(5, &["impaling"], DamageTier::Sd),
            &wounding,
            &mits,
            &[hp(20)],
        );
        assert_eq!(app.final_amount, 7);
    }

    #[test]
    fn armor_rating_threshold_negates_small_hits() {
        let mits = vec![Mitigation {
            kind: MitigationKind::ArmorRating,
            value: 4,
            applies_to_types: vec![],
        }];
        // 4 ≤ 4 → negated.
        let app = apply_damage(
            &packet(4, &[], DamageTier::Sd),
            &BTreeMap::new(),
            &mits,
            &[hp(10)],
        );
        assert_eq!(app.final_amount, 0);
        assert!(app.deltas.is_empty());
        // 5 > 4 → passes.
        let app = apply_damage(
            &packet(5, &[], DamageTier::Sd),
            &BTreeMap::new(),
            &mits,
            &[hp(10)],
        );
        assert_eq!(app.final_amount, 5);
    }

    #[test]
    fn rifts_md_bypasses_sd_pool() {
        let sd = Pool {
            id: "sdc".into(),
            tags: vec!["sd".into()],
            current: 50,
            max: 50,
            can_go_negative: false,
        };
        let md = Pool {
            id: "mdc".into(),
            tags: vec!["md".into()],
            current: 20,
            max: 20,
            can_go_negative: false,
        };
        // An MD packet routes past the SD pool to the MD pool.
        let app = apply_damage(
            &packet(5, &[], DamageTier::Md),
            &BTreeMap::new(),
            &[],
            &[sd.clone(), md.clone()],
        );
        assert_eq!(
            app.deltas,
            vec![PoolDelta {
                pool_id: "mdc".into(),
                delta: -5
            }]
        );
        // An SD packet hits the SD pool.
        let app = apply_damage(
            &packet(5, &[], DamageTier::Sd),
            &BTreeMap::new(),
            &[],
            &[sd, md],
        );
        assert_eq!(
            app.deltas,
            vec![PoolDelta {
                pool_id: "sdc".into(),
                delta: -5
            }]
        );
    }

    #[test]
    fn sd_packet_with_only_md_pool_is_dropped() {
        let md = Pool {
            id: "mdc".into(),
            tags: vec!["md".into()],
            current: 20,
            max: 20,
            can_go_negative: false,
        };
        let app = apply_damage(
            &packet(5, &[], DamageTier::Sd),
            &BTreeMap::new(),
            &[],
            &[md],
        );
        assert!(app.deltas.is_empty()); // SD vs MDC ≈ 0
    }
}
