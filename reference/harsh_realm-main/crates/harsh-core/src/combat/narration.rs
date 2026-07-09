//! Combat narration loading and attack-text builders.
//!
//! Ported from `src/harsh_realm/engine/combat/_narration.py`.

use rand::Rng;

use crate::combat_content::CombatNarrationDocument;
use crate::combat_runtime::Combatant;
use crate::engine_results::DamageResult;
use crate::packs::paths::default_content_dir;

/// Load the combat narration templates from content, or defaults if absent.
pub fn load_narration() -> CombatNarrationDocument {
    let path = default_content_dir()
        .join("templates")
        .join("combat_narration.yaml");
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_yaml::from_str(&text).unwrap_or_default(),
        Err(_) => CombatNarrationDocument::default(),
    }
}

fn choose<'a, R: Rng>(options: &'a [String], rng: &mut R) -> &'a str {
    if options.is_empty() {
        return "...";
    }
    let index = rng.gen_range(0..options.len());
    &options[index]
}

/// Pick a random narration string from the loaded templates.
fn pick_narration<R: Rng>(
    doc: &CombatNarrationDocument,
    category: &str,
    sub: &str,
    rng: &mut R,
) -> String {
    let options: &[String] = match category {
        "natural_1" => &doc.natural_1,
        "natural_20" => &doc.natural_20,
        _ => {
            let group = if category == "hit" { &doc.hit } else { &doc.miss };
            let candidate = match sub {
                "bite" => &group.bite,
                "ranged" => &group.ranged,
                "punch" => &group.punch,
                _ => &group.melee,
            };
            if candidate.is_empty() {
                &group.melee
            } else {
                candidate
            }
        }
    };
    choose(options, rng).to_string()
}

/// Determine the narration sub-category from an attacker's attack description.
fn attack_type(combatant: &Combatant) -> &'static str {
    let desc = combatant.attack_description.to_lowercase();
    if desc.contains("bite") || desc.contains("jaw") || desc.contains("fang") {
        "bite"
    } else if combatant.is_player {
        "melee"
    } else {
        "punch"
    }
}

/// Build a narrative string for an attack result.
#[allow(clippy::too_many_arguments)]
pub fn build_attack_narration<R: Rng>(
    doc: &CombatNarrationDocument,
    attacker: &Combatant,
    target: &Combatant,
    roll: i32,
    total: i32,
    hit: bool,
    natural_1: bool,
    natural_20: bool,
    damage: Option<&DamageResult>,
    rng: &mut R,
) -> String {
    if natural_1 {
        let miss_phrase = pick_narration(doc, "natural_1", "natural_1", rng);
        return format!(
            "{} {} {}. {miss_phrase} (roll 1 vs AC {})",
            attacker.display_name, attacker.attack_description, target.display_name, target.ac
        );
    }
    if natural_20 {
        let hit_phrase = pick_narration(doc, "natural_20", "natural_20", rng);
        let dmg_str = damage
            .map(|d| format!(" for {} damage", d.total))
            .unwrap_or_default();
        return format!(
            "{} {} {}. {hit_phrase} (critical!){dmg_str}.",
            attacker.display_name, attacker.attack_description, target.display_name
        );
    }

    let sub = attack_type(attacker);
    if hit {
        let phrase = pick_narration(doc, "hit", sub, rng);
        let dmg_str = damage
            .map(|d| format!(" for {} damage", d.total))
            .unwrap_or_default();
        format!(
            "{} {} {}. {phrase} (roll {roll} \u{2192} {total} vs AC {}){dmg_str}.",
            attacker.display_name, attacker.attack_description, target.display_name, target.ac
        )
    } else {
        let phrase = pick_narration(doc, "miss", sub, rng);
        format!(
            "{} {} {}. {phrase} (roll {roll} \u{2192} {total} vs AC {}).",
            attacker.display_name, attacker.attack_description, target.display_name, target.ac
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn combatant(name: &str, is_player: bool, desc: &str) -> Combatant {
        serde_json::from_value(serde_json::json!({
            "entity_id": name, "name": name, "display_name": name,
            "is_player": is_player, "initiative": 0, "hp": 5, "max_hp": 5,
            "ac": 12, "attack_bonus": 1, "damage_expr": "1d6",
            "attack_description": desc, "behavior": "aggressive", "num_attacks": 1,
        }))
        .unwrap()
    }

    #[test]
    fn attack_type_detection() {
        assert_eq!(attack_type(&combatant("wolf", false, "bites")), "bite");
        assert_eq!(attack_type(&combatant("hero", true, "attacks")), "melee");
        assert_eq!(attack_type(&combatant("goblin", false, "claws")), "punch");
    }

    #[test]
    fn narration_includes_actors_and_ac() {
        let doc = CombatNarrationDocument::default();
        let mut rng = SmallRng::seed_from_u64(1);
        let hero = combatant("Hero", true, "attacks");
        let wolf = combatant("Wolf", false, "bites");
        let n = build_attack_narration(&doc, &hero, &wolf, 15, 17, true, false, false, None, &mut rng);
        assert!(n.contains("Hero attacks Wolf"));
        assert!(n.contains("vs AC 12"));
    }
}
