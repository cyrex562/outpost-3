//! Content-pack loading for authored game data.
//!
//! All authored records (buildings, commodities, recipes, tech nodes, events)
//! are defined in YAML content packs under `content/` and loaded at runtime —
//! never hardcoded in the kernel (see `docs/DESIGN.md §14`).
//!
//! # Usage
//!
//! The caller reads pack files from disk and passes their raw bytes here:
//!
//! ```rust
//! use outpost_core::content::PackLoader;
//!
//! let manifest = "id: test\nname: Test\nversion: \"0.1.0\"";
//! let commodity = "- id: water\n  name: Water\n  category: consumable\n  base_value: 5.0";
//! let files = &[("pack.yaml", manifest), ("commodities.yaml", commodity)];
//! let registry = PackLoader::load(files).expect("pack loads");
//! assert!(registry.commodity("water").is_some());
//! ```

pub mod error;
pub mod lint;
pub mod loader;
pub mod registry;
pub mod types;

pub use error::ContentError;
pub use lint::ContentWarning;
pub use loader::PackLoader;
pub use registry::ContentRegistry;
pub use types::{
    BuildingCategory, BuildingDef, CommodityDef, CommodityTier, DefaultAction, DefaultDirectiveDef,
    Ingredient, Phase, RecipeDef, ResourceDef, ResourceKind,
};

#[cfg(test)]
mod tests {
    use super::*;

    // ── Embedded sample pack ──────────────────────────────────────────────

    const PACK_MANIFEST: &str = r#"
id: base-colony
name: Base Colony Pack
version: "0.1.0"
description: Minimal sample pack for testing.
"#;

    const COMMODITIES: &str = r#"
- id: water
  name: Water
  description: Essential for all life.
  category: consumable
  phase: liquid
  base_value: 5.0

- id: iron_ore
  name: Iron Ore
  description: Raw iron ore.
  category: metallic_ore
  base_value: 10.0

- id: iron_plate
  name: Iron Plate
  description: Refined iron sheeting.
  category: metal
  base_value: 25.0
"#;

    const RECIPES: &str = r#"
- id: smelt_iron
  name: Smelt Iron
  building: smelter
  cycle_sols: 2
  inputs:
    - id: iron_ore
      quantity: 2.0
  outputs:
    - id: iron_plate
      quantity: 1.0
"#;

    const BUILDINGS: &str = r#"
- id: smelter
  name: Smelter
  description: Converts raw ore into metal plate.
  category: production
  power_delta: -10.0
  worker_slots: 2
  construction_cost:
    - id: iron_plate
      quantity: 20.0
"#;

    fn sample_files() -> Vec<(&'static str, &'static str)> {
        vec![
            ("pack.yaml", PACK_MANIFEST),
            ("commodities.yaml", COMMODITIES),
            ("recipes.yaml", RECIPES),
            ("buildings.yaml", BUILDINGS),
        ]
    }

    // ── Valid-pack tests ──────────────────────────────────────────────────

    #[test]
    fn valid_pack_loads_successfully() {
        let reg = PackLoader::load(&sample_files()).expect("sample pack should load");
        assert_eq!(reg.manifest().id, "base-colony");
    }

    #[test]
    fn commodities_are_queryable() {
        let reg = PackLoader::load(&sample_files()).unwrap();
        assert!(reg.commodity("water").is_some());
        assert!(reg.commodity("iron_ore").is_some());
        assert!(reg.commodity("iron_plate").is_some());
        assert!(reg.commodity("unobtanium").is_none());
    }

    #[test]
    fn recipes_are_queryable() {
        let reg = PackLoader::load(&sample_files()).unwrap();
        let recipe = reg.recipe("smelt_iron").expect("smelt_iron recipe present");
        assert_eq!(recipe.building, "smelter");
        assert_eq!(recipe.cycle_sols, 2);
        assert_eq!(recipe.inputs.len(), 1);
        assert_eq!(recipe.outputs.len(), 1);
    }

