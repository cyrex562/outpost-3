//! Narrator — natural-language descriptions of cells, movement, and surroundings.
//!
//! Ported from `src/harsh_realm/gm/narrator.py`. Loads description templates from
//! YAML and picks among variants at random; falls back to simple descriptions
//! when templates are missing. The RNG lives behind a `RefCell` so the describe
//! methods take `&self`, matching the shared-narrator usage in Python.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::cell::{CellData, TerrainRegistry};
use crate::narrator_content::{
    AdjacentHintsDocument, FeaturePrefixMap, MovementDescriptionMap, TerrainDescriptionMap,
    TerrainDescriptionsDocument,
};

/// Generates text descriptions for cells, movement, and nearby terrain.
#[derive(Clone)]
pub struct Narrator {
    registry: TerrainRegistry,
    terrain_descs: TerrainDescriptionMap,
    first_visit_suffixes: Vec<String>,
    feature_prefixes: FeaturePrefixMap,
    movement_descs: MovementDescriptionMap,
    adjacent_hints: Vec<String>,
    /// The content directory the templates were loaded from. Retained so scenes
    /// that hold a narrator can resolve other content (e.g. the creature catalog)
    /// without separately threading the path.
    content_dir: PathBuf,
    rng: RefCell<SmallRng>,
}

impl Narrator {
    /// Create a narrator, loading templates from `templates_dir`.
    pub fn new(terrain_registry: TerrainRegistry, templates_dir: &Path) -> Self {
        Narrator::with_rng(terrain_registry, templates_dir, SmallRng::from_entropy())
    }

    /// Create a narrator with an explicit RNG (deterministic for tests).
    pub fn with_rng(terrain_registry: TerrainRegistry, templates_dir: &Path, rng: SmallRng) -> Self {
        let dir = PathBuf::from(templates_dir);
        let terrain_doc: TerrainDescriptionsDocument =
            load_doc(&dir.join("terrain_descriptions.yaml"));
        let movement_doc: crate::narrator_content::MovementDescriptionsDocument =
            load_doc(&dir.join("movement_descriptions.yaml"));
        let hints_doc: AdjacentHintsDocument = load_doc(&dir.join("adjacent_hints.yaml"));
        Narrator {
            registry: terrain_registry,
            terrain_descs: terrain_doc.descriptions,
            first_visit_suffixes: terrain_doc.first_visit_suffix,
            feature_prefixes: terrain_doc.feature_prefix,
            movement_descs: movement_doc.movement,
            adjacent_hints: hints_doc.hints,
            content_dir: dir,
            rng: RefCell::new(rng),
        }
    }

    /// The content directory templates were loaded from.
    pub fn content_dir(&self) -> &Path {
        &self.content_dir
    }

    fn terrain_name(&self, cell: &CellData) -> String {
        match self.registry.get(&cell.terrain) {
            Some(t) => t.name.clone(),
            None => title_case(&cell.terrain.replace('_', " ")),
        }
    }

    fn pick(&self, variants: &[String]) -> String {
        if variants.is_empty() {
            return String::new();
        }
        variants
            .choose(&mut *self.rng.borrow_mut())
            .cloned()
            .unwrap_or_default()
    }

    fn feature_sentence(&self, cell: &CellData) -> String {
        let Some(feature_text) = cell.features.first() else {
            return String::new();
        };
        let template = self
            .feature_prefixes
            .get(&cell.terrain)
            .or_else(|| self.feature_prefixes.get("default"))
            .cloned()
            .unwrap_or_else(|| "Nearby: {feature}.".to_string());
        let name = cell
            .data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(feature_text);
        render(&template, &[("name", name), ("feature", feature_text)])
    }

    /// Return a 1-3 sentence description of the current hex.
    pub fn describe_cell(&self, cell: &CellData, first_visit: bool) -> String {
        let base = match self.terrain_descs.get(&cell.terrain) {
            Some(variants) if !variants.is_empty() => self.pick(variants),
            _ => format!("You are in {} terrain.", self.terrain_name(cell).to_lowercase()),
        };
        let mut parts = vec![base];

        let feature = self.feature_sentence(cell);
        if !feature.is_empty() {
            parts.push(feature);
        }
        if first_visit && !self.first_visit_suffixes.is_empty() {
            parts.push(self.pick(&self.first_visit_suffixes).trim().to_string());
        }
        parts.join(" ")
    }

