//! Shared character-build logic for the chat wizard and forms UI.
//!
//! Ported from `src/harsh_realm/gm/scenes/character_build.py`.

use std::collections::BTreeMap;

use crate::character::Character;
use crate::character_creation::{
    CharacterDraftSubmission, AttributeMethod, ClassOption, EquipmentKit,
    EquipmentKitCatalog, RecommendedSkill, SkillCatalog, SkillDefinition, SkillRequirement,
};
use crate::dice::attr_modifier;
use crate::packs::paths::default_content_dir;
use crate::runtime::InventoryItemRecord;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const VALID_CLASSES: &[&str] = &["warrior", "expert", "adventurer"];
pub const STANDARD_ARRAY: &[i32] = &[15, 14, 13, 12, 10, 8];
pub const POINT_BUY_BUDGET: i32 = 27;
pub const POINT_BUY_MIN: i32 = 8;
pub const POINT_BUY_MAX: i32 = 15;
pub const MAX_SKILL_LEVEL: i32 = 4;
pub const MIN_TRAINED_SKILL_LEVEL: i32 = 0;
pub const UNTRAINED_SKILL_LEVEL: i32 = -1;
pub const CURRENT_MILESTONE: i32 = 5;

pub const ATTRIBUTES: &[&str] = &["str", "dex", "con", "int", "wis", "cha"];

pub fn class_skill_points(class: &str) -> i32 {
    match class {
        "warrior" => 2,
        "expert" => 3,
        "adventurer" => 2,
        _ => 2,
    }
}

pub fn class_hit_die(class: &str) -> i32 {
    match class {
        "warrior" => 8,
        _ => 6,
    }
}

pub fn class_attack_bonus(class: &str) -> i32 {
    match class {
        "warrior" => 1,
        _ => 0,
    }
}

fn point_buy_cost_for_score(score: i32) -> Option<i32> {
    match score {
        8 => Some(0),
        9 => Some(1),
        10 => Some(2),
        11 => Some(3),
        12 => Some(4),
        13 => Some(5),
        14 => Some(7),
        15 => Some(9),
        _ => None,
    }
}

/// Total point-buy cost for a list of scores. Returns `Err` if any score is out of range.
pub fn point_buy_cost(scores: &[i32]) -> Result<i32, String> {
    let mut total = 0;
    for &score in scores {
        let cost = point_buy_cost_for_score(score)
            .ok_or_else(|| format!("Score {score} is outside the point-buy range {POINT_BUY_MIN}-{POINT_BUY_MAX}."))?;
        total += cost;
    }
    Ok(total)
}

/// Cost to train a skill from untrained up to `level`.
pub fn skill_cost(level: i32) -> i32 {
    level - UNTRAINED_SKILL_LEVEL
}

pub fn skill_point_budget(class: &str) -> i32 {
    class_skill_points(class)
}

// ---------------------------------------------------------------------------
// Skill requirement evaluation
// ---------------------------------------------------------------------------

/// The in-progress character state a skill's requirements evaluate against.
#[derive(Debug, Clone, Default)]
pub struct SkillDraftContext {
    pub character_class: String,
    pub attributes: BTreeMap<String, i32>,
    pub current_milestone: i32,
    pub traits: Vec<String>,
}

impl SkillDraftContext {
    pub fn new(character_class: impl Into<String>, attributes: BTreeMap<String, i32>) -> Self {
        Self {
            character_class: character_class.into(),
            attributes,
            current_milestone: CURRENT_MILESTONE,
            traits: Vec::new(),
        }
    }
}

fn eval_requirement(req: &SkillRequirement, ctx: &SkillDraftContext) -> Option<String> {
    match req.requirement_type.as_str() {
        "class" => {
            if !req.classes.is_empty() && !req.classes.contains(&ctx.character_class) {
                return Some(req.message.clone().unwrap_or_else(|| {
                    format!("Requires class: {}.", req.classes.join(", "))
                }));
            }
            None
        }
        "attribute" => {
            if let (Some(attr), Some(min)) = (&req.attribute, req.min) {
                if ctx.attributes.get(attr).copied().unwrap_or(0) < min {
                    return Some(req.message.clone().unwrap_or_else(|| {
                        format!("Requires {} {}+.", attr.to_uppercase(), min)
                    }));
                }
            }
            None
        }
        "trait" => {
            if let Some(trait_id) = &req.trait_id {
                if !ctx.traits.contains(trait_id) {
                    return Some(req.message.clone().unwrap_or_else(|| {
                        format!("Requires: {}.", trait_id.replace('_', " "))
                    }));
                }
            }
            None
        }
        other => Some(
            req.message.clone().unwrap_or_else(|| format!("Requires: {other}."))
        ),
    }
}

