//! Creature data model for Harsh Realm.
//!
//! Ported from `src/harsh_realm/models/creature.py`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::resolution::damage::Pool;

fn d_survive() -> String {
    "survive".to_string()
}
fn d_eight() -> i32 {
    8
}
fn d_four() -> i32 {
    4
}
fn d_one() -> i32 {
    1
}
fn d_fifteen() -> i32 {
    15
}
fn d_melee() -> String {
    "melee".to_string()
}
fn d_punch() -> String {
    "punch".to_string()
}
fn d_attacks() -> String {
    "attacks".to_string()
}
fn d_aggressive() -> String {
    "aggressive".to_string()
}

/// Capitalize the first character (rest unchanged), matching the lowercased
/// inputs used by the dragon generator.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Harvestable material definition embedded in creature data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatureHarvestable {
    /// Material yielded.
    pub material: String,
    /// Skill used (default `"survive"`).
    #[serde(default = "d_survive")]
    pub skill: String,
    /// Difficulty (default 8).
    #[serde(default = "d_eight")]
    pub difficulty: i32,
}

/// Immutable data record for a creature type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatureData {
    /// Unique id (e.g. `"wolf"`).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Hit dice count.
    pub hd: i32,
    /// HP rolled per hit die (default 4).
    #[serde(default = "d_four")]
    pub hp_per_hd: i32,
    /// Armour class.
    pub ac: i32,
    /// Flat attack roll bonus.
    pub attack_bonus: i32,
    /// Damage expression.
    pub damage: String,
    /// `"melee"`, `"ranged"`, or `"special"`.
    #[serde(default = "d_melee")]
    pub damage_type: String,
    /// Skill used for attacks.
    #[serde(default = "d_punch")]
    pub attack_skill: String,
    /// Attack narration phrase.
    #[serde(default = "d_attacks")]
    pub attack_description: String,
    /// Attacks per turn (default 1).
    #[serde(default = "d_one")]
    pub num_attacks: i32,
    /// AI behaviour tag.
    #[serde(default = "d_aggressive")]
    pub behavior: String,
    /// Detection difficulty (default 8).
    #[serde(default = "d_eight")]
    pub awareness_difficulty: i32,
    /// Flee-check difficulty (default 8).
    #[serde(default = "d_eight")]
    pub flee_difficulty: i32,
    /// Whether the creature always surprises.
    #[serde(default)]
    pub unavoidable: bool,
    /// Morale rating (default 8).
    #[serde(default = "d_eight")]
    pub morale: i32,
    /// Loot table id, if any.
    #[serde(default)]
    pub loot_table: Option<String>,
    /// Harvestable definition, if any.
    #[serde(default)]
    pub harvestable: Option<CreatureHarvestable>,
    /// Description before the creature is visible.
    #[serde(default)]
    pub description_unseen: String,
    /// Description when fully visible.
    #[serde(default)]
    pub description_seen: String,
    /// Short label.
    #[serde(default)]
    pub description_short: String,
    /// XP awarded (default 15).
    #[serde(default = "d_fifteen")]
    pub xp_value: i32,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Special ability strings.
    #[serde(default)]
    pub special_abilities: Vec<String>,
    /// Intrinsic IR trait ids the creature carries (sourced into combat so the
    /// creature's own bite/breath can fire IR triggers and apply modifiers).
    #[serde(default)]
    pub traits: Vec<String>,
    /// Innate attack ids.
    #[serde(default)]
    pub innate_attacks: Vec<String>,
    /// Authored action ids (the IR action-model; not yet consumed by the turn
    /// loop, but carried so it is no longer dropped).
    #[serde(default)]
    pub actions: Vec<String>,
    /// Typed health pools beyond the derived `hp` pool (e.g. shields, MDC). Empty
    /// for the common single-`hp` creature.
    #[serde(default)]
    pub pools: Vec<Pool>,
    /// Named defenses (e.g. `{"ac": 14}`). Avoidance defenses are carried for
    /// future attack-contest use; defenses named `dr`/`armor` also act as damage
    /// mitigation in the IR damage pipeline.
    #[serde(default)]
    pub defenses: BTreeMap<String, i64>,
}

impl CreatureData {
    /// Apply the dynamic default: when `description_short` is empty, set it to
    /// the lowercased name (mirrors the Python `model_validator`).
    pub fn apply_defaults(&mut self) {
        if self.description_short.is_empty() && !self.name.is_empty() {
            self.description_short = self.name.to_lowercase();
        }
    }
}

/// One authored creature YAML document.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CreatureCatalog {
    /// Creatures.
    #[serde(default)]
    pub creatures: Vec<CreatureData>,
}

