use outpost3::domain::production_chain::{ProductionChains, ProductionRecipe};
use outpost3::domain::resource::{ResourceType, Resources};

fn can_apply(resources: &Resources, recipe: &ProductionRecipe) -> bool {
    recipe
        .inputs
        .iter()
        .all(|(t, qty)| resources.get(*t) >= *qty as i64)
}

fn apply_recipe(resources: &mut Resources, recipe: &ProductionRecipe) {
    for (t, qty) in &recipe.inputs {
        assert!(resources.subtract(*t, *qty as i64));
    }
    for (t, qty) in &recipe.outputs {
        resources.add_resource(*t, *qty as i64);
    }
}

#[test]
fn full_chain_lithium_to_battery_pack() {
    let mut resources = Resources::new();
    resources.set(ResourceType::Lithium, 10);
    resources.set(ResourceType::Cobalt, 5);
    resources.set(ResourceType::SolarPower, 6);

    // Step 1: Lithium + Cobalt -> BatteryPack
    let cobalt_path =
        ProductionChains::get_recipe(ResourceType::Lithium, ResourceType::BatteryPack)
            .expect("Lithium to BatteryPack recipe should exist");
    assert!(can_apply(&resources, &cobalt_path));
    apply_recipe(&mut resources, &cobalt_path);

    // Step 2: SolarPower + Lithium -> BatteryPack (alternative path)
    let solar_path =
        ProductionChains::get_recipe(ResourceType::SolarPower, ResourceType::BatteryPack)
            .expect("SolarPower + Lithium to BatteryPack recipe should exist");
    assert!(can_apply(&resources, &solar_path));
    apply_recipe(&mut resources, &solar_path);

    // Validate BatteryPack stock increased and Lithium decreased from both paths
    assert!(resources.get(ResourceType::BatteryPack) >= 3);
    assert!(resources.get(ResourceType::Lithium) < 10);
}

#[test]
fn steel_has_multiple_paths_including_nickel() {
    let recipes = ProductionChains::recipes_for_output(ResourceType::Steel);
    assert!(recipes.len() >= 2, "Should have multiple steel recipes");

    let mut unique_inputs = std::collections::HashSet::new();
    let mut has_nickel_path = false;
    for r in recipes {
        let mut inputs: Vec<_> = r.inputs.iter().map(|(t, _)| *t as i32).collect();
        inputs.sort();
        unique_inputs.insert(inputs);
        if r.inputs.iter().any(|(t, _)| *t == ResourceType::Nickel) {
            has_nickel_path = true;
        }
    }

    assert!(
        unique_inputs.len() >= 2,
        "Steel should have at least two distinct input sets"
    );
    assert!(has_nickel_path, "Steel should have a path involving Nickel");
}

#[test]
fn hundred_turn_resource_chain_simulation() {
    let mut resources = Resources::new();
    resources.set(ResourceType::Lithium, 40);
    resources.set(ResourceType::Cobalt, 25);
    resources.set(ResourceType::SolarPower, 30);
    resources.set(ResourceType::IronOre, 120);
    resources.set(ResourceType::Nickel, 40);

    let battery_recipe =
        ProductionChains::get_recipe(ResourceType::Lithium, ResourceType::BatteryPack)
            .expect("Lithium + Cobalt battery recipe exists");
    let battery_alt =
        ProductionChains::get_recipe(ResourceType::SolarPower, ResourceType::BatteryPack)
            .expect("SolarPower + Lithium battery recipe exists");
    let steel_recipe = ProductionChains::get_recipe(ResourceType::IronOre, ResourceType::Steel)
        .expect("Iron ore steel recipe exists");
    let steel_alt = ProductionChains::get_recipe(ResourceType::Nickel, ResourceType::Steel)
        .expect("Nickel steel recipe exists");

    for _turn in 0..100 {
        if can_apply(&resources, &battery_recipe) {
            apply_recipe(&mut resources, &battery_recipe);
        }
        if can_apply(&resources, &battery_alt) {
            apply_recipe(&mut resources, &battery_alt);
        }
        if can_apply(&resources, &steel_recipe) {
            apply_recipe(&mut resources, &steel_recipe);
        }
        if can_apply(&resources, &steel_alt) {
            apply_recipe(&mut resources, &steel_alt);
        }
    }

    assert!(resources.get(ResourceType::BatteryPack) > 0);
    assert!(resources.get(ResourceType::Steel) > 0);
    // Ensure no resource went negative
    for t in [
        ResourceType::Lithium,
        ResourceType::Cobalt,
        ResourceType::SolarPower,
        ResourceType::IronOre,
        ResourceType::Nickel,
        ResourceType::BatteryPack,
        ResourceType::Steel,
    ] {
        assert!(
            resources.get(t) >= 0,
            "Resource {:?} should not be negative",
            t
        );
    }
}

