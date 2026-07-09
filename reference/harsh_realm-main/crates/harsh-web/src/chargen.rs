//! Stateless character-creation option assembly (no world needed).
//!
//! These read authored content from the built-in content directory and shape it
//! into the JSON the character-creation UI expects.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

use harsh_core::character_build::{
    auto_pick_skills, load_class_options, load_kits, load_skill_definitions, point_buy_cost,
    ATTRIBUTES, CURRENT_MILESTONE, POINT_BUY_BUDGET, POINT_BUY_MAX, POINT_BUY_MIN, STANDARD_ARRAY,
};

/// Assemble the `/api/character/options` payload.
pub fn build_character_options() -> Value {
    let classes = load_class_options();
    let skill_defs = load_skill_definitions();
    let skills: Vec<_> = skill_defs.values().cloned().collect();
    let kits = load_kits();
    let kits_vec: Vec<_> = kits.values().cloned().collect();

    let costs: Map<String, Value> = (POINT_BUY_MIN..=POINT_BUY_MAX)
        .map(|score| {
            let cost = point_buy_cost(&[score]).unwrap_or(0);
            (score.to_string(), json!(cost))
        })
        .collect();

    json!({
        "attribute_order": ATTRIBUTES,
        "methods": ["roll", "standard_array", "point_buy"],
        "standard_array": STANDARD_ARRAY,
        "point_buy": {
            "budget": POINT_BUY_BUDGET,
            "min_score": POINT_BUY_MIN,
            "max_score": POINT_BUY_MAX,
            "costs": costs,
        },
        "classes": classes,
        "skills": skills,
        "kits": kits_vec,
        "current_milestone": CURRENT_MILESTONE,
    })
}

/// Compute auto-picked skills for a class + attribute spread.
pub fn compute_auto_pick(class: &str, attributes: &BTreeMap<String, i32>) -> BTreeMap<String, i32> {
    let skill_defs = load_skill_definitions();
    let recommendations = load_class_options()
        .into_iter()
        .find(|c| c.id == class)
        .map(|c| c.recommended_skills)
        .unwrap_or_default();
    auto_pick_skills(class, attributes, &skill_defs, &recommendations, CURRENT_MILESTONE)
}
