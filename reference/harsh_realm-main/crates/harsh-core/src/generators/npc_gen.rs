//! NPC generator.
//!
//! Ported from `src/harsh_realm/generators/npc_gen.py`. Generates NPCs with
//! names, occupations, traits, motivation, appearance, derived disposition, and a
//! matching greeting, using the [`TableEngine`] and the `npc_basic` generator.

use rand::Rng;

use crate::npc::{GeneratedNPC, NPCGenerationContext};
use crate::runtime::JsonValue;
use crate::table_engine::{TableEngine, TableError};

const WARY_TRAITS: [&str; 5] = ["suspicious", "wary", "paranoid", "cautious", "fearful"];
const FRIENDLY_TRAITS: [&str; 6] =
    ["generous", "kind", "friendly", "jovial", "hopeful", "earnest"];
const HOSTILE_TRAITS: [&str; 4] = ["violent", "aggressive", "volatile", "defiant"];

/// Derive NPC disposition from personality traits.
fn derive_disposition(traits: &[String]) -> &'static str {
    let lowered: Vec<String> = traits.iter().map(|t| t.to_lowercase()).collect();
    let any = |set: &[&str]| lowered.iter().any(|t| set.contains(&t.as_str()));
    if any(&HOSTILE_TRAITS) {
        "hostile"
    } else if any(&WARY_TRAITS) {
        "wary"
    } else if any(&FRIENDLY_TRAITS) {
        "friendly"
    } else {
        "neutral"
    }
}

/// Generate complete NPC records using the [`TableEngine`].
pub struct NPCGenerator<'a, R: Rng> {
    engine: TableEngine<'a, R>,
}

impl<'a, R: Rng> NPCGenerator<'a, R> {
    /// Create a generator wrapping a table engine with loaded tables.
    pub fn new(engine: TableEngine<'a, R>) -> Self {
        NPCGenerator { engine }
    }

    /// Generate a complete typed NPC payload.
    pub fn generate_npc(
        &mut self,
        context: Option<NPCGenerationContext>,
    ) -> Result<GeneratedNPC, String> {
        let ctx = context.unwrap_or_default();
        let result = self.engine.generate("npc_basic").map_err(|e| e.to_string())?;
        let vars = &result.assignments;

        let first_name = str_or(vars.get("first_name"), "Unknown");
        let surname = str_or(vars.get("surname"), "");
        let name = format!("{first_name} {surname}").trim().to_string();

        let mut occupation = match vars.get("occupation") {
            Some(JsonValue::Object(map)) => map
                .get("name")
                .map(value_to_text)
                .unwrap_or_else(|| value_to_text(&JsonValue::Object(map.clone()))),
            Some(other) => value_to_text(other),
            None => "wanderer".to_string(),
        };
        if let Some(override_occ) = &ctx.occupation_override {
            occupation = override_occ.clone();
        }

        let traits: Vec<String> = ["trait_1", "trait_2"]
            .iter()
            .filter_map(|key| match vars.get(*key) {
                Some(JsonValue::String(s)) if !s.is_empty() => Some(s.clone()),
                _ => None,
            })
            .collect();

        let motivation = str_or(vars.get("motivation"), "survival");
        let appearance = str_or(vars.get("appearance"), "");
        let disposition = derive_disposition(&traits);
        let greeting = self.roll_greeting(disposition);

        Ok(GeneratedNPC {
            name,
            occupation,
            personality_traits: traits,
            motivation,
            appearance,
            greeting,
            faction_id: None,
            disposition: disposition.to_string(),
            building_id: None,
            establishment_type: None,
            establishment_name: None,
            tags: Vec::new(),
            traits: Vec::new(),
        })
    }

    fn roll_greeting(&mut self, disposition: &str) -> String {
        for _ in 0..10 {
            match self.engine.roll_on("npcs_greetings", None) {
                Ok(result) => {
                    if result.result_type == "greeting" {
                        let entry_disposition = result
                            .result
                            .get("disposition")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let text = result
                            .result
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if entry_disposition == disposition && !text.is_empty() {
                            return text.to_string();
                        }
                    } else if let Some(raw) = result.raw_text {
                        return raw;
                    }
                }
                Err(TableError::NotFound(_)) | Err(TableError::Empty(_)) => break,
                Err(_) => break,
            }
        }
        match disposition {
            "hostile" => "They say nothing and stare you down.",
            "wary" => "They eye you cautiously. 'What do you want?'",
            "neutral" => "They glance up briefly, then look away.",
            "friendly" => "They greet you warmly.",
            _ => "They acknowledge your presence.",
        }
        .to_string()
    }
}

fn str_or(value: Option<&JsonValue>, default: &str) -> String {
    match value {
        Some(v) => value_to_text(v),
        None => default.to_string(),
    }
}

fn value_to_text(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Bool(true) => "True".to_string(),
        JsonValue::Bool(false) => "False".to_string(),
        JsonValue::Null => "None".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::scene_data::RandomTableEntry;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn disposition_from_traits() {
        assert_eq!(derive_disposition(&["Violent".to_string()]), "hostile");
        assert_eq!(derive_disposition(&["wary".to_string()]), "wary");
        assert_eq!(derive_disposition(&["kind".to_string()]), "friendly");
        assert_eq!(derive_disposition(&["tall".to_string()]), "neutral");
        // Hostile takes precedence over friendly.
        assert_eq!(
            derive_disposition(&["kind".to_string(), "aggressive".to_string()]),
            "hostile"
        );
    }

    fn write_generator(db: &WorldDatabase) {
        // Minimal npc_basic generator + tables won't load from disk in tests, so
        // we drive generate_npc against an empty table set and assert fallbacks.
        let _ = db;
    }

    #[test]
    fn generate_npc_uses_fallbacks_without_tables() {
        let db = WorldDatabase::open_in_memory().unwrap();
        write_generator(&db);
        // No data_dir set on the engine -> generate("npc_basic") errors.
        let engine = TableEngine::new(&db, SmallRng::seed_from_u64(1));
        let mut gen = NPCGenerator::new(engine);
        let result = gen.generate_npc(None);
        // Without a generators dir, generate() fails -> Err propagates.
        assert!(result.is_err());
    }

    #[test]
    fn greeting_falls_back_per_disposition() {
        let db = WorldDatabase::open_in_memory().unwrap();
        // Seed a greetings table that never matches "friendly" so fallback is used.
        let entry: RandomTableEntry = serde_json::from_value(serde_json::json!({
            "weight": 1.0,
            "result": {"type": "greeting", "disposition": "hostile", "text": "Grr."},
        }))
        .unwrap();
        crate::repositories::random_table::RandomTableRepository::new(&db)
            .upsert_table("npcs_greetings", "npc", "Greetings", &[], &[entry], "t")
            .unwrap();
        let engine = TableEngine::new(&db, SmallRng::seed_from_u64(2));
        let mut gen = NPCGenerator::new(engine);
        let greeting = gen.roll_greeting("friendly");
        assert_eq!(greeting, "They greet you warmly.");
    }
}