#[test]
fn arc_furnace_steel_recipe_exists() {
    let recipes = ProductionChains::recipes_for_output(ResourceType::Steel);

    // Should have multiple steel recipes including arc furnace
    assert!(
        recipes.len() >= 3,
        "Should have at least 3 steel recipes (blast furnace, alloy method, arc furnace)"
    );

    // Find the arc furnace recipe (uses Oxygen)
    let arc_furnace_recipe = recipes
        .iter()
        .find(|r| r.inputs.iter().any(|(t, _)| *t == ResourceType::Oxygen))
        .expect("Arc furnace recipe (with Oxygen) should exist");

    // Verify inputs: IronOre (2) + Coal (1) + Oxygen (1)
    let iron_input = arc_furnace_recipe
        .inputs
        .iter()
        .find(|(t, _)| *t == ResourceType::IronOre);
    assert_eq!(
        iron_input,
        Some(&(ResourceType::IronOre, 2)),
        "Arc furnace should use 2 IronOre"
    );

    let coal_input = arc_furnace_recipe
        .inputs
        .iter()
        .find(|(t, _)| *t == ResourceType::Coal);
    assert_eq!(
        coal_input,
        Some(&(ResourceType::Coal, 1)),
        "Arc furnace should use 1 Coal"
    );

    let oxygen_input = arc_furnace_recipe
        .inputs
        .iter()
        .find(|(t, _)| *t == ResourceType::Oxygen);
    assert_eq!(
        oxygen_input,
        Some(&(ResourceType::Oxygen, 1)),
        "Arc furnace should use 1 Oxygen"
    );

    // Verify output: Steel (3)
    assert_eq!(
        arc_furnace_recipe.outputs.len(),
        1,
        "Arc furnace should produce 1 output type"
    );
    assert_eq!(
        arc_furnace_recipe.outputs[0],
        (ResourceType::Steel, 3),
        "Arc furnace should produce 3 Steel"
    );

    // Verify production time: 2 turns
    assert_eq!(
        arc_furnace_recipe.production_time, 2,
        "Arc furnace should take 2 turns"
    );
}

#[test]
fn arc_furnace_production_cycle() {
    let mut resources = Resources::new();
    resources.set(ResourceType::IronOre, 10);
    resources.set(ResourceType::Coal, 5);
    resources.set(ResourceType::Oxygen, 5);

    // Get arc furnace recipe via Oxygen as key input
    let recipes = ProductionChains::recipes_for_output(ResourceType::Steel);
    let arc_furnace_recipe = recipes
        .iter()
        .find(|r| r.inputs.iter().any(|(t, _)| *t == ResourceType::Oxygen))
        .expect("Arc furnace recipe should exist");

    let initial_steel = resources.get(ResourceType::Steel);

    // Apply recipe twice (consuming 4 IronOre, 2 Coal, 2 Oxygen → producing 6 Steel)
    assert!(can_apply(&resources, arc_furnace_recipe));
    apply_recipe(&mut resources, arc_furnace_recipe);

    assert!(can_apply(&resources, arc_furnace_recipe));
    apply_recipe(&mut resources, arc_furnace_recipe);

    // Verify resource changes
    assert_eq!(
        resources.get(ResourceType::IronOre),
        6,
        "Should have consumed 4 IronOre"
    );
    assert_eq!(
        resources.get(ResourceType::Coal),
        3,
        "Should have consumed 2 Coal"
    );
    assert_eq!(
        resources.get(ResourceType::Oxygen),
        3,
        "Should have consumed 2 Oxygen"
    );
    assert_eq!(
        resources.get(ResourceType::Steel),
        initial_steel + 6,
        "Should have produced 6 Steel"
    );
}
#[test]
fn blast_furnace_coke_production_step() {
    let mut resources = Resources::new();
    resources.set(ResourceType::Coal, 20);

    // Get coal to coke recipe
    let recipes = ProductionChains::recipes_for_output(ResourceType::Coke);
    let coke_recipe = recipes
        .iter()
        .find(|r| r.inputs.iter().any(|(t, _)| *t == ResourceType::Coal))
        .expect("Coal to Coke recipe should exist");

    // Verify recipe: Coal (2) -> Coke (1) in 1 turn
    assert_eq!(coke_recipe.inputs.len(), 1, "Should only use Coal");
    assert_eq!(
        coke_recipe.inputs[0],
        (ResourceType::Coal, 2),
        "Should use 2 Coal"
    );
    assert_eq!(coke_recipe.outputs.len(), 1, "Should produce 1 output type");
    assert_eq!(
        coke_recipe.outputs[0],
        (ResourceType::Coke, 1),
        "Should produce 1 Coke"
    );
    assert_eq!(coke_recipe.production_time, 1, "Should take 1 turn");

    // Apply the recipe 10 times (consumes 20 coal)
    for _ in 0..10 {
        assert!(can_apply(&resources, coke_recipe));
        apply_recipe(&mut resources, coke_recipe);
    }

    // Verify resources: 20 coal -> 10 coke
    assert_eq!(
        resources.get(ResourceType::Coal),
        0,
        "Should consume all coal"
    );
    assert_eq!(
        resources.get(ResourceType::Coke),
        10,
        "Should produce 10 coke"
    );
}