    #[test]
    fn buildings_are_queryable() {
        let reg = PackLoader::load(&sample_files()).unwrap();
        let b = reg.building("smelter").expect("smelter building present");
        assert_eq!(b.category, BuildingCategory::Production);
        assert_eq!(b.worker_slots, 2);
    }

    #[test]
    fn commodity_count_matches_yaml() {
        let reg = PackLoader::load(&sample_files()).unwrap();
        assert_eq!(reg.commodities().count(), 3);
    }

    // ── Malformed-pack error tests ────────────────────────────────────────

    #[test]
    fn missing_manifest_returns_error() {
        let files: Vec<(&str, &str)> = vec![("commodities.yaml", COMMODITIES)];
        let err = PackLoader::load(&files).unwrap_err();
        assert!(
            matches!(err, ContentError::MissingManifest),
            "expected MissingManifest, got {err:?}"
        );
    }

    #[test]
    fn invalid_yaml_returns_parse_error() {
        let files = vec![
            ("pack.yaml", PACK_MANIFEST),
            ("commodities.yaml", ": not: valid: yaml: [[["),
        ];
        let err = PackLoader::load(&files).unwrap_err();
        assert!(
            matches!(err, ContentError::ParseError { .. }),
            "expected ParseError, got {err:?}"
        );
    }

    #[test]
    fn duplicate_commodity_id_returns_error() {
        let dupe = r#"
- id: water
  name: Water
  category: consumable
  base_value: 5.0
- id: water
  name: Water Again
  category: consumable
  base_value: 6.0
"#;
        let files = vec![("pack.yaml", PACK_MANIFEST), ("commodities.yaml", dupe)];
        let err = PackLoader::load(&files).unwrap_err();
        assert!(
            matches!(err, ContentError::DuplicateId { ref id, .. } if id == "water"),
            "expected DuplicateId for 'water', got {err:?}"
        );
    }

    #[test]
    fn recipe_referencing_unknown_commodity_returns_error() {
        let bad_recipe = r#"
- id: make_unobtanium
  name: Make Unobtanium
  building: lab
  inputs:
    - id: water
      quantity: 1.0
  outputs:
    - id: unobtanium
      quantity: 1.0
"#;
        let files = vec![
            ("pack.yaml", PACK_MANIFEST),
            ("commodities.yaml", COMMODITIES),
            ("recipes.yaml", bad_recipe),
        ];
        let err = PackLoader::load(&files).unwrap_err();
        assert!(
            matches!(
                err,
                ContentError::UnknownCommodityRef { ref commodity_id, .. }
                    if commodity_id == "unobtanium"
            ),
            "expected UnknownCommodityRef for 'unobtanium', got {err:?}"
        );
    }

    // ── Multi-pack merge tests ────────────────────────────────────────────

    #[test]
    fn later_pack_wins_on_id_collision() {
        let manifest2 = r#"
id: override-pack
name: Override Pack
version: "0.1.0"
"#;
        let override_commodity = r#"
- id: water
  name: Purified Water
  category: consumable
  base_value: 99.0
"#;

        let pack1 = PackLoader::load(&sample_files()).unwrap();
        let pack2 = PackLoader::load(&[
            ("pack.yaml", manifest2),
            ("commodities.yaml", override_commodity),
        ])
        .unwrap();

        let mut combined = pack1;
        combined.merge(pack2);

        let water = combined.commodity("water").unwrap();
        assert_eq!(water.name, "Purified Water");
        assert!((water.base_value - 99.0).abs() < f64::EPSILON);
        // Commodities from pack1 that are not overridden still present.
        assert!(combined.commodity("iron_ore").is_some());
    }

    #[test]
    fn merged_registry_manifest_reflects_last_pack() {
        let manifest2 = r#"
id: addon-pack
name: Addon Pack
version: "0.2.0"
"#;
        let pack1 = PackLoader::load(&sample_files()).unwrap();
        let pack2 = PackLoader::load(&[("pack.yaml", manifest2)]).unwrap();

        let mut combined = pack1;
        combined.merge(pack2);

        assert_eq!(combined.manifest().id, "addon-pack");
    }
}
