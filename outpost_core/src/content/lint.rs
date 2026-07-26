//! Authoring-time content checks that warn rather than reject (issue #272).
//!
//! Distinct from [`ContentError`](crate::content::ContentError), which is for
//! content that is *wrong* — a duplicate id, an unknown commodity reference — and
//! makes a pack fail to load. These are suspicious patterns that a careless edit
//! produces and a deliberate one might genuinely want, so rejecting them would
//! block legitimate content.
//!
//! The motivating case: two [`concurrent`] recipes on one building both producing
//! `power` silently double it. That may be exactly what an author meant (two
//! distinct generation processes), so it cannot be an error — but it is far more
//! often a copy-paste slip, and nothing surfaced it at all before.
//!
//! [`concurrent`]: crate::content::types::RecipeDef::concurrent

use std::collections::BTreeMap;

use super::registry::ContentRegistry;

/// A suspicious-but-legal content pattern found by [`ContentRegistry::lint`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentWarning {
    /// Two always-on recipes on the same building produce the same commodity,
    /// so its output is the sum of both — every turn, unconditionally.
    DuplicateConcurrentOutput {
        /// Building whose recipes overlap.
        building_id: String,
        /// Commodity produced by more than one always-on recipe.
        commodity_id: String,
        /// The overlapping recipe ids, in id order.
        recipe_ids: Vec<String>,
    },
    /// An always-on recipe produces the same commodity as a selectable one, so
    /// picking that recipe doubles the commodity while other choices don't.
    ConcurrentShadowsPickOne {
        /// Building whose recipes overlap.
        building_id: String,
        /// Commodity produced by both.
        commodity_id: String,
        /// The always-on recipe.
        concurrent_recipe_id: String,
        /// The selectable recipe it overlaps with.
        pick_one_recipe_id: String,
    },
    /// A recipe sets both `line` and `concurrent` (issue #272). `line` wins —
    /// the recipe is a selectable alternative within that line — so the
    /// `concurrent` flag has no effect and misdescribes the recipe.
    ConcurrentIgnoredOnLinedRecipe {
        /// Building the recipe belongs to.
        building_id: String,
        /// The recipe carrying both flags.
        recipe_id: String,
        /// The line it was placed on, which takes precedence.
        line: String,
    },
}

impl std::fmt::Display for ContentWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateConcurrentOutput {
                building_id,
                commodity_id,
                recipe_ids,
            } => write!(
                f,
                "building '{building_id}': always-on recipes {} all produce \
                 '{commodity_id}', so its output is their sum",
                recipe_ids.join(", ")
            ),
            Self::ConcurrentShadowsPickOne {
                building_id,
                commodity_id,
                concurrent_recipe_id,
                pick_one_recipe_id,
            } => write!(
                f,
                "building '{building_id}': always-on recipe \
                 '{concurrent_recipe_id}' and selectable recipe \
                 '{pick_one_recipe_id}' both produce '{commodity_id}', so \
                 selecting that recipe doubles it"
            ),
            Self::ConcurrentIgnoredOnLinedRecipe {
                building_id,
                recipe_id,
                line,
            } => write!(
                f,
                "building '{building_id}': recipe '{recipe_id}' sets both \
                 `concurrent` and `line: {line}`; the line wins, so the \
                 recipe is selectable rather than always-on"
            ),
        }
    }
}