#[test]
fn blast_furnace_steel_production_step() {
    let mut resources = Resources::new();
    resources.set(ResourceType::IronOre, 30);
    resources.set(ResourceType::Coke, 5);
    resources.set(ResourceType::Limestone, 5);

    // Get blast furnace recipe (IronOre + Coke + Limestone -> Steel)
    let recipes = ProductionChains::recipes_for_output(ResourceType::Steel);
    let blast_furnace_recipe = recipes
        .iter()
        .find(|r| {
            r.inputs.iter().any(|(t, _)| *t == ResourceType::Coke)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Limestone)
        })
        .expect("Blast furnace recipe should exist");

    // Verify recipe: IronOre (3) + Coke (1) + Limestone (1) -> Steel (4) in 3 turns
    assert_eq!(
        blast_furnace_recipe.inputs.len(),
        3,
        "Should use 3 input types"
    );
    assert_eq!(
        blast_furnace_recipe
            .inputs
            .iter()
            .find(|(t, _)| *t == ResourceType::IronOre),
        Some(&(ResourceType::IronOre, 3)),
        "Should use 3 IronOre"
    );
    assert_eq!(
        blast_furnace_recipe
            .inputs
            .iter()
            .find(|(t, _)| *t == ResourceType::Coke),
        Some(&(ResourceType::Coke, 1)),
        "Should use 1 Coke"
    );
    assert_eq!(
        blast_furnace_recipe
            .inputs
            .iter()
            .find(|(t, _)| *t == ResourceType::Limestone),
        Some(&(ResourceType::Limestone, 1)),
        "Should use 1 Limestone"
    );
    assert_eq!(
        blast_furnace_recipe.outputs[0],
        (ResourceType::Steel, 4),
        "Should produce 4 Steel"
    );
    assert_eq!(
        blast_furnace_recipe.production_time, 3,
        "Should take 3 turns"
    );

    // Apply the recipe 5 times
    for _ in 0..5 {
        assert!(can_apply(&resources, blast_furnace_recipe));
        apply_recipe(&mut resources, blast_furnace_recipe);
    }

    // Verify resources: 30 iron ore + 5 coke + 5 limestone -> 20 steel
    assert_eq!(
        resources.get(ResourceType::IronOre),
        15,
        "Should have consumed 15 IronOre"
    );
    assert_eq!(
        resources.get(ResourceType::Coke),
        0,
        "Should consume all Coke"
    );
    assert_eq!(
        resources.get(ResourceType::Limestone),
        0,
        "Should consume all Limestone"
    );
    assert_eq!(
        resources.get(ResourceType::Steel),
        20,
        "Should produce 20 Steel"
    );
}

#[test]
fn coal_to_coke_to_steel_full_chain() {
    let mut resources = Resources::new();
    resources.set(ResourceType::Coal, 30);
    resources.set(ResourceType::IronOre, 60);
    resources.set(ResourceType::Limestone, 10);

    // Step 1: Convert coal to coke
    let coke_recipe = ProductionChains::get_recipe(ResourceType::Coal, ResourceType::Coke)
        .expect("Coal to Coke recipe should exist");

    // Convert 30 coal into 15 coke (2:1 ratio, takes 1 turn per recipe)
    for _ in 0..15 {
        assert!(can_apply(&resources, &coke_recipe));
        apply_recipe(&mut resources, &coke_recipe);
    }

    assert_eq!(resources.get(ResourceType::Coal), 0);
    assert_eq!(resources.get(ResourceType::Coke), 15);

    // Step 2: Convert coke + iron ore + limestone to steel via blast furnace
    let blast_furnace_recipe =
        ProductionChains::get_recipe(ResourceType::Coke, ResourceType::Steel)
            .expect("Blast furnace recipe should exist");

    // Apply 10 times (consumes 10 coke, 30 iron ore, 10 limestone -> 40 steel)
    for _ in 0..10 {
        assert!(can_apply(&resources, &blast_furnace_recipe));
        apply_recipe(&mut resources, &blast_furnace_recipe);
    }

    // Verify final state
    assert_eq!(
        resources.get(ResourceType::Coal),
        0,
        "All coal should be used"
    );
    assert_eq!(resources.get(ResourceType::Coke), 5, "5 Coke should remain");
    assert_eq!(
        resources.get(ResourceType::IronOre),
        30,
        "30 IronOre should remain"
    );
    assert_eq!(
        resources.get(ResourceType::Limestone),
        0,
        "All limestone should be used"
    );
    assert_eq!(
        resources.get(ResourceType::Steel),
        40,
        "Should produce 40 Steel from blast furnace"
    );
}

#[test]
fn blast_furnace_recipes_exist_in_production_chains() {
    let all_recipes = ProductionChains::all_recipes();

    // Find coke recipe
    let coke_recipes: Vec<_> = all_recipes
        .iter()
        .filter(|r| r.outputs.iter().any(|(t, _)| *t == ResourceType::Coke))
        .collect();
    assert!(
        !coke_recipes.is_empty(),
        "Coke production recipe should exist"
    );

    // Find blast furnace steel recipe (uses Coke and Limestone)
    let blast_recipes: Vec<_> = all_recipes
        .iter()
        .filter(|r| {
            r.outputs.iter().any(|(t, _)| *t == ResourceType::Steel)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Coke)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Limestone)
        })
        .collect();
    assert!(
        !blast_recipes.is_empty(),
        "Blast furnace steel recipe (using Coke and Limestone) should exist"
    );

    // Verify steel now has at least 4 recipes (basic x2, nickel, arc furnace, blast furnace)
    let steel_recipes = ProductionChains::recipes_for_output(ResourceType::Steel);
    assert!(
        steel_recipes.len() >= 4,
        "Should have at least 4 steel recipes"
    );
}

#[test]
fn electronics_has_three_alternative_paths() {
    let recipes = ProductionChains::recipes_for_output(ResourceType::Electronics);
    assert!(
        recipes.len() >= 3,
        "Should have at least 3 electronics recipes, found {}",
        recipes.len()
    );
}