/// Return every reason a skill is un-takeable (empty = takeable).
pub fn skill_lock_reasons(
    available_milestone: i32,
    requirements: &[SkillRequirement],
    ctx: &SkillDraftContext,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if available_milestone > ctx.current_milestone {
        reasons.push("Not yet available — unlocks at a later milestone.".to_string());
    }
    for req in requirements {
        if let Some(reason) = eval_requirement(req, ctx) {
            reasons.push(reason);
        }
    }
    reasons
}

// ---------------------------------------------------------------------------
// Skill auto-pick
// ---------------------------------------------------------------------------

/// Spend a class's starting skill points on its recommended skills.
pub fn auto_pick_skills(
    character_class: &str,
    attributes: &BTreeMap<String, i32>,
    skill_defs: &BTreeMap<String, SkillDefinition>,
    recommendations: &[RecommendedSkill],
    current_milestone: i32,
) -> BTreeMap<String, i32> {
    let mut budget = skill_point_budget(character_class);
    let ctx = SkillDraftContext {
        character_class: character_class.to_string(),
        attributes: attributes.clone(),
        current_milestone,
        traits: Vec::new(),
    };
    let mut picked: BTreeMap<String, i32> = BTreeMap::new();
    for rec in recommendations {
        if budget < 1 {
            break;
        }
        let def = match skill_defs.get(&rec.skill) {
            Some(d) => d,
            None => continue,
        };
        if !skill_lock_reasons(def.available_milestone, &def.requirements, &ctx).is_empty() {
            continue;
        }
        let target = rec.level.clamp(MIN_TRAINED_SKILL_LEVEL, MAX_SKILL_LEVEL);
        let mut level = UNTRAINED_SKILL_LEVEL;
        while level < target && budget >= 1 {
            level += 1;
            budget -= 1;
        }
        if level >= MIN_TRAINED_SKILL_LEVEL {
            picked.insert(rec.skill.clone(), level);
        }
    }
    picked
}

/// Human-readable summary of an auto-pick result.
pub fn auto_pick_summary(picked: &BTreeMap<String, i32>, skill_defs: &BTreeMap<String, SkillDefinition>) -> String {
    if picked.is_empty() {
        return "No skills auto-picked.".to_string();
    }
    let parts: Vec<String> = picked.iter().map(|(sid, lvl)| {
        let name = skill_defs.get(sid).map(|d| d.name.as_str()).unwrap_or(sid.as_str());
        format!("{name} {lvl}")
    }).collect();
    format!("Auto-picked: {}.", parts.join(", "))
}

// ---------------------------------------------------------------------------
// Derived stats
// ---------------------------------------------------------------------------

