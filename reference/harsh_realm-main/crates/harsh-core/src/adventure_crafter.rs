//! Adventure Crafter — plotline management and themed scene generation.
//!
//! Ported from `src/harsh_realm/engine/adventure_crafter.py`. The theme,
//! character, and plot tables load from the default content dir (the Python
//! module cached them globally; here they load once per crafter instance, with
//! graceful fallback to defaults if a file is missing).

use std::path::Path;

use rand::Rng;
use uuid::Uuid;

use crate::db::WorldDatabase;
use crate::oracle_models::{
    AdventureCrafterEntriesDocument, AdventureCrafterThemesDocument, Plotline, PlotlineRecord,
};
use crate::packs::paths::default_content_dir;
use crate::repositories::oracle::OracleRepository;
use crate::scene_data::AdventureScene;

/// Manages plotlines and generates themed scenes.
pub struct AdventureCrafter<'a, R: Rng> {
    repo: OracleRepository<'a>,
    rng: R,
    themes: AdventureCrafterThemesDocument,
    characters: AdventureCrafterEntriesDocument,
    plots: AdventureCrafterEntriesDocument,
}

fn short_id() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}

fn load_doc<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_yaml::from_str(&text).unwrap_or_default(),
        Err(_) => T::default(),
    }
}

fn plotline_from_record(row: PlotlineRecord) -> Plotline {
    Plotline {
        id: row.id,
        title: row.title,
        theme: row.theme,
        status: row.status,
        scenes: row.scenes,
    }
}

impl<'a, R: Rng> AdventureCrafter<'a, R> {
    /// Create a crafter over a world database with an explicit RNG.
    pub fn new(db: &'a WorldDatabase, rng: R) -> Self {
        let oracle_dir = default_content_dir().join("tables").join("oracle");
        AdventureCrafter {
            repo: OracleRepository::new(db),
            rng,
            themes: load_doc(&oracle_dir.join("ac_themes.yaml")),
            characters: load_doc(&oracle_dir.join("ac_characters.yaml")),
            plots: load_doc(&oracle_dir.join("ac_plots.yaml")),
        }
    }

    /// Create a new plotline with the given theme (normalized / defaulted).
    pub fn create_plotline(&self, title: &str, theme: &str) -> Result<Plotline, String> {
        let theme = self.normalize_theme(theme);
        let plotline_id = short_id();
        self.repo.create_plotline(&plotline_id, title, &theme)?;
        Ok(Plotline {
            id: plotline_id,
            title: title.to_string(),
            theme,
            status: "active".to_string(),
            scenes: Vec::new(),
        })
    }

    /// List plotlines, optionally filtered by status.
    pub fn list_plotlines(&self, status: Option<&str>) -> Result<Vec<Plotline>, String> {
        Ok(self
            .repo
            .list_plotlines(status)?
            .into_iter()
            .map(plotline_from_record)
            .collect())
    }

    /// Get a plotline by id or prefix.
    pub fn get_plotline(&self, plotline_id: &str) -> Result<Option<Plotline>, String> {
        Ok(self.repo.find_plotline(plotline_id)?.map(plotline_from_record))
    }

    /// Advance a plotline by generating a themed scene.
    pub fn advance_plotline(&mut self, plotline_id: &str) -> Result<Option<AdventureScene>, String> {
        let Some(mut plotline) = self.get_plotline(plotline_id)? else {
            return Ok(None);
        };
        if plotline.status != "active" {
            return Ok(None);
        }
        let scene = self.generate_scene(&plotline.theme);
        plotline.scenes.push(scene.clone());
        self.repo
            .update_plotline_scenes(&plotline.id, &plotline.scenes)?;
        Ok(Some(scene))
    }

    /// Mark a plotline complete by exact or unique prefix id.
    pub fn resolve_plotline(&self, plotline_id: &str) -> Result<bool, String> {
        let Some(plotline) = self.get_plotline(plotline_id)? else {
            return Ok(false);
        };
        self.repo.update_plotline_status(&plotline.id, "completed")?;
        Ok(true)
    }