#[test]
fn electronics_path1_copperore_direct_fast_and_cheap() {
    let mut resources = Resources::new();
    resources.set(ResourceType::CopperOre, 10);

    // Path 1: CopperOre (2) -> Electronics (1), 1 turn - fast & cheap
    let recipe = ProductionChains::get_recipe(ResourceType::CopperOre, ResourceType::Electronics)
        .expect("CopperOre to Electronics recipe should exist");

    assert_eq!(recipe.inputs.len(), 1);
    assert_eq!(recipe.inputs[0].0, ResourceType::CopperOre);
    assert_eq!(recipe.inputs[0].1, 2);
    assert_eq!(recipe.outputs.len(), 1);
    assert_eq!(recipe.outputs[0].0, ResourceType::Electronics);
    assert_eq!(recipe.outputs[0].1, 1);
    assert_eq!(
        recipe.production_time, 1,
        "Path 1 should be fastest (1 turn)"
    );

    // Apply recipe 5 times: 10 copper ore -> 5 electronics
    for _ in 0..5 {
        assert!(can_apply(&resources, &recipe));
        apply_recipe(&mut resources, &recipe);
    }
    assert_eq!(resources.get(ResourceType::CopperOre), 0);
    assert_eq!(resources.get(ResourceType::Electronics), 5);
}

#[test]
fn electronics_path2_silicon_rare_metals_gold_complex_expensive() {
    let mut resources = Resources::new();
    resources.set(ResourceType::Silicon, 3);
    resources.set(ResourceType::RareMetals, 3);
    resources.set(ResourceType::Gold, 3);

    // Path 2: Silicon (1) + RareMetals (1) + Gold (1) -> Electronics (2), 3 turns - complex & expensive
    let recipes = ProductionChains::recipes_for_output(ResourceType::Electronics);
    let complex_recipe = recipes
        .iter()
        .find(|r| {
            r.production_time == 3
                && r.inputs.len() == 3
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Silicon)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::RareMetals)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Gold)
        })
        .expect("Complex electronics recipe (Silicon+RareMetals+Gold) should exist");

    assert_eq!(
        complex_recipe.production_time, 3,
        "Path 2 should be slowest (3 turns)"
    );
    assert_eq!(
        complex_recipe.outputs[0].1, 2,
        "Complex path should produce 2 electronics per recipe"
    );

    // Apply recipe 3 times: 3 silicon + 3 rare metals + 3 gold -> 6 electronics
    for _ in 0..3 {
        assert!(can_apply(&resources, complex_recipe));
        apply_recipe(&mut resources, complex_recipe);
    }
    assert_eq!(resources.get(ResourceType::Silicon), 0);
    assert_eq!(resources.get(ResourceType::RareMetals), 0);
    assert_eq!(resources.get(ResourceType::Gold), 0);
    assert_eq!(resources.get(ResourceType::Electronics), 6);
}

#[test]
fn electronics_path3_copperwise_plastics_medium_complexity() {
    let mut resources = Resources::new();
    // First produce CopperWire from CopperOre
    resources.set(ResourceType::CopperOre, 20);
    resources.set(ResourceType::Plastics, 5);

    // Get CopperWire recipe and produce it
    let copperwise_recipe =
        ProductionChains::get_recipe(ResourceType::CopperOre, ResourceType::CopperWire)
            .expect("CopperOre to CopperWire recipe should exist");

    // Apply 10 times: 20 copper ore -> 10 copper wire
    for _ in 0..10 {
        assert!(can_apply(&resources, &copperwise_recipe));
        apply_recipe(&mut resources, &copperwise_recipe);
    }
    assert_eq!(resources.get(ResourceType::CopperWire), 10);

    // Path 3: CopperWire (2) + Plastics (1) -> Electronics (1), 2 turns - medium
    let recipes = ProductionChains::recipes_for_output(ResourceType::Electronics);
    let medium_recipe = recipes
        .iter()
        .find(|r| {
            r.production_time == 2
                && r.inputs.len() == 2
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::CopperWire)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Plastics)
        })
        .expect("Medium electronics recipe (CopperWire+Plastics) should exist");

    assert_eq!(
        medium_recipe.production_time, 2,
        "Path 3 should be medium (2 turns)"
    );

    // Apply recipe 5 times: 10 copper wire + 5 plastics -> 5 electronics
    for _ in 0..5 {
        assert!(can_apply(&resources, medium_recipe));
        apply_recipe(&mut resources, medium_recipe);
    }
    assert_eq!(resources.get(ResourceType::CopperWire), 0);
    assert_eq!(resources.get(ResourceType::Plastics), 0);
    assert_eq!(resources.get(ResourceType::Electronics), 5);
}

#[test]
fn electronics_efficiency_trade_offs() {
    // Path 1: CopperOre (2) -> Electronics (1), 1 turn = 0.5 electronics/ore/turn
    // Path 2: Silicon (1) + RareMetals (1) + Gold (1) -> Electronics (2), 3 turns = 0.67/component/turn
    // Path 3: CopperWire (2) + Plastics (1) -> Electronics (1), 2 turns = 0.5/wire/turn

    let recipes = ProductionChains::recipes_for_output(ResourceType::Electronics);
    assert!(
        recipes.len() >= 3,
        "Must have at least 3 electronics recipes"
    );

    // Find fast path (1 turn, CopperOre)
    let path1 = recipes
        .iter()
        .find(|r| {
            r.production_time == 1
                && r.inputs.len() == 1
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::CopperOre)
        })
        .expect("Path 1 (fast CopperOre) should exist");

    // Find complex path (3 turns, multiple rare inputs)
    let path2 = recipes
        .iter()
        .find(|r| {
            r.production_time == 3
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Silicon)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::RareMetals)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Gold)
        })
        .expect("Path 2 (complex Silicon+RareMetals+Gold) should exist");

    // Find medium path (2 turns, CopperWire + Plastics)
    let path3 = recipes
        .iter()
        .find(|r| {
            r.production_time == 2
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::CopperWire)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Plastics)
        })
        .expect("Path 3 (medium CopperWire+Plastics) should exist");

    // Path 1: Fast, cheap (basic CopperOre)
    assert_eq!(path1.production_time, 1);
    assert!(path1
        .inputs
        .iter()
        .any(|(t, _)| *t == ResourceType::CopperOre));

    // Path 2: Slow, complex (rare materials) - produces more
    assert_eq!(path2.production_time, 3);
    assert!(
        path2.outputs[0].1 > path1.outputs[0].1,
        "Path 2 should produce more per recipe"
    );

    // Path 3: Medium, multi-step (requires wire production)
    assert_eq!(path3.production_time, 2);
    assert!(path3
        .inputs
        .iter()
        .any(|(t, _)| *t == ResourceType::CopperWire));
    assert!(path3
        .inputs
        .iter()
        .any(|(t, _)| *t == ResourceType::Plastics));
}