/// Set HP, AC, attack bonus, saves, and class abilities on `character`.
pub fn compute_derived_stats(character: &mut Character, hit_die_roll: i32) {
    let cls = character.character_class.clone();
    let con_mod = character.attr_mods.get("con").copied().unwrap_or(0);
    let dex_mod = character.attr_mods.get("dex").copied().unwrap_or(0);
    character.base_max_hp = (hit_die_roll + con_mod).max(1);
    character.max_hp = character.base_max_hp;
    character.hp = character.max_hp;
    character.ac = 10 + dex_mod;
    character.attack_bonus = class_attack_bonus(&cls);
    let con = character.attributes.get("con").copied().unwrap_or(0);
    let str_ = character.attributes.get("str").copied().unwrap_or(0);
    let int_ = character.attributes.get("int").copied().unwrap_or(0);
    let dex = character.attributes.get("dex").copied().unwrap_or(0);
    let wis = character.attributes.get("wis").copied().unwrap_or(0);
    let cha = character.attributes.get("cha").copied().unwrap_or(0);
    character.physical_save = 15 - con.max(str_);
    character.evasion_save = 15 - dex.max(int_);
    character.mental_save = 15 - wis.max(cha);
    // Class abilities
    use serde_json::{json, Map};
    let abilities: Map<String, serde_json::Value> = match cls.as_str() {
        "warrior" => {
            let mut m = Map::new();
            m.insert("veterans_luck_used".to_string(), json!(false));
            m.insert("killing_blow_level".to_string(), json!(1));
            m
        }
        "expert" => {
            let mut m = Map::new();
            m.insert("masterful_expertise_used".to_string(), json!(false));
            m
        }
        "adventurer" => {
            let mut m = Map::new();
            m.insert("veterans_luck_used".to_string(), json!(false));
            m.insert("partial_expertise_used".to_string(), json!(false));
            m
        }
        _ => Map::new(),
    };
    character.class_abilities = abilities;
}

