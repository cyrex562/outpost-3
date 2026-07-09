//! Quest content lookup.
//!
//! Quests are plain pack content (not IR-compiled), so — like loot tables and
//! creatures — they load directly from `content/xwn-core/content/quests/*.yaml`
//! rather than through the `PackRegistry`/`RuntimeContentStore` (which the GM
//! controller does not hold). The [`QuestContent`] trait keeps the service
//! decoupled from loading so tests can supply an in-memory catalog.

use std::collections::BTreeMap;
use std::path::Path;

use crate::packs::paths::default_content_dir;

use super::schema::Quest;

/// Source of quest content records keyed by id.
pub trait QuestContent {
    /// Return the quest definition for `id`, if present.
    fn get(&self, id: &str) -> Option<Quest>;
}

/// All quests loaded from a content directory.
#[derive(Debug, Clone, Default)]
pub struct QuestCatalog {
    quests: BTreeMap<String, Quest>,
}

impl QuestCatalog {
    /// An empty catalog.
    pub fn new() -> Self {
        QuestCatalog::default()
    }

    /// Build a catalog from an explicit list of quests (used in tests).
    pub fn from_quests(quests: Vec<Quest>) -> Self {
        let mut map = BTreeMap::new();
        for q in quests {
            map.insert(q.id.clone(), q);
        }
        QuestCatalog { quests: map }
    }

    /// Load every quest under `<content_dir>/quests/*.yaml`. Each file holds a
    /// YAML list of quests. Unreadable/malformed files are skipped (load never
    /// aborts), matching the loot-table loader's tolerance.
    pub fn load_from_content_dir(content_dir: &Path) -> Self {
        let mut map = BTreeMap::new();
        let quests_dir = content_dir.join("quests");
        if let Ok(entries) = std::fs::read_dir(&quests_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                if let Ok(quests) = serde_yaml::from_str::<Vec<Quest>>(&text) {
                    for q in quests {
                        map.insert(q.id.clone(), q);
                    }
                }
            }
        }
        QuestCatalog { quests: map }
    }

    /// Load from the default pack content directory.
    pub fn load_default() -> Self {
        Self::load_from_content_dir(&default_content_dir())
    }

    /// All quests, ordered by id.
    pub fn all(&self) -> impl Iterator<Item = &Quest> {
        self.quests.values()
    }

    /// The number of loaded quests.
    pub fn len(&self) -> usize {
        self.quests.len()
    }

    /// Whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.quests.is_empty()
    }
}

impl QuestContent for QuestCatalog {
    fn get(&self, id: &str) -> Option<Quest> {
        self.quests.get(id).cloned()
    }
}

impl QuestContent for &QuestCatalog {
    fn get(&self, id: &str) -> Option<Quest> {
        self.quests.get(id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_quests_indexes_by_id() {
        let cat = QuestCatalog::from_quests(vec![Quest {
            id: "a".into(),
            name: "A".into(),
            description: String::new(),
            objectives: vec![],
            reward_gold: 0,
            reward_xp: 0,
            reward_items: vec![],
            prerequisites: vec![],
            expiry_tick: None,
            faction_id: None,
        }]);
        assert_eq!(cat.len(), 1);
        assert_eq!(cat.get("a").unwrap().name, "A");
        assert!(cat.get("missing").is_none());
    }

    #[test]
    fn loads_the_starter_pack_quests_from_disk() {
        // The authored starter.yaml must parse and load (prerequisite chain intact).
        // Use a path relative to the crate so the test is CWD-independent (the
        // runtime `load_default` resolves the same dir via `current_dir`).
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/xwn-core/content");
        let cat = QuestCatalog::load_from_content_dir(&dir);
        assert!(!cat.is_empty(), "starter quests must load from the content dir");
        let recover = cat.get("recover_the_relic").expect("recover_the_relic quest");
        assert_eq!(recover.prerequisites, vec!["clear_the_crawlers".to_string()]);
        assert_eq!(recover.reward_xp, 300);
    }
}