#[test]
fn food_has_three_production_methods() {
    let recipes = ProductionChains::recipes_for_output(ResourceType::Food);
    assert!(
        recipes.len() >= 3,
        "Should have at least 3 food production recipes, found {}",
        recipes.len()
    );
}

#[test]
fn food_path1_traditional_farm_simple() {
    let mut resources = Resources::new();
    resources.set(ResourceType::Water, 20);

    // Path 1: Water (2) -> Food (3), 2 turns - simple & traditional
    let recipe = ProductionChains::get_recipe(ResourceType::Water, ResourceType::Food)
        .expect("Water to Food recipe should exist");

    assert_eq!(recipe.inputs.len(), 1);
    assert_eq!(recipe.inputs[0].0, ResourceType::Water);
    assert_eq!(recipe.inputs[0].1, 2);
    assert_eq!(recipe.outputs.len(), 1);
    assert_eq!(recipe.outputs[0].0, ResourceType::Food);
    assert_eq!(recipe.outputs[0].1, 3);
    assert_eq!(recipe.production_time, 2);

    // Apply recipe 10 times: 20 water -> 30 food
    for _ in 0..10 {
        assert!(can_apply(&resources, &recipe));
        apply_recipe(&mut resources, &recipe);
    }
    assert_eq!(resources.get(ResourceType::Water), 0);
    assert_eq!(resources.get(ResourceType::Food), 30);
}

#[test]
fn food_path2_hydroponic_balanced_sustainable() {
    let mut resources = Resources::new();
    resources.set(ResourceType::Water, 5);
    resources.set(ResourceType::Nutrients, 5);
    resources.set(ResourceType::Energy, 20);

    // Path 2: Water (1) + Nutrients (1) + Energy (2) -> Food (4), 3 turns - balanced
    let recipes = ProductionChains::recipes_for_output(ResourceType::Food);
    let hydroponic_recipe = recipes
        .iter()
        .find(|r| {
            r.production_time == 3
                && r.inputs.len() == 3
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Water)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Nutrients)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Energy)
        })
        .expect("Hydroponic recipe should exist");

    assert_eq!(hydroponic_recipe.production_time, 3);
    assert_eq!(hydroponic_recipe.outputs[0].1, 4);

    // Apply recipe 5 times: 5 water + 5 nutrients + 10 energy -> 20 food
    for _ in 0..5 {
        assert!(can_apply(&resources, hydroponic_recipe));
        apply_recipe(&mut resources, hydroponic_recipe);
    }
    assert_eq!(resources.get(ResourceType::Water), 0);
    assert_eq!(resources.get(ResourceType::Nutrients), 0);
    assert_eq!(resources.get(ResourceType::Energy), 10);
    assert_eq!(resources.get(ResourceType::Food), 20);
}

#[test]
fn food_path3_chemical_synthesis_fast_high_yield() {
    let mut resources = Resources::new();
    resources.set(ResourceType::Chemicals, 10);
    resources.set(ResourceType::Energy, 50);

    // Path 3: Chemicals (1) + Energy (3) -> Food (5), 2 turns - fast, high yield
    let recipes = ProductionChains::recipes_for_output(ResourceType::Food);
    let synthesis_recipe = recipes
        .iter()
        .find(|r| {
            r.production_time == 2
                && r.inputs.len() == 2
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Chemicals)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Energy)
                && r.outputs[0].1 == 5
        })
        .expect("Chemical synthesis recipe should exist");

    assert_eq!(synthesis_recipe.production_time, 2);
    assert_eq!(synthesis_recipe.outputs[0].1, 5);

    // Apply recipe 10 times: 10 chemicals + 30 energy -> 50 food
    for _ in 0..10 {
        assert!(can_apply(&resources, synthesis_recipe));
        apply_recipe(&mut resources, synthesis_recipe);
    }
    assert_eq!(resources.get(ResourceType::Chemicals), 0);
    assert_eq!(resources.get(ResourceType::Energy), 20);
    assert_eq!(resources.get(ResourceType::Food), 50);
}