/// Equip a starting kit and fold armor/shield into AC.
pub fn apply_kit(character: &mut Character, kit: &EquipmentKit) {
    use serde_json::Value;
    character.equipment = kit.items.iter().filter_map(|item| {
        serde_json::to_value(item).ok().and_then(|v| {
            if let Value::Object(m) = v { Some(m) } else { None }
        })
    }).collect();

    let mut armor_ac: Option<i32> = None;
    let mut shield_bonus = 0i32;
    for raw in &character.equipment {
        if let Ok(item) = serde_json::from_value::<InventoryItemRecord>(Value::Object(raw.clone())) {
            match item.item_type.as_deref() {
                Some("armor") => {
                    if let Some(ac) = item.ac {
                        armor_ac = Some(armor_ac.unwrap_or(0).max(ac));
                    } else if let Some(bonus) = item.ac_bonus {
                        let v = bonus + 10;
                        armor_ac = Some(armor_ac.unwrap_or(0).max(v));
                    }
                }
                Some("shield") => {
                    shield_bonus += item.ac_bonus.unwrap_or(0);
                }
                _ => {}
            }
        }
    }
    if let Some(base) = armor_ac {
        character.ac = base + shield_bonus;
    } else {
        character.ac += shield_bonus;
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_attributes(submission: &CharacterDraftSubmission) -> Result<(), String> {
    let attrs = &submission.attributes;
    let expected: std::collections::BTreeSet<&str> = ATTRIBUTES.iter().copied().collect();
    let got: std::collections::BTreeSet<&str> = attrs.keys().map(|s| s.as_str()).collect();
    if got != expected {
        return Err(format!("Attributes must assign exactly {}.", ATTRIBUTES.join(", ")));
    }
    let mut values: Vec<i32> = attrs.values().copied().collect();
    values.sort_unstable();
    match submission.attribute_method {
        AttributeMethod::Roll => {
            let rolled = submission.rolled_scores.as_ref()
                .ok_or("The 'roll' method requires the six rolled scores.")?;
            if rolled.len() != ATTRIBUTES.len() {
                return Err("The 'roll' method requires the six rolled scores.".to_string());
            }
            for &s in rolled {
                if !(3..=18).contains(&s) {
                    return Err(format!("Rolled score {s} is out of range 3-18."));
                }
            }
            let mut rs = rolled.clone();
            rs.sort_unstable();
            if values != rs {
                return Err("Assigned attributes must use exactly the rolled scores.".to_string());
            }
        }
        AttributeMethod::StandardArray => {
            let mut sa = STANDARD_ARRAY.to_vec();
            sa.sort_unstable();
            if values != sa {
                return Err(format!(
                    "Assigned attributes must use exactly the standard array ({}).",
                    STANDARD_ARRAY.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")
                ));
            }
        }
        AttributeMethod::PointBuy => {
            for &score in attrs.values() {
                if !(POINT_BUY_MIN..=POINT_BUY_MAX).contains(&score) {
                    return Err(format!(
                        "Point-buy scores must be {POINT_BUY_MIN}-{POINT_BUY_MAX}; got {score}."
                    ));
                }
            }
            let spent = point_buy_cost(&attrs.values().copied().collect::<Vec<_>>())?;
            if spent > POINT_BUY_BUDGET {
                return Err(format!(
                    "Point-buy spends {spent} points but the budget is {POINT_BUY_BUDGET}."
                ));
            }
        }
    }
    Ok(())
}

fn validate_skills(
    submission: &CharacterDraftSubmission,
    valid_skills: &BTreeMap<String, String>,
    skill_defs: Option<&BTreeMap<String, SkillDefinition>>,
    current_milestone: i32,
) -> Result<(), String> {
    let budget = skill_point_budget(&submission.character_class);
    let ctx = SkillDraftContext {
        character_class: submission.character_class.clone(),
        attributes: submission.attributes.clone(),
        current_milestone,
        traits: Vec::new(),
    };
    let mut spent = 0;
    for (skill_id, &level) in &submission.skills {
        if !valid_skills.is_empty() && !valid_skills.contains_key(skill_id.as_str()) {
            return Err(format!("Unknown skill '{skill_id}'."));
        }
        if !(MIN_TRAINED_SKILL_LEVEL..=MAX_SKILL_LEVEL).contains(&level) {
            return Err(format!(
                "Skill '{skill_id}' level {level} is out of range {MIN_TRAINED_SKILL_LEVEL}-{MAX_SKILL_LEVEL}."
            ));
        }
        if let Some(defs) = skill_defs {
            if let Some(def) = defs.get(skill_id.as_str()) {
                let reasons = skill_lock_reasons(def.available_milestone, &def.requirements, &ctx);
                if !reasons.is_empty() {
                    return Err(format!("Cannot take '{skill_id}': {}", reasons[0]));
                }
            }
        }
        spent += skill_cost(level);
    }
    if spent > budget {
        return Err(format!(
            "Skills spend {spent} points but {} has only {budget}.",
            submission.character_class
        ));
    }
    Ok(())
}

/// Validate a full submission.
pub fn validate_submission(
    submission: &CharacterDraftSubmission,
    valid_skills: &BTreeMap<String, String>,
    kits: &BTreeMap<String, EquipmentKit>,
    skill_defs: Option<&BTreeMap<String, SkillDefinition>>,
    current_milestone: i32,
) -> Result<(), String> {
    if submission.name.trim().is_empty() {
        return Err("Character name must not be blank.".to_string());
    }
    if !VALID_CLASSES.contains(&submission.character_class.as_str()) {
        return Err(format!(
            "'{}' is not a valid class. Choose: {}.",
            submission.character_class,
            VALID_CLASSES.join(", ")
        ));
    }
    validate_attributes(submission)?;
    validate_skills(submission, valid_skills, skill_defs, current_milestone)?;
    let kit = kits.get(&submission.kit_id)
        .ok_or_else(|| format!("Unknown equipment kit '{}'.", submission.kit_id))?;
    if kit.character_class != submission.character_class {
        return Err(format!(
            "Kit '{}' is for {}, not {}.",
            kit.name, kit.character_class, submission.character_class
        ));
    }
    Ok(())
}

/// Build a fully-initialised `Character` from a validated submission.
pub fn build_character(
    submission: &CharacterDraftSubmission,
    valid_skills: &BTreeMap<String, String>,
    kits: &BTreeMap<String, EquipmentKit>,
    hp_roll: i32,
) -> Result<Character, String> {
    validate_submission(submission, valid_skills, kits, None, CURRENT_MILESTONE)?;
    let mut character = Character::new(
        submission.name.trim(),
        &submission.character_class,
    );
    for attr in ATTRIBUTES {
        let score = submission.attributes.get(*attr).copied().unwrap_or(10);
        character.attributes.insert(attr.to_string(), score);
        character.attr_mods.insert(attr.to_string(), attr_modifier(score));
    }
    compute_derived_stats(&mut character, hp_roll);
    character.skills = submission.skills.clone();
    apply_kit(&mut character, &kits[&submission.kit_id]);
    Ok(character)
}

// ---------------------------------------------------------------------------
// Content loaders
// ---------------------------------------------------------------------------

/// Load skill id -> name map from YAML.
pub fn load_valid_skills() -> BTreeMap<String, String> {
    let path = default_content_dir().join("skills.yaml");
    if !path.exists() {
        return BTreeMap::new();
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let catalog: SkillCatalog = serde_yaml::from_str(&text).unwrap_or_default();
    catalog.skills.into_iter().map(|s| {
        let name = if s.name.is_empty() { s.id.clone() } else { s.name.clone() };
        (s.id, name)
    }).collect()
}

/// Load skill definitions keyed by id.
pub fn load_skill_definitions() -> BTreeMap<String, SkillDefinition> {
    let path = default_content_dir().join("skills.yaml");
    if !path.exists() {
        return BTreeMap::new();
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let catalog: SkillCatalog = serde_yaml::from_str(&text).unwrap_or_default();
    catalog.skills.into_iter().map(|s| (s.id.clone(), s)).collect()
}

/// Load equipment kits keyed by id.
pub fn load_kits() -> BTreeMap<String, EquipmentKit> {
    let path = default_content_dir().join("equipment_kits.yaml");
    if !path.exists() {
        return BTreeMap::new();
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let catalog: EquipmentKitCatalog = serde_yaml::from_str(&text).unwrap_or_default();
    catalog.kits.into_iter().map(|k| (k.id.clone(), k)).collect()
}

/// Load class options from classes.yaml.
pub fn load_class_options() -> Vec<ClassOption> {
    use serde_yaml::Value;
    let path = default_content_dir().join("classes.yaml");
    if !path.exists() {
        return vec![];
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let raw: serde_yaml::Value = serde_yaml::from_str(&text).unwrap_or(Value::Null);
    let classes = match raw.get("classes").and_then(|v| v.as_sequence()) {
        Some(v) => v.clone(),
        None => return vec![],
    };
    let mut options = vec![];
    for record in classes {
        let class_id = record["id"].as_str().unwrap_or("").to_string();
        if !VALID_CLASSES.contains(&class_id.as_str()) {
            continue;
        }
        let name = record["name"].as_str().unwrap_or(&class_id).to_string();
        let description = record["description"].as_str().unwrap_or("").to_string();
        let hit_die = record["hit_die"].as_i64().unwrap_or(class_hit_die(&class_id) as i64) as i32;
        let abilities: Vec<String> = record["abilities"].as_sequence()
            .map(|seq| seq.iter().filter_map(|a| {
                a.get("name").or_else(|| a.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string())
            }).collect())
            .unwrap_or_default();
        let recommended_skills: Vec<RecommendedSkill> = record["recommended_skills"]
            .as_sequence()
            .map(|seq| seq.iter().filter_map(|r| {
                serde_yaml::from_value(r.clone()).ok()
            }).collect())
            .unwrap_or_default();
        options.push(ClassOption {
            id: class_id.clone(),
            name,
            description,
            hit_die,
            skill_points_start: skill_point_budget(&class_id),
            abilities,
            recommended_skills,
        });
    }
    options
}

/// Find the closest valid skill id using difflib-like substring matching.
pub fn suggest_skill(skill_id: &str, valid_skills: &BTreeMap<String, String>) -> Option<String> {
    if valid_skills.is_empty() {
        return None;
    }
    // Simple prefix/substring match
    let lower = skill_id.to_lowercase();
    valid_skills.keys()
        .find(|k| k.starts_with(&lower) || lower.starts_with(k.as_str()))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_attrs() -> BTreeMap<String, i32> {
        let mut m = BTreeMap::new();
        for (attr, val) in ATTRIBUTES.iter().zip(STANDARD_ARRAY) {
            m.insert(attr.to_string(), *val);
        }
        m
    }

    #[test]
    fn skill_cost_from_untrained() {
        assert_eq!(skill_cost(0), 1);
        assert_eq!(skill_cost(1), 2);
        assert_eq!(skill_cost(4), 5);
    }

    #[test]
    fn point_buy_cost_valid() {
        assert_eq!(point_buy_cost(&[8, 10, 12]).unwrap(), 0 + 2 + 4);
    }

    #[test]
    fn point_buy_cost_out_of_range() {
        assert!(point_buy_cost(&[7]).is_err());
        assert!(point_buy_cost(&[16]).is_err());
    }

    #[test]
    fn validate_submission_standard_array_ok() {
        let mut attrs = BTreeMap::new();
        for (attr, val) in ATTRIBUTES.iter().zip(STANDARD_ARRAY) {
            attrs.insert(attr.to_string(), *val);
        }
        let kit = EquipmentKit {
            id: "k1".to_string(),
            name: "Scout Kit".to_string(),
            character_class: "warrior".to_string(),
            description: String::new(),
            items: vec![],
        };
        let mut kits = BTreeMap::new();
        kits.insert("k1".to_string(), kit);
        let sub = CharacterDraftSubmission {
            name: "Hero".to_string(),
            character_class: "warrior".to_string(),
            attribute_method: AttributeMethod::StandardArray,
            attributes: attrs,
            skills: BTreeMap::new(),
            kit_id: "k1".to_string(),
            rolled_scores: None,
        };
        assert!(validate_submission(&sub, &BTreeMap::new(), &kits, None, CURRENT_MILESTONE).is_ok());
    }

    #[test]
    fn validate_submission_wrong_kit_class() {
        let mut attrs = BTreeMap::new();
        for (attr, val) in ATTRIBUTES.iter().zip(STANDARD_ARRAY) {
            attrs.insert(attr.to_string(), *val);
        }
        let kit = EquipmentKit {
            id: "k1".to_string(),
            name: "Expert Kit".to_string(),
            character_class: "expert".to_string(),
            description: String::new(),
            items: vec![],
        };
        let mut kits = BTreeMap::new();
        kits.insert("k1".to_string(), kit);
        let sub = CharacterDraftSubmission {
            name: "Hero".to_string(),
            character_class: "warrior".to_string(),
            attribute_method: AttributeMethod::StandardArray,
            attributes: attrs,
            skills: BTreeMap::new(),
            kit_id: "k1".to_string(),
            rolled_scores: None,
        };
        assert!(validate_submission(&sub, &BTreeMap::new(), &kits, None, CURRENT_MILESTONE).is_err());
    }

    #[test]
    fn skill_lock_reasons_milestone() {
        let req = SkillRequirement {
            requirement_type: "class".to_string(),
            message: None,
            classes: vec![],
            attribute: None,
            min: None,
            trait_id: None,
        };
        let ctx = SkillDraftContext::new("warrior", make_attrs());
        // available_milestone(10) > current_milestone(5)
        let reasons = skill_lock_reasons(10, &[req], &ctx);
        assert!(!reasons.is_empty());
        assert!(reasons[0].contains("milestone"));
    }

    #[test]
    fn auto_pick_skills_warrior() {
        let recs = vec![
            RecommendedSkill { skill: "stab".to_string(), level: 1 },
            RecommendedSkill { skill: "survive".to_string(), level: 0 },
        ];
        let mut defs = BTreeMap::new();
        defs.insert("stab".to_string(), SkillDefinition {
            id: "stab".to_string(),
            name: "Stab".to_string(),
            description: String::new(),
            default_attribute: "str".to_string(),
            alternate_attributes: vec![],
            can_use_untrained: false,
            combat_skill: true,
            available_milestone: 1,
            category: "Combat".to_string(),
            subcategory: None,
            requirements: vec![],
        });
        defs.insert("survive".to_string(), SkillDefinition {
            id: "survive".to_string(),
            name: "Survive".to_string(),
            description: String::new(),
            default_attribute: "con".to_string(),
            alternate_attributes: vec![],
            can_use_untrained: true,
            combat_skill: false,
            available_milestone: 1,
            category: "General".to_string(),
            subcategory: None,
            requirements: vec![],
        });
        let attrs = make_attrs();
        let picked = auto_pick_skills("warrior", &attrs, &defs, &recs, CURRENT_MILESTONE);
        // warrior gets 2 skill points: stab→1 costs 2 points → stab=1, survive=nothing
        assert!(picked.contains_key("stab"));
        assert_eq!(picked["stab"], 1);
    }
}