/// Static stat bundle used for generated dragons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragonSizeProfile {
    /// Hit dice.
    pub hd: i32,
    /// HP per hit die.
    pub hp_per_hd: i32,
    /// Armour class.
    pub ac: i32,
    /// Attack bonus.
    pub attack_bonus: i32,
    /// Damage expression.
    pub damage: &'static str,
    /// XP value.
    pub xp_value: i32,
}

/// Registry of all creature types loaded from YAML files.
#[derive(Debug, Clone, Default)]
pub struct CreatureRegistry {
    creatures: BTreeMap<String, CreatureData>,
}

impl CreatureRegistry {
    /// Build a registry from an in-memory list.
    pub fn new(creatures: Vec<CreatureData>) -> Self {
        let mut map = BTreeMap::new();
        for c in creatures {
            map.insert(c.id.clone(), c);
        }
        CreatureRegistry { creatures: map }
    }

    /// Load all creature YAML files from `data_dir/creatures/*.yaml`.
    ///
    /// Mirrors the Python loader: unreadable/unparseable files are skipped.
    pub fn load(data_dir: &Path) -> Self {
        let creatures_dir = data_dir.join("creatures");
        let mut all: Vec<CreatureData> = Vec::new();
        let Ok(read_dir) = std::fs::read_dir(&creatures_dir) else {
            return CreatureRegistry::default();
        };
        let mut paths: Vec<_> = read_dir
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
            .collect();
        paths.sort();
        for path in paths {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(catalog) = serde_yaml::from_str::<CreatureCatalog>(&text) else {
                continue;
            };
            for mut creature in catalog.creatures {
                creature.apply_defaults();
                all.push(creature);
            }
        }
        CreatureRegistry::new(all)
    }

    /// Merge additional creatures into the registry, last-wins on id. Used to fold
    /// the world's IR-authored creatures into the legacy file catalog so there is
    /// one creature catalog at runtime (IR can augment or override legacy entries).
    pub fn extend(&mut self, creatures: impl IntoIterator<Item = CreatureData>) {
        for c in creatures {
            self.creatures.insert(c.id.clone(), c);
        }
    }

    /// Retrieve a creature by id.
    pub fn get(&self, creature_id: &str) -> Option<&CreatureData> {
        self.creatures.get(creature_id)
    }

    /// All creatures that have every one of `tags`.
    pub fn by_tags(&self, tags: &[String]) -> Vec<&CreatureData> {
        self.creatures
            .values()
            .filter(|c| tags.iter().all(|t| c.tags.contains(t)))
            .collect()
    }

    /// All registered creatures.
    pub fn all(&self) -> Vec<&CreatureData> {
        self.creatures.values().collect()
    }
}

/// Stat profile for a dragon size, or `None` if unknown.
fn dragon_size(size: &str) -> Option<DragonSizeProfile> {
    Some(match size {
        "small" => DragonSizeProfile {
            hd: 3,
            hp_per_hd: 4,
            ac: 14,
            attack_bonus: 3,
            damage: "1d8",
            xp_value: 45,
        },
        "medium" => DragonSizeProfile {
            hd: 5,
            hp_per_hd: 5,
            ac: 15,
            attack_bonus: 5,
            damage: "1d10",
            xp_value: 75,
        },
        "large" => DragonSizeProfile {
            hd: 8,
            hp_per_hd: 6,
            ac: 17,
            attack_bonus: 8,
            damage: "2d8",
            xp_value: 120,
        },
        "huge" => DragonSizeProfile {
            hd: 12,
            hp_per_hd: 8,
            ac: 19,
            attack_bonus: 12,
            damage: "3d8",
            xp_value: 180,
        },
        _ => return None,
    })
}

/// Element descriptor for a dragon, or `None` if unknown.
fn dragon_element_descriptor(element: &str) -> Option<&'static str> {
    match element {
        "fire" => Some("fire-breathing"),
        "ice" => Some("ice-breathing"),
        "lightning" => Some("lightning-breathing"),
        "poison" => Some("poison-breathing"),
        _ => None,
    }
}