impl ContentRegistry {
    /// Report suspicious authoring patterns in the loaded content (issue #272).
    ///
    /// Never fails and never rejects: every finding is a warning for a human to
    /// judge. An empty result means nothing suspicious was spotted, not that the
    /// content is correct.
    ///
    /// Findings are sorted so output is stable across runs — `ContentRegistry`
    /// stores recipes in a `HashMap`, so anything derived from iteration order
    /// has to be sorted explicitly.
    #[must_use]
    pub fn lint(&self) -> Vec<ContentWarning> {
        let mut warnings = Vec::new();

        // building id → commodity id → recipe ids producing it.
        let mut concurrent_outputs: BTreeMap<&str, BTreeMap<&str, Vec<&str>>> = BTreeMap::new();
        let mut pick_one_outputs: BTreeMap<&str, BTreeMap<&str, Vec<&str>>> = BTreeMap::new();

        for recipe in self.recipes() {
            let target = if recipe.concurrent {
                &mut concurrent_outputs
            } else {
                &mut pick_one_outputs
            };
            let per_building = target.entry(recipe.building.as_str()).or_default();
            for output in &recipe.outputs {
                per_building
                    .entry(output.id.as_str())
                    .or_default()
                    .push(recipe.id.as_str());
            }
        }

        for (building_id, per_commodity) in &concurrent_outputs {
            for (commodity_id, recipe_ids) in per_commodity {
                // Count *distinct* recipes. A single recipe that lists the same
                // commodity twice in its own `outputs` would otherwise be
                // reported as two recipes overlapping, which is a different —
                // and wrong — diagnosis to send an author chasing.
                let mut ids: Vec<String> = recipe_ids.iter().map(|s| (*s).to_owned()).collect();
                ids.sort();
                ids.dedup();
                if ids.len() > 1 {
                    warnings.push(ContentWarning::DuplicateConcurrentOutput {
                        building_id: (*building_id).to_owned(),
                        commodity_id: (*commodity_id).to_owned(),
                        recipe_ids: ids,
                    });
                }

                // Overlap with a selectable recipe on the same building.
                let Some(pick_one) = pick_one_outputs
                    .get(building_id)
                    .and_then(|m| m.get(commodity_id))
                else {
                    continue;
                };
                let mut concurrent_ids: Vec<&str> = recipe_ids.clone();
                concurrent_ids.sort_unstable();
                concurrent_ids.dedup();
                let mut pick_one_ids: Vec<&str> = pick_one.clone();
                pick_one_ids.sort_unstable();
                pick_one_ids.dedup();
                for c in &concurrent_ids {
                    for p in &pick_one_ids {
                        warnings.push(ContentWarning::ConcurrentShadowsPickOne {
                            building_id: (*building_id).to_owned(),
                            commodity_id: (*commodity_id).to_owned(),
                            concurrent_recipe_id: (*c).to_owned(),
                            pick_one_recipe_id: (*p).to_owned(),
                        });
                    }
                }
            }
        }

        // `line` takes precedence over `concurrent` in `lines_for_building`, so
        // a recipe carrying both is not always-on despite saying it is.
        for recipe in self.recipes() {
            if let (true, Some(line)) = (recipe.concurrent, recipe.line.as_ref()) {
                warnings.push(ContentWarning::ConcurrentIgnoredOnLinedRecipe {
                    building_id: recipe.building.clone(),
                    recipe_id: recipe.id.clone(),
                    line: line.clone(),
                });
            }
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{BuildingCategory, BuildingDef, Ingredient, RecipeDef};

    fn building(id: &str) -> BuildingDef {
        BuildingDef {
            id: id.into(),
            name: id.into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
        }
    }

    fn recipe(id: &str, b: &str, concurrent: bool, outputs: &[(&str, f64)]) -> RecipeDef {
        RecipeDef {
            id: id.into(),
            name: id.into(),
            building: b.into(),
            cycle_sols: 1,
            inputs: vec![],
            outputs: outputs
                .iter()
                .map(|(id, quantity)| Ingredient {
                    id: (*id).into(),
                    quantity: *quantity,
                })
                .collect(),
            concurrent,
            line: None,
            power_draw: 0.0,
        }
    }

    /// The `line` field silently overrides `concurrent` (issue #272), so an
    /// author who sets both gets a selectable recipe while believing they
    /// authored an always-on one. Say so rather than letting it pass.
    #[test]
    fn a_recipe_setting_both_line_and_concurrent_is_flagged() {
        let mut reg = ContentRegistry::default();
        reg.insert_building(building("complex"));
        let mut lined = recipe("confused", "complex", true, &[("metal", 1.0)]);
        lined.line = Some("foundry".into());
        reg.insert_recipe(lined);
        // A plain always-on recipe on the same building must NOT be flagged.
        reg.insert_recipe(recipe("honest", "complex", true, &[("power", 1.0)]));

        let warnings = reg.lint();
        let flagged: Vec<&ContentWarning> = warnings
            .iter()
            .filter(|w| matches!(w, ContentWarning::ConcurrentIgnoredOnLinedRecipe { .. }))
            .collect();
        assert_eq!(
            flagged.len(),
            1,
            "expected exactly one finding: {flagged:?}"
        );
        assert!(
            format!("{}", flagged[0]).contains("confused"),
            "wrong recipe flagged: {}",
            flagged[0]
        );
    }

    #[test]
    fn clean_content_produces_no_warnings() {
        let mut reg = ContentRegistry::default();
        reg.insert_building(building("hq"));
        reg.insert_recipe(recipe("hq_power", "hq", true, &[("power", 5.0)]));
        reg.insert_recipe(recipe("hq_water", "hq", true, &[("water", 5.0)]));
        assert!(reg.lint().is_empty(), "distinct outputs are fine");
    }

    /// The motivating case: two always-on recipes producing the same commodity
    /// double it silently, every turn.
    #[test]
    fn two_concurrent_recipes_sharing_an_output_are_flagged() {
        let mut reg = ContentRegistry::default();
        reg.insert_building(building("hq"));
        reg.insert_recipe(recipe("hq_solar", "hq", true, &[("power", 5.0)]));
        reg.insert_recipe(recipe("hq_rtg", "hq", true, &[("power", 3.0)]));

        let warnings = reg.lint();
        assert_eq!(warnings.len(), 1, "expected one finding, got {warnings:?}");
        assert_eq!(
            warnings[0],
            ContentWarning::DuplicateConcurrentOutput {
                building_id: "hq".into(),
                commodity_id: "power".into(),
                recipe_ids: vec!["hq_rtg".into(), "hq_solar".into()],
            }
        );
        // The message has to name the building, the commodity, and both recipes,
        // or an author can't act on it.
        let text = warnings[0].to_string();
        for needle in ["hq", "power", "hq_rtg", "hq_solar"] {
            assert!(text.contains(needle), "{needle:?} missing from {text:?}");
        }
    }

    /// An always-on recipe overlapping a *selectable* one is conditional
    /// doubling: it depends on which recipe the player picked.
    #[test]
    fn a_concurrent_recipe_shadowing_a_pick_one_is_flagged() {
        let mut reg = ContentRegistry::default();
        reg.insert_building(building("refinery"));
        reg.insert_recipe(recipe("vent_heat", "refinery", true, &[("power", 2.0)]));
        reg.insert_recipe(recipe("burn_fuel", "refinery", false, &[("power", 10.0)]));

        let warnings = reg.lint();
        assert_eq!(
            warnings,
            vec![ContentWarning::ConcurrentShadowsPickOne {
                building_id: "refinery".into(),
                commodity_id: "power".into(),
                concurrent_recipe_id: "vent_heat".into(),
                pick_one_recipe_id: "burn_fuel".into(),
            }]
        );
    }

    /// Two *selectable* recipes sharing an output are not suspicious at all —
    /// that's the whole point of alternatives, and only one runs at a time.
    #[test]
    fn two_pick_one_recipes_sharing_an_output_are_not_flagged() {
        let mut reg = ContentRegistry::default();
        reg.insert_building(building("refinery"));
        reg.insert_recipe(recipe("route_a", "refinery", false, &[("metal", 1.0)]));
        reg.insert_recipe(recipe("route_b", "refinery", false, &[("metal", 2.0)]));
        assert!(reg.lint().is_empty());
    }

    /// One recipe listing the same commodity twice in its own `outputs` is not
    /// two recipes overlapping — reporting it that way would send an author
    /// hunting for a second recipe that doesn't exist.
    #[test]
    fn a_single_recipe_listing_one_output_twice_is_not_a_recipe_overlap() {
        let mut reg = ContentRegistry::default();
        reg.insert_building(building("hq"));
        reg.insert_recipe(recipe(
            "hq_power",
            "hq",
            true,
            &[("power", 5.0), ("power", 3.0)],
        ));
        assert!(
            reg.lint().is_empty(),
            "one recipe is one recipe, however it lists its outputs: {:?}",
            reg.lint()
        );
    }

    /// Same commodity, *different* buildings: not an overlap.
    #[test]
    fn recipes_on_different_buildings_do_not_overlap() {
        let mut reg = ContentRegistry::default();
        reg.insert_building(building("hq"));
        reg.insert_building(building("array"));
        reg.insert_recipe(recipe("hq_power", "hq", true, &[("power", 5.0)]));
        reg.insert_recipe(recipe("array_power", "array", true, &[("power", 5.0)]));
        assert!(reg.lint().is_empty());
    }
}