    /// Return a sentence or two about visible adjacent terrain.
    ///
    /// `adjacent` maps a direction name to a cell (or `None` if off-map),
    /// preserving caller order.
    pub fn describe_adjacent_features(&self, adjacent: &[(String, Option<CellData>)]) -> String {
        let mut hints: Vec<String> = Vec::new();
        for (direction, cell) in adjacent {
            let Some(cell) = cell else { continue };
            let terrain_name = self.terrain_name(cell).to_lowercase();
            let direction_lower = direction.to_lowercase();
            let direction_cap = capitalize(&direction_lower);
            let hint = if !self.adjacent_hints.is_empty() {
                render(
                    &self.pick(&self.adjacent_hints),
                    &[
                        ("direction", &direction_lower),
                        ("direction_cap", &direction_cap),
                        ("terrain", &terrain_name),
                    ],
                )
            } else {
                format!("To the {direction_lower} lies {terrain_name}.")
            };
            hints.push(hint);
        }
        if hints.is_empty() {
            return "Nothing notable is visible nearby.".to_string();
        }
        hints.join(" ")
    }

    /// Return a brief movement description (or "look" when `direction` is `None`).
    pub fn describe_movement(
        &self,
        direction: Option<&str>,
        _from_cell: &CellData,
        to_cell: &CellData,
    ) -> String {
        let direction_lower = direction.map(|d| d.to_lowercase()).unwrap_or_else(|| "here".to_string());
        let variants = self
            .movement_descs
            .get(&to_cell.terrain)
            .filter(|v| !v.is_empty())
            .or_else(|| self.movement_descs.get("default"));
        if let Some(variants) = variants {
            if !variants.is_empty() {
                return render(&self.pick(variants), &[("direction", &direction_lower)]);
            }
        }
        let terrain_name = self.terrain_name(to_cell).to_lowercase();
        match direction {
            None => format!("You are in {terrain_name}."),
            Some(_) => format!("You head {direction_lower} into {terrain_name}."),
        }
    }
}

fn load_doc<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_yaml::from_str(&text).unwrap_or_default(),
        Err(_) => T::default(),
    }
}

/// Replace `{key}` placeholders in `template` with the supplied values.
fn render(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in pairs {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}

fn title_case(text: &str) -> String {
    text.split(' ').map(capitalize).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::TerrainType;
    use crate::runtime::JsonObject;

    fn narrator() -> Narrator {
        let registry = TerrainRegistry::from_types(vec![serde_json::from_value::<TerrainType>(
            serde_json::json!({"id": "forest", "name": "Forest", "passable": true, "blocks_vision": true}),
        )
        .unwrap()]);
        // No templates dir -> all docs default to empty; exercises fallbacks.
        Narrator::with_rng(registry, Path::new("/nonexistent"), SmallRng::seed_from_u64(1))
    }

    fn cell(terrain: &str, features: Vec<&str>) -> CellData {
        let mut c: CellData = serde_json::from_value(serde_json::json!({
            "q": 0, "r": 0, "terrain": terrain,
        }))
        .unwrap();
        c.features = features.into_iter().map(str::to_string).collect();
        c
    }

    #[test]
    fn describe_cell_fallback_uses_terrain_name() {
        let n = narrator();
        let desc = n.describe_cell(&cell("forest", vec![]), false);
        assert_eq!(desc, "You are in forest terrain.");
    }

    #[test]
    fn unknown_terrain_falls_back_to_titleized_id() {
        let n = narrator();
        let desc = n.describe_cell(&cell("dead_marsh", vec![]), false);
        assert_eq!(desc, "You are in dead marsh terrain.");
    }

    #[test]
    fn feature_sentence_uses_default_template() {
        let n = narrator();
        let desc = n.describe_cell(&cell("forest", vec!["ruins"]), false);
        assert!(desc.contains("Nearby: ruins."));
    }

    #[test]
    fn adjacent_features_fallback() {
        let n = narrator();
        let adjacent = vec![
            ("north".to_string(), Some(cell("forest", vec![]))),
            ("south".to_string(), None),
        ];
        let desc = n.describe_adjacent_features(&adjacent);
        assert_eq!(desc, "To the north lies forest.");
    }

    #[test]
    fn movement_fallback_with_and_without_direction() {
        let n = narrator();
        let from = cell("forest", vec![]);
        let to = cell("forest", vec![]);
        assert_eq!(n.describe_movement(Some("north"), &from, &to), "You head north into forest.");
        assert_eq!(n.describe_movement(None, &from, &to), "You are in forest.");
    }

    #[test]
    fn render_replaces_placeholders() {
        assert_eq!(
            render("To the {direction} lies {terrain}.", &[("direction", "east"), ("terrain", "hills")]),
            "To the east lies hills."
        );
        let _ = JsonObject::new();
    }
}