#[test]
fn food_production_resource_efficiency_comparison() {
    // Compare efficiency across the three paths:
    // Path 1: 2 water -> 3 food in 2 turns = 1.5 food per water per turn
    // Path 2: (1 water + 1 nutrients + 2 energy) -> 4 food in 3 turns = 4/4 = 1.0 food per input per turn
    // Path 3: (1 chemicals + 3 energy) -> 5 food in 2 turns = 1.25 food per input per turn

    let recipes = ProductionChains::recipes_for_output(ResourceType::Food);
    assert!(recipes.len() >= 3, "Must have at least 3 food recipes");

    // Find each path by production time and composition
    let path1 = recipes
        .iter()
        .find(|r| r.production_time == 2 && r.inputs.len() == 1)
        .expect("Path 1 (simple farm) should exist");

    let path2 = recipes
        .iter()
        .find(|r| r.production_time == 3 && r.inputs.len() == 3)
        .expect("Path 2 (hydroponic) should exist");

    let path3 = recipes
        .iter()
        .find(|r| r.production_time == 2 && r.inputs.len() == 2)
        .expect("Path 3 (synthesis) should exist");

    // Path 1: Simplest - water only
    assert_eq!(path1.inputs[0].0, ResourceType::Water);
    assert_eq!(path1.inputs[0].1, 2);
    assert_eq!(path1.outputs[0].0, ResourceType::Food);
    assert_eq!(path1.outputs[0].1, 3);

    // Path 2: Balanced - three inputs
    assert!(path2.inputs.iter().any(|(t, _)| *t == ResourceType::Water));
    assert!(path2
        .inputs
        .iter()
        .any(|(t, _)| *t == ResourceType::Nutrients));
    assert!(path2.inputs.iter().any(|(t, _)| *t == ResourceType::Energy));
    assert_eq!(path2.outputs[0].1, 4);

    // Path 3: High-yield - chemicals + energy
    assert!(path3
        .inputs
        .iter()
        .any(|(t, _)| *t == ResourceType::Chemicals));
    assert!(path3.inputs.iter().any(|(t, _)| *t == ResourceType::Energy));
    assert_eq!(
        path3.outputs[0].1, 5,
        "Synthesis should produce most food per recipe"
    );
}

#[test]
fn sustain_population_traditional_farm_path() {
    // Simulate sustaining population with traditional farm
    let mut resources = Resources::new();
    resources.set(ResourceType::Water, 100);

    let farm_recipe = ProductionChains::get_recipe(ResourceType::Water, ResourceType::Food)
        .expect("Farm recipe should exist");

    // Produce food for 5 iterations
    for _ in 0..5 {
        assert!(can_apply(&resources, &farm_recipe));
        apply_recipe(&mut resources, &farm_recipe);
    }

    // Should have produced 15 food from 10 water (5 iterations × 2 water → 3 food)
    assert_eq!(resources.get(ResourceType::Food), 15);
    assert_eq!(resources.get(ResourceType::Water), 90);
}

#[test]
fn sustain_population_hydroponic_path() {
    // Simulate sustaining population with hydroponic
    let mut resources = Resources::new();
    resources.set(ResourceType::Water, 50);
    resources.set(ResourceType::Nutrients, 50);
    resources.set(ResourceType::Energy, 200);

    let hydroponic_recipes = ProductionChains::recipes_for_output(ResourceType::Food);
    let hydroponic_recipe = hydroponic_recipes
        .iter()
        .find(|r| r.inputs.len() == 3)
        .expect("Hydroponic recipe should exist");

    // Produce food for 10 iterations
    for _ in 0..10 {
        assert!(can_apply(&resources, hydroponic_recipe));
        apply_recipe(&mut resources, hydroponic_recipe);
    }

    // Should have produced 40 food, consumed half resources
    assert_eq!(resources.get(ResourceType::Food), 40);
    assert_eq!(resources.get(ResourceType::Water), 40);
    assert_eq!(resources.get(ResourceType::Nutrients), 40);
}

#[test]
fn sustain_population_synthesis_path() {
    // Simulate sustaining population with chemical synthesis
    let mut resources = Resources::new();
    resources.set(ResourceType::Chemicals, 50);
    resources.set(ResourceType::Energy, 200);

    let synthesis_recipes = ProductionChains::recipes_for_output(ResourceType::Food);
    let synthesis_recipe = synthesis_recipes
        .iter()
        .find(|r| r.inputs.len() == 2 && r.outputs[0].1 == 5)
        .expect("Synthesis recipe should exist");

    // Produce food for 20 iterations
    for _ in 0..20 {
        assert!(can_apply(&resources, synthesis_recipe));
        apply_recipe(&mut resources, synthesis_recipe);
    }

    // Should have produced 100 food, consumed all chemicals and some energy
    assert_eq!(resources.get(ResourceType::Food), 100);
    assert_eq!(resources.get(ResourceType::Chemicals), 30);
}

// Task 7.4.8: Production Chain Integration Tests

