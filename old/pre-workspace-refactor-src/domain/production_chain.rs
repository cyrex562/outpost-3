use super::ResourceType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductionRecipe {
    pub inputs: Vec<(ResourceType, u32)>,
    pub outputs: Vec<(ResourceType, u32)>,
    pub production_time: u8, // Turns required
}

impl ProductionRecipe {
    pub fn new(
        inputs: Vec<(ResourceType, u32)>,
        outputs: Vec<(ResourceType, u32)>,
        production_time: u8,
    ) -> Self {
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
        Self::all_recipes().into_iter().find(|r| {
            r.inputs.iter().any(|(t, _)| *t == input) && r.outputs.iter().any(|(t, _)| *t == output)
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
                vec![(ResourceType::Steel, 2), (ResourceType::Electronics, 1)],
                vec![(ResourceType::Machinery, 1)],
                2,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::Steel, 1), (ResourceType::Titanium, 1)],
                vec![(ResourceType::AdvancedComponents, 1)],
                2,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::Silicon, 2), (ResourceType::RareMetals, 1)],
                vec![(ResourceType::Electronics, 2)],
                2,
            ),
            // Construction Materials
            ProductionRecipe::new(
                vec![(ResourceType::IronOre, 1), (ResourceType::Water, 1)],
                vec![(ResourceType::Concrete, 2)],
                1,
            ),
            // Consumer Goods
            ProductionRecipe::new(
                vec![(ResourceType::Plastics, 1), (ResourceType::Electronics, 1)],
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
                vec![(ResourceType::Chemicals, 2), (ResourceType::Water, 1)],
                vec![(ResourceType::Medicine, 2)],
                2,
            ),
            // Chemical Production
            ProductionRecipe::new(
                vec![(ResourceType::Oil, 2), (ResourceType::Water, 1)],
                vec![(ResourceType::Chemicals, 2)],
                1,
            ),
            // Mineral Processing - Ore to Pure Metals
            ProductionRecipe::new(
                vec![(ResourceType::IronOre, 3)],
                vec![(ResourceType::Steel, 1)],
                2,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::CopperOre, 3)],
                vec![(ResourceType::Electronics, 1)],
                2,
            ),
            // Advanced Mineral Processing
            ProductionRecipe::new(
                vec![(ResourceType::Lithium, 2), (ResourceType::Cobalt, 1)],
                vec![(ResourceType::BatteryPack, 1)],
                3,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::IronOre, 2), (ResourceType::Nickel, 1)],
                vec![(ResourceType::Steel, 2)],
                3,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::Zinc, 2), (ResourceType::Silver, 1)],
                vec![(ResourceType::AdvancedComponents, 1)],
                2,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::Gold, 1), (ResourceType::Graphite, 1)],
                vec![(ResourceType::Electronics, 2)],
                3,
            ),
            // Energy Systems Recipes
            ProductionRecipe::new(
                vec![(ResourceType::Silicon, 2), (ResourceType::Titanium, 1)],
                vec![(ResourceType::SolarPower, 2)],
                2,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::Uranium, 3)],
                vec![(ResourceType::NuclearFuel, 1)],
                4,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::Gas, 2), (ResourceType::Coal, 1)],
                vec![(ResourceType::HydrogenCell, 1)],
                2,
            ),
            ProductionRecipe::new(
                vec![
                    (ResourceType::BatteryPack, 3),
                    (ResourceType::Electronics, 1),
                ],
                vec![(ResourceType::Energy, 5)],
                1,
            ),
            ProductionRecipe::new(
                vec![(ResourceType::SolarPower, 1), (ResourceType::Lithium, 1)],
                vec![(ResourceType::BatteryPack, 2)],
                3,
            ),
            // Alternative Steel Production - Arc Furnace
            ProductionRecipe::new(
                vec![
                    (ResourceType::IronOre, 2),
                    (ResourceType::Coal, 1),
                    (ResourceType::Oxygen, 1),
                ],
                vec![(ResourceType::Steel, 3)],
                2,
            ),
            // Alternative Steel Production - Blast Furnace (Multi-step)
            // Step 1: Coal -> Coke (preprocessing)
            ProductionRecipe::new(
                vec![(ResourceType::Coal, 2)],
                vec![(ResourceType::Coke, 1)],
                1,
            ),
            // Step 2: Coke + Limestone + Iron Ore -> Steel (blast furnace reduction)
            ProductionRecipe::new(
                vec![
                    (ResourceType::IronOre, 3),
                    (ResourceType::Coke, 1),
                    (ResourceType::Limestone, 1),
                ],
                vec![(ResourceType::Steel, 4)],
                3,
            ),
            // Electronics Production - Alternative Paths
            // Path 1: Fast & cheap - CopperOre directly to Electronics
            ProductionRecipe::new(
                vec![(ResourceType::CopperOre, 2)],
                vec![(ResourceType::Electronics, 1)],
                1,
            ),
            // Intermediate: CopperOre -> CopperWire (for path 3)
            ProductionRecipe::new(
                vec![(ResourceType::CopperOre, 2)],
                vec![(ResourceType::CopperWire, 1)],
                1,
            ),
            // Path 2: Complex & expensive, high output - Silicon + RareMetals + Gold -> Electronics
            ProductionRecipe::new(
                vec![
                    (ResourceType::Silicon, 1),
                    (ResourceType::RareMetals, 1),
                    (ResourceType::Gold, 1),
                ],
                vec![(ResourceType::Electronics, 2)],
                3,
            ),
            // Path 3: Medium complexity - CopperWire + Plastics -> Electronics
            ProductionRecipe::new(
                vec![(ResourceType::CopperWire, 2), (ResourceType::Plastics, 1)],
                vec![(ResourceType::Electronics, 1)],
                2,
            ),
            // Food Production - Multiple Paths
            // Path 1: Traditional farm - simplest, relies on Water
            ProductionRecipe::new(
                vec![(ResourceType::Water, 2)],
                vec![(ResourceType::Food, 3)],
                2,
            ),
            // Path 2: Hydroponic - Water + Nutrients + Energy (balanced, sustainable)
            ProductionRecipe::new(
                vec![
                    (ResourceType::Water, 1),
                    (ResourceType::Nutrients, 1),
                    (ResourceType::Energy, 2),
                ],
                vec![(ResourceType::Food, 4)],
                3,
            ),
            // Path 3: Chemical synthesis - Chemicals + Energy (fast, high yield)
            ProductionRecipe::new(
                vec![(ResourceType::Chemicals, 1), (ResourceType::Energy, 3)],
                vec![(ResourceType::Food, 5)],
                2,
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
        assert!(deps
            .get(&ResourceType::Steel)
            .unwrap()
            .contains(&ResourceType::IronOre));
    }

    #[test]
    fn test_all_recipes_defined() {
        let recipes = ProductionChains::all_recipes();
        assert!(recipes.len() >= 10);
    }

    #[test]
    fn test_mineral_processing_recipes_exist() {
        // Verify at least 4 advanced mineral processing recipes (using new minerals from 7.2.1)
        let all_recipes = ProductionChains::all_recipes();
        let mineral_recipes: Vec<_> = all_recipes
            .iter()
            .filter(|r| {
                r.inputs.iter().any(|(t, _)| {
                    matches!(
                        t,
                        ResourceType::Nickel
                            | ResourceType::Cobalt
                            | ResourceType::Zinc
                            | ResourceType::Silver
                            | ResourceType::Gold
                            | ResourceType::Lithium
                            | ResourceType::Graphite
                    )
                })
            })
            .collect();
        assert!(
            mineral_recipes.len() >= 4,
            "Expected at least 4 advanced mineral processing recipes, found {}",
            mineral_recipes.len()
        );
    }

    #[test]
    fn test_battery_pack_production() {
        let recipe = ProductionChains::recipes_for_output(ResourceType::BatteryPack);
        assert!(
            !recipe.is_empty(),
            "BatteryPack production recipe not found"
        );
        let battery_recipe = &recipe[0];

        // Verify inputs
        assert!(battery_recipe
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::Lithium));
        assert!(battery_recipe
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::Cobalt));

        // Verify output
        assert_eq!(battery_recipe.outputs.len(), 1);
        assert_eq!(battery_recipe.outputs[0].0, ResourceType::BatteryPack);
        assert_eq!(battery_recipe.outputs[0].1, 1);

        // Verify production time (3 turns)
        assert_eq!(battery_recipe.production_time, 3);
    }

    #[test]
    fn test_nickel_enhanced_steel_production() {
        let recipe = ProductionChains::recipes_for_output(ResourceType::Steel)
            .into_iter()
            .find(|r| r.inputs.iter().any(|(t, _)| *t == ResourceType::Nickel))
            .expect("Steel recipe with Nickel input not found");

        // Verify inputs
        assert!(recipe
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::IronOre));
        assert!(recipe
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::Nickel));

        // Verify output is 2 steel (better than basic recipe)
        assert_eq!(recipe.outputs[0].1, 2);

        // Verify longer production time (3 turns)
        assert_eq!(recipe.production_time, 3);
    }

    #[test]
    fn test_precious_metals_electronics_production() {
        let recipe = ProductionChains::recipes_for_output(ResourceType::Electronics)
            .into_iter()
            .find(|r| r.inputs.iter().any(|(t, _)| *t == ResourceType::Gold))
            .expect("Electronics recipe with Gold input not found");

        // Verify inputs include precious metals
        assert!(recipe.inputs.iter().any(|(t, _)| *t == ResourceType::Gold));
        assert!(recipe
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::Graphite));

        // Verify higher output
        assert_eq!(recipe.outputs[0].1, 2);
    }

    #[test]
    fn test_multiple_steel_paths() {
        let steel_recipes = ProductionChains::recipes_for_output(ResourceType::Steel);

        // Should have at least 2 paths to steel (basic + nickel-enhanced)
        assert!(
            steel_recipes.len() >= 2,
            "Expected multiple paths to Steel, found {}",
            steel_recipes.len()
        );
    }

    #[test]
    fn test_energy_system_recipes_exist() {
        // Verify at least 5 energy system recipes
        let all_recipes = ProductionChains::all_recipes();
        let energy_recipes: Vec<_> = all_recipes
            .iter()
            .filter(|r| {
                r.outputs.iter().any(|(t, _)| {
                    matches!(
                        t,
                        ResourceType::SolarPower
                            | ResourceType::NuclearFuel
                            | ResourceType::BatteryPack
                            | ResourceType::HydrogenCell
                            | ResourceType::Energy
                    )
                }) || r.inputs.iter().any(|(t, _)| {
                    matches!(
                        t,
                        ResourceType::SolarPower
                            | ResourceType::NuclearFuel
                            | ResourceType::BatteryPack
                            | ResourceType::HydrogenCell
                    )
                })
            })
            .collect();
        assert!(
            energy_recipes.len() >= 5,
            "Expected at least 5 energy system recipes, found {}",
            energy_recipes.len()
        );
    }

    #[test]
    fn test_solar_power_production() {
        let recipe = ProductionChains::recipes_for_output(ResourceType::SolarPower);
        assert!(!recipe.is_empty(), "SolarPower production recipe not found");
        let solar_recipe = &recipe[0];

        // Verify inputs
        assert!(solar_recipe
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::Silicon));
        assert!(solar_recipe
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::Titanium));

        // Verify output
        assert_eq!(solar_recipe.outputs[0].0, ResourceType::SolarPower);
        assert_eq!(solar_recipe.outputs[0].1, 2);

        // Verify production time
        assert_eq!(solar_recipe.production_time, 2);
    }

    #[test]
    fn test_nuclear_fuel_production() {
        let recipe = ProductionChains::recipes_for_output(ResourceType::NuclearFuel);
        assert!(
            !recipe.is_empty(),
            "NuclearFuel production recipe not found"
        );
        let nuclear_recipe = &recipe[0];

        // Verify input
        assert_eq!(nuclear_recipe.inputs.len(), 1);
        assert_eq!(nuclear_recipe.inputs[0].0, ResourceType::Uranium);
        assert_eq!(nuclear_recipe.inputs[0].1, 3);

        // Verify output
        assert_eq!(nuclear_recipe.outputs[0].0, ResourceType::NuclearFuel);

        // Verify longer production time (4 turns - most time-consuming)
        assert_eq!(nuclear_recipe.production_time, 4);
    }

    #[test]
    fn test_hydrogen_cell_production() {
        let recipe = ProductionChains::recipes_for_output(ResourceType::HydrogenCell);
        assert!(
            !recipe.is_empty(),
            "HydrogenCell production recipe not found"
        );
        let hydrogen_recipe = &recipe[0];

        // Verify inputs
        assert!(hydrogen_recipe
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::Gas));
        assert!(hydrogen_recipe
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::Coal));

        // Verify output
        assert_eq!(hydrogen_recipe.outputs[0].0, ResourceType::HydrogenCell);
    }

    #[test]
    fn test_battery_energy_discharge() {
        let recipe = ProductionChains::recipes_for_output(ResourceType::Energy);
        let battery_discharge = recipe
            .iter()
            .find(|r| {
                r.inputs
                    .iter()
                    .any(|(t, _)| *t == ResourceType::BatteryPack)
            })
            .expect("Battery discharge recipe not found");

        // Verify inputs
        assert!(battery_discharge
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::BatteryPack));
        assert!(battery_discharge
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::Electronics));

        // Verify significant energy output from battery
        assert!(
            battery_discharge.outputs[0].1 >= 5,
            "Battery discharge should produce substantial energy"
        );

        // Verify fast discharge (1 turn)
        assert_eq!(battery_discharge.production_time, 1);
    }

    #[test]
    fn test_advanced_battery_production() {
        let recipe = ProductionChains::recipes_for_output(ResourceType::BatteryPack)
            .into_iter()
            .find(|r| r.inputs.iter().any(|(t, _)| *t == ResourceType::SolarPower))
            .expect("Advanced battery (solar) production recipe not found");

        // Verify inputs: SolarPower + Lithium
        assert!(recipe
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::SolarPower));
        assert!(recipe
            .inputs
            .iter()
            .any(|(t, _)| *t == ResourceType::Lithium));

        // Verify output (2 battery packs from solar + lithium)
        assert_eq!(recipe.outputs[0].1, 2);

        // Verify moderate production time
        assert_eq!(recipe.production_time, 3);
    }

    #[test]
    fn test_energy_production_chain_from_minerals() {
        // Verify complete chain: minerals → energy storage
        // Lithium + Cobalt → BatteryPack (from 7.2.3)
        let battery_from_minerals = ProductionChains::recipes_for_output(ResourceType::BatteryPack)
            .into_iter()
            .find(|r| {
                r.inputs.iter().any(|(t, _)| *t == ResourceType::Lithium)
                    && r.inputs.iter().any(|(t, _)| *t == ResourceType::Cobalt)
            })
            .expect("Battery from minerals recipe not found");

        assert_eq!(battery_from_minerals.inputs.len(), 2);
        assert_eq!(battery_from_minerals.production_time, 3);
    }
}
