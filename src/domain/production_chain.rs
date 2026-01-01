use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::ResourceType;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductionRecipe {
    pub inputs: Vec<(ResourceType, u32)>,
    pub outputs: Vec<(ResourceType, u32)>,
    pub production_time: u8, // Turns required
}

impl ProductionRecipe {
    pub fn new(inputs: Vec<(ResourceType, u32)>, outputs: Vec<(ResourceType, u32)>, production_time: u8) -> Self {
        Self {
            inputs,
            outputs,
            production_time,
        }
    }
}

/// Defines all production recipes in the game
pub struct ProductionChains;

impl ProductionChains {
    /// Get the recipe for converting one resource to another
    pub fn get_recipe(input: ResourceType, output: ResourceType) -> Option<ProductionRecipe> {
        Self::all_recipes()
            .into_iter()
            .find(|r| {
                r.inputs.iter().any(|(t, _)| *t == input) &&
                r.outputs.iter().any(|(t, _)| *t == output)
            })
    }

    /// Get all recipes that produce a specific resource
    pub fn recipes_for_output(output: ResourceType) -> Vec<ProductionRecipe> {
        Self::all_recipes()
            .into_iter()
            .filter(|r| r.outputs.iter().any(|(t, _)| *t == output))
            .collect()
    }

    /// Get all defined production recipes
    pub fn all_recipes() -> Vec<ProductionRecipe> {
        vec![
            // Basic Refining
            ProductionRecipe::new(
                vec![(ResourceType::IronOre, 2)],
                vec![(ResourceType::Steel, 1)],
                1,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::CopperOre, 2)],
                vec![(ResourceType::Electronics, 1)],
                1,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::Oil, 3)],
                vec![(ResourceType::Fuel, 2)],
                1,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::Oil, 2)],
                vec![(ResourceType::Plastics, 1)],
                1,
            ),

            // Advanced Production
            ProductionRecipe::new(
                vec![
                    (ResourceType::Steel, 2),
                    (ResourceType::Electronics, 1),
                ],
                vec![(ResourceType::Machinery, 1)],
                2,
            ),
            ProductionRecipe::new(
                vec![
                    (ResourceType::Steel, 1),
                    (ResourceType::Titanium, 1),
                ],
                vec![(ResourceType::AdvancedComponents, 1)],
                2,
            ),
            ProductionRecipe::new(
                vec![
                    (ResourceType::Silicon, 2),
                    (ResourceType::RareMetals, 1),
                ],
                vec![(ResourceType::Electronics, 2)],
                2,
            ),

            // Construction Materials
            ProductionRecipe::new(
                vec![
                    (ResourceType::IronOre, 1),
                    (ResourceType::Water, 1),
                ],
                vec![(ResourceType::Concrete, 2)],
                1,
            ),

            // Consumer Goods
            ProductionRecipe::new(
                vec![
                    (ResourceType::Plastics, 1),
                    (ResourceType::Electronics, 1),
                ],
                vec![(ResourceType::ConsumerGoods, 2)],
                1,
            ),
            ProductionRecipe::new(
                vec![
                    (ResourceType::RareMetals, 2),
                    (ResourceType::AdvancedComponents, 1),
                ],
                vec![(ResourceType::Luxuries, 1)],
                3,
            ),

            // Medical Production
            ProductionRecipe::new(
                vec![
                    (ResourceType::Chemicals, 2),
                    (ResourceType::Water, 1),
                ],
                vec![(ResourceType::Medicine, 2)],
                2,
            ),

            // Chemical Production
            ProductionRecipe::new(
                vec![
                    (ResourceType::Oil, 2),
                    (ResourceType::Water, 1),
                ],
                vec![(ResourceType::Chemicals, 2)],
                1,
            ),
        ]
    }

    /// Get a map of resource dependencies (what can be made from what)
    pub fn dependency_map() -> HashMap<ResourceType, Vec<ResourceType>> {
        let mut map: HashMap<ResourceType, Vec<ResourceType>> = HashMap::new();

        for recipe in Self::all_recipes() {
            for (output, _) in &recipe.outputs {
                let inputs: Vec<ResourceType> = recipe.inputs.iter().map(|(t, _)| *t).collect();
                map.entry(*output).or_insert_with(Vec::new).extend(inputs);
            }
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_recipe() {
        let recipe = ProductionChains::get_recipe(ResourceType::IronOre, ResourceType::Steel);
        assert!(recipe.is_some());
        let recipe = recipe.unwrap();
        assert_eq!(recipe.inputs.len(), 1);
        assert_eq!(recipe.outputs.len(), 1);
        assert_eq!(recipe.inputs[0].0, ResourceType::IronOre);
        assert_eq!(recipe.outputs[0].0, ResourceType::Steel);
    }

    #[test]
    fn test_recipes_for_output() {
        let recipes = ProductionChains::recipes_for_output(ResourceType::Electronics);
        assert!(!recipes.is_empty());
    }

    #[test]
    fn test_dependency_map() {
        let deps = ProductionChains::dependency_map();
        assert!(deps.contains_key(&ResourceType::Steel));
        assert!(deps.get(&ResourceType::Steel).unwrap().contains(&ResourceType::IronOre));
    }

    #[test]
    fn test_all_recipes_defined() {
        let recipes = ProductionChains::all_recipes();
        assert!(recipes.len() >= 10);
    }
}