    fn normalize_theme(&self, theme: &str) -> String {
        for entry in &self.themes.themes {
            if entry.name.eq_ignore_ascii_case(theme) {
                return entry.name.clone();
            }
        }
        // Unknown theme -> default to the first defined theme, else the input.
        self.themes
            .themes
            .first()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| theme.to_string())
    }

    fn generate_scene(&mut self, theme: &str) -> AdventureScene {
        let theme_elements: Vec<String> = self
            .themes
            .themes
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(theme))
            .map(|t| t.elements.clone())
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| vec!["unknown".to_string()]);

        let element = self.choose(&theme_elements, "unknown");
        let character = self.choose(&self.characters.entries.clone(), "a stranger");
        let plot = self.choose(&self.plots.entries.clone(), "something happens");

        let narration = self
            .themes
            .narration_template
            .replace("{theme}", theme)
            .replace("{element}", &element)
            .replace("{character}", &character)
            .replace("{plot}", &plot);

        AdventureScene {
            theme: theme.to_string(),
            element,
            character,
            plot,
            narration,
        }
    }

    fn choose(&mut self, options: &[String], fallback: &str) -> String {
        if options.is_empty() {
            return fallback.to_string();
        }
        let index = self.rng.gen_range(0..options.len());
        options[index].clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle_models::AdventureCrafterTheme;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn crafter(db: &WorldDatabase) -> AdventureCrafter<'_, SmallRng> {
        let mut c = AdventureCrafter::new(db, SmallRng::seed_from_u64(4));
        // Inject deterministic tables regardless of on-disk content.
        c.themes = AdventureCrafterThemesDocument {
            themes: vec![AdventureCrafterTheme {
                name: "Mystery".into(),
                elements: vec!["a clue".into()],
            }],
            narration_template: "[{theme}] {character}. {plot}. (Focus: {element})".into(),
        };
        c.characters = AdventureCrafterEntriesDocument {
            entries: vec!["the smith".into()],
        };
        c.plots = AdventureCrafterEntriesDocument {
            entries: vec!["a betrayal".into()],
        };
        c
    }

    #[test]
    fn create_normalizes_theme_case() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let c = crafter(&db);
        let plotline = c.create_plotline("The Heist", "mystery").unwrap();
        assert_eq!(plotline.theme, "Mystery");
        assert_eq!(plotline.status, "active");
    }

    #[test]
    fn unknown_theme_defaults_to_first() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let c = crafter(&db);
        let plotline = c.create_plotline("X", "nonexistent").unwrap();
        assert_eq!(plotline.theme, "Mystery");
    }

    #[test]
    fn advance_generates_and_persists_scene() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let mut c = crafter(&db);
        let plotline = c.create_plotline("The Heist", "Mystery").unwrap();
        let scene = c.advance_plotline(&plotline.id).unwrap().unwrap();
        assert_eq!(scene.theme, "Mystery");
        assert_eq!(scene.element, "a clue");
        assert_eq!(scene.character, "the smith");
        assert_eq!(scene.plot, "a betrayal");
        assert_eq!(scene.narration, "[Mystery] the smith. a betrayal. (Focus: a clue)");
        // Persisted on the plotline.
        let reloaded = c.get_plotline(&plotline.id).unwrap().unwrap();
        assert_eq!(reloaded.scenes.len(), 1);
    }

    #[test]
    fn resolve_marks_completed() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let mut c = crafter(&db);
        let plotline = c.create_plotline("The Heist", "Mystery").unwrap();
        assert!(c.resolve_plotline(&plotline.id).unwrap());
        // Completed plotlines cannot advance.
        assert!(c.advance_plotline(&plotline.id).unwrap().is_none());
        assert!(!c.resolve_plotline("ghost").unwrap());
    }
}