#[test]
fn steel_production_via_three_methods() {
    // Test steel production using all 3 methods: basic, blast furnace, enhanced nickel
    let mut resources = Resources::new();
    resources.set(ResourceType::IronOre, 200);
    resources.set(ResourceType::Coal, 50);
    resources.set(ResourceType::Nickel, 50);
    resources.set(ResourceType::Limestone, 20);

    let initial_steel = resources.get(ResourceType::Steel);

    // Method 1: Basic Iron → Steel (IronOre(3) → Steel(1))
    let basic_recipe = ProductionChains::get_recipe(ResourceType::IronOre, ResourceType::Steel)
        .expect("Basic iron to steel recipe should exist");
    assert!(can_apply(&resources, &basic_recipe));
    apply_recipe(&mut resources, &basic_recipe);
    let steel_after_basic = resources.get(ResourceType::Steel);
    assert!(
        steel_after_basic > initial_steel,
        "Basic recipe should produce steel"
    );

    // Method 2: Enhanced Nickel (IronOre(2) + Nickel(1) → Steel(2))
    let enhanced_recipes = ProductionChains::recipes_for_output(ResourceType::Steel);
    let enhanced = enhanced_recipes
        .iter()
        .find(|r| r.inputs.iter().any(|(t, _)| *t == ResourceType::Nickel))
        .expect("Nickel enhanced recipe should exist");
    assert!(can_apply(&resources, enhanced));
    apply_recipe(&mut resources, enhanced);
    let steel_after_enhanced = resources.get(ResourceType::Steel);
    assert!(
        steel_after_enhanced > steel_after_basic,
        "Enhanced recipe should produce more steel"
    );

    // Method 3: Blast Furnace (two-step: Coal → Coke, then Coke + IronOre + Limestone → Steel)
    // Step 1: Make coke from coal
    let coke_recipe = ProductionChains::get_recipe(ResourceType::Coal, ResourceType::Coke)
        .expect("Coal to Coke recipe should exist");
    assert!(can_apply(&resources, &coke_recipe));
    apply_recipe(&mut resources, &coke_recipe);
    assert!(
        resources.get(ResourceType::Coke) > 0,
        "Should have coke after first step"
    );

    // Step 2: Use coke to make steel
    let blast_furnace_recipes = ProductionChains::recipes_for_output(ResourceType::Steel);
    let blast_furnace = blast_furnace_recipes
        .iter()
        .find(|r| {
            r.inputs.iter().any(|(t, _)| *t == ResourceType::Coke)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::IronOre)
                && r.inputs.iter().any(|(t, _)| *t == ResourceType::Limestone)
        })
        .expect("Blast furnace recipe should exist");
    assert!(can_apply(&resources, blast_furnace));
    apply_recipe(&mut resources, blast_furnace);
    let steel_after_blast = resources.get(ResourceType::Steel);
    assert!(
        steel_after_blast > steel_after_enhanced,
        "Blast furnace should produce even more steel"
    );

    // Verify all three methods were used and produced steel
    assert!(
        resources.get(ResourceType::IronOre) < 200,
        "Iron ore should be consumed"
    );
    assert!(
        resources.get(ResourceType::Coal) < 50,
        "Coal should be consumed"
    );
    assert!(
        resources.get(ResourceType::Nickel) < 50,
        "Nickel should be consumed"
    );
    assert!(
        resources.get(ResourceType::Limestone) < 20,
        "Limestone should be consumed"
    );
    assert!(
        steel_after_blast >= 7,
        "Should have produced at least 7 steel (1+2+4)"
    );
}

#[test]
fn recipe_switching_mid_production() {
    // Simulate switching between different recipes during production
    let mut resources = Resources::new();
    resources.set(ResourceType::Lithium, 100);
    resources.set(ResourceType::Cobalt, 50);
    resources.set(ResourceType::SolarPower, 50);

    // Start with Lithium + Cobalt recipe
    let cobalt_recipe =
        ProductionChains::get_recipe(ResourceType::Lithium, ResourceType::BatteryPack)
            .expect("Lithium + Cobalt recipe exists");

    // Produce 5 batteries with cobalt recipe
    for _ in 0..5 {
        if can_apply(&resources, &cobalt_recipe) {
            apply_recipe(&mut resources, &cobalt_recipe);
        }
    }
    let batteries_after_cobalt = resources.get(ResourceType::BatteryPack);
    assert!(
        batteries_after_cobalt >= 5,
        "Should have at least 5 batteries"
    );

    // Switch to Solar + Lithium recipe
    let solar_recipe =
        ProductionChains::get_recipe(ResourceType::SolarPower, ResourceType::BatteryPack)
            .expect("Solar + Lithium recipe exists");

    // Produce 5 more batteries with solar recipe
    for _ in 0..5 {
        if can_apply(&resources, &solar_recipe) {
            apply_recipe(&mut resources, &solar_recipe);
        }
    }
    let batteries_after_solar = resources.get(ResourceType::BatteryPack);
    assert!(
        batteries_after_solar > batteries_after_cobalt,
        "Should have more batteries after switching recipe"
    );

    // Verify both Cobalt and SolarPower were consumed (proving recipe switch)
    assert!(resources.get(ResourceType::Cobalt) < 50, "Cobalt consumed");
    assert!(
        resources.get(ResourceType::SolarPower) < 50,
        "Solar power consumed"
    );
    assert!(
        batteries_after_solar >= 10,
        "Should have at least 10 batteries total"
    );
}