/// Generate a dragon `CreatureData` from size and element parameters.
///
/// Returns `Err` if size or element is not recognized.
pub fn generate_dragon(size: &str, element: &str) -> Result<CreatureData, String> {
    let sz = dragon_size(size)
        .ok_or_else(|| format!("Unknown dragon size {size:?}"))?;
    let descriptor = dragon_element_descriptor(element)
        .ok_or_else(|| format!("Unknown dragon element {element:?}"))?;

    Ok(CreatureData {
        id: format!("dragon_{size}_{element}"),
        name: format!("{} Dragon ({})", capitalize(element), capitalize(size)),
        hd: sz.hd,
        hp_per_hd: sz.hp_per_hd,
        ac: sz.ac,
        attack_bonus: sz.attack_bonus,
        damage: sz.damage.to_string(),
        damage_type: "special".to_string(),
        attack_skill: "punch".to_string(),
        attack_description: "snaps enormous jaws at".to_string(),
        num_attacks: 2,
        behavior: "aggressive".to_string(),
        awareness_difficulty: 5,
        flee_difficulty: 14,
        unavoidable: false,
        morale: 12,
        loot_table: Some("mythical".to_string()),
        harvestable: Some(CreatureHarvestable {
            material: "dragon scale".to_string(),
            skill: "survive".to_string(),
            difficulty: 14,
        }),
        description_unseen: format!(
            "A shadow crosses the sun. A {descriptor} dragon circles overhead."
        ),
        description_seen: format!(
            "A {size} {element}-breathing dragon descends, smoke curling from its nostrils."
        ),
        description_short: format!("{size} {element} dragon"),
        xp_value: sz.xp_value,
        tags: vec![
            "mythical".to_string(),
            "wilderness".to_string(),
            "dungeon".to_string(),
            "dragon".to_string(),
            element.to_string(),
        ],
        special_abilities: vec![format!("{element}_breath")],
        traits: Vec::new(),
        innate_attacks: Vec::new(),
        actions: Vec::new(),
        pools: Vec::new(),
        defenses: BTreeMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_defaults_sets_short_from_name() {
        let mut c: CreatureData = serde_json::from_str(
            r#"{"id":"wolf","name":"Grey Wolf","hd":1,"ac":12,"attack_bonus":1,"damage":"1d6"}"#,
        )
        .unwrap();
        assert_eq!(c.description_short, "");
        c.apply_defaults();
        assert_eq!(c.description_short, "grey wolf");
        assert_eq!(c.hp_per_hd, 4); // default
        assert_eq!(c.morale, 8); // default
    }

    #[test]
    fn registry_get_and_by_tags() {
        let a: CreatureData = serde_json::from_str(
            r#"{"id":"wolf","name":"Wolf","hd":1,"ac":12,"attack_bonus":1,"damage":"1d6","tags":["beast","wilderness"]}"#,
        )
        .unwrap();
        let b: CreatureData = serde_json::from_str(
            r#"{"id":"crab","name":"Crab","hd":1,"ac":13,"attack_bonus":0,"damage":"1d4","tags":["beast"]}"#,
        )
        .unwrap();
        let reg = CreatureRegistry::new(vec![a, b]);
        assert_eq!(reg.get("wolf").unwrap().name, "Wolf");
        assert_eq!(reg.by_tags(&["beast".to_string()]).len(), 2);
        assert_eq!(reg.by_tags(&["wilderness".to_string()]).len(), 1);
        assert_eq!(reg.all().len(), 2);
    }

    #[test]
    fn extend_merges_last_wins() {
        let base: CreatureData = serde_json::from_str(
            r#"{"id":"wolf","name":"Wolf","hd":1,"ac":12,"attack_bonus":1,"damage":"1d6"}"#,
        )
        .unwrap();
        let mut reg = CreatureRegistry::new(vec![base]);
        // An IR-sourced creature plus an override of the existing `wolf`.
        let ir_wolf: CreatureData = serde_json::from_str(
            r#"{"id":"wolf","name":"Dire Wolf","hd":3,"ac":14,"attack_bonus":3,"damage":"1d8"}"#,
        )
        .unwrap();
        let ir_new: CreatureData = serde_json::from_str(
            r#"{"id":"ash_crawler","name":"Ash Crawler","hd":2,"ac":12,"attack_bonus":2,"damage":"1d6"}"#,
        )
        .unwrap();
        reg.extend(vec![ir_wolf, ir_new]);
        assert_eq!(reg.all().len(), 2);
        assert_eq!(reg.get("wolf").unwrap().name, "Dire Wolf"); // IR overrode legacy
        assert!(reg.get("ash_crawler").is_some()); // IR-only creature is present
    }

    #[test]
    fn generate_dragon_builds_variant() {
        let d = generate_dragon("large", "fire").unwrap();
        assert_eq!(d.id, "dragon_large_fire");
        assert_eq!(d.name, "Fire Dragon (Large)");
        assert_eq!(d.hd, 8);
        assert_eq!(d.damage, "2d8");
        assert!(d.tags.contains(&"fire".to_string()));
        assert!(generate_dragon("colossal", "fire").is_err());
        assert!(generate_dragon("large", "acid").is_err());
    }
}