#[test]
fn hundred_turn_complex_production_simulation() {
    // Comprehensive 100-turn simulation with multiple production chains
    let mut resources = Resources::new();

    // Initial resource setup
    resources.set(ResourceType::IronOre, 500);
    resources.set(ResourceType::Coal, 200);
    resources.set(ResourceType::Nickel, 150);
    resources.set(ResourceType::CopperOre, 300);
    resources.set(ResourceType::Lithium, 200);
    resources.set(ResourceType::Cobalt, 100);
    resources.set(ResourceType::SolarPower, 150);
    resources.set(ResourceType::Uranium, 100);
    resources.set(ResourceType::Silicon, 100);
    resources.set(ResourceType::Titanium, 100);

    // Get all production recipes
    let steel_basic = ProductionChains::get_recipe(ResourceType::IronOre, ResourceType::Steel)
        .expect("Steel recipe exists");
    let electronics =
        ProductionChains::get_recipe(ResourceType::CopperOre, ResourceType::Electronics)
            .expect("Electronics recipe exists");
    let battery = ProductionChains::get_recipe(ResourceType::Lithium, ResourceType::BatteryPack)
        .expect("Battery recipe exists");
    let solar = ProductionChains::get_recipe(ResourceType::Silicon, ResourceType::SolarPower)
        .expect("Solar power recipe exists");

    let mut steel_produced = 0;
    let mut electronics_produced = 0;
    let mut batteries_produced = 0;

    // Run 100 turn simulation
    for turn in 0..100 {
        let mut actions_this_turn = 0;

        // Try to produce steel
        if can_apply(&resources, &steel_basic) {
            apply_recipe(&mut resources, &steel_basic);
            steel_produced += 1;
            actions_this_turn += 1;
        }

        // Try to produce electronics
        if can_apply(&resources, &electronics) {
            apply_recipe(&mut resources, &electronics);
            electronics_produced += 1;
            actions_this_turn += 1;
        }

        // Try to produce batteries
        if can_apply(&resources, &battery) {
            apply_recipe(&mut resources, &battery);
            batteries_produced += 1;
            actions_this_turn += 1;
        }

        // Try to produce solar power (renewable)
        if can_apply(&resources, &solar) && turn % 5 == 0 {
            apply_recipe(&mut resources, &solar);
            actions_this_turn += 1;
        }

        // Verify no negative resources
        for resource_type in [
            ResourceType::IronOre,
            ResourceType::Coal,
            ResourceType::Steel,
            ResourceType::Electronics,
            ResourceType::BatteryPack,
        ] {
            assert!(
                resources.get(resource_type) >= 0,
                "Turn {}: Resource {:?} should not be negative",
                turn,
                resource_type
            );
        }
    }

    // Verify production occurred
    assert!(steel_produced > 0, "Should have produced steel");
    assert!(electronics_produced > 0, "Should have produced electronics");
    assert!(batteries_produced > 0, "Should have produced batteries");

    // Verify final resource counts make sense
    assert!(
        resources.get(ResourceType::Steel) > 0,
        "Should have steel in stock"
    );
    assert!(
        resources.get(ResourceType::Electronics) > 0,
        "Should have electronics in stock"
    );
    assert!(
        resources.get(ResourceType::BatteryPack) > 0,
        "Should have batteries in stock"
    );
}

#[test]
fn thousand_buildings_performance_test() {
    // Performance test: simulate 1000 buildings producing for multiple turns
    // Target: < 1 second per turn
    use std::time::Instant;

    let mut resources = Resources::new();

    // Massive resource pool for 1000 buildings
    resources.set(ResourceType::IronOre, 50000);
    resources.set(ResourceType::Coal, 20000);
    resources.set(ResourceType::Nickel, 15000);
    resources.set(ResourceType::CopperOre, 30000);
    resources.set(ResourceType::Lithium, 20000);
    resources.set(ResourceType::Cobalt, 10000);

    // Create a mix of production recipes (simulating 1000 buildings)
    let mut building_recipes = Vec::new();

    // 400 steel buildings (mix of methods)
    let steel_basic = ProductionChains::get_recipe(ResourceType::IronOre, ResourceType::Steel)
        .expect("Steel recipe");
    for _ in 0..400 {
        building_recipes.push(steel_basic.clone());
    }

    // 300 electronics buildings
    let electronics =
        ProductionChains::get_recipe(ResourceType::CopperOre, ResourceType::Electronics)
            .expect("Electronics recipe");
    for _ in 0..300 {
        building_recipes.push(electronics.clone());
    }

    // 300 battery buildings
    let battery = ProductionChains::get_recipe(ResourceType::Lithium, ResourceType::BatteryPack)
        .expect("Battery recipe");
    for _ in 0..300 {
        building_recipes.push(battery.clone());
    }

    assert_eq!(
        building_recipes.len(),
        1000,
        "Should have exactly 1000 buildings"
    );

    // Run simulation for 5 turns and measure performance
    let mut turn_times = Vec::new();

    for turn in 0..5 {
        let turn_start = Instant::now();

        let mut applied = 0;
        // Process all 1000 buildings
        for recipe in &building_recipes {
            if can_apply(&resources, recipe) {
                apply_recipe(&mut resources, recipe);
                applied += 1;
            }
        }

        let turn_duration = turn_start.elapsed();
        turn_times.push(turn_duration);

        println!(
            "Turn {}: Processed 1000 buildings in {:?} ({} recipes applied)",
            turn, turn_duration, applied
        );

        // Performance requirement: < 1 second per turn
        assert!(
            turn_duration.as_secs() < 1,
            "Turn {} took {:?}, should be < 1s",
            turn,
            turn_duration
        );
    }

    // Calculate average turn time
    let avg_time = turn_times.iter().sum::<std::time::Duration>() / turn_times.len() as u32;
    println!("Average turn time for 1000 buildings: {:?}", avg_time);

    // Verify resources were consumed and produced
    assert!(
        resources.get(ResourceType::Steel) > 0,
        "Steel should be produced"
    );
    assert!(
        resources.get(ResourceType::Electronics) > 0,
        "Electronics should be produced"
    );
    assert!(
        resources.get(ResourceType::BatteryPack) > 0,
        "Batteries should be produced"
    );
    assert!(
        resources.get(ResourceType::IronOre) < 50000,
        "Iron ore should be consumed"
    );
}
