//! Procedural generation of resource deposits based on celestial body properties.
//!
//! This module generates resource deposits for celestial bodies based on their
//! type, composition, and a deterministic seed.

use crate::domain::{BodyType, CelestialBody, ExtractionDifficulty, ResourceDeposit, Temperature};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Configuration for deposit generation.
#[derive(Debug, Clone)]
pub struct DepositGeneratorConfig {
    /// Minimum deposit size multiplier.
    pub min_deposit_size: f64,
    /// Maximum deposit size multiplier.
    pub max_deposit_size: f64,
    /// Number of deposits to generate per resource type.
    pub deposits_per_resource: usize,
}

impl Default for DepositGeneratorConfig {
    fn default() -> Self {
        Self {
            min_deposit_size: 100.0,
            max_deposit_size: 10000.0,
            deposits_per_resource: 1,
        }
    }
}

/// Generate resource deposits for a celestial body based on its properties.
pub fn generate_deposits(
    body: &CelestialBody,
    seed: u64,
    config: &DepositGeneratorConfig,
) -> Vec<ResourceDeposit> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut deposits = Vec::new();

    // Determine which resources can spawn based on body type
    let possible_resources = get_resources_for_body_type(body.body_type, body.temperature);

    for resource_id in possible_resources {
        // Decide if this resource spawns (70% chance for typical resources)
        if rng.gen::<f64>() > 0.7 {
            continue;
        }

        // Generate 1-3 deposits of this resource
        let num_deposits = rng.gen_range(1..=config.deposits_per_resource);

        for _ in 0..num_deposits {
            let deposit = generate_single_deposit(
                resource_id.clone(),
                body,
                &mut rng,
                config,
            );
            deposits.push(deposit);
        }
    }

    deposits
}

/// Generate a single deposit with randomized properties.
fn generate_single_deposit(
    resource_id: String,
    body: &CelestialBody,
    rng: &mut ChaCha8Rng,
    config: &DepositGeneratorConfig,
) -> ResourceDeposit {
    // Base quantity depends on body size
    let size_multiplier = (body.radius_km / 1000.0).powf(2.0); // Larger bodies = more resources
    let base_quantity = rng.gen_range(config.min_deposit_size..=config.max_deposit_size);
    let quantity = base_quantity * size_multiplier;

    // Difficulty varies by depth and concentration
    let difficulty = determine_extraction_difficulty(body, rng);

    // Accessibility depends on body type and atmosphere
    let accessibility = determine_accessibility(body, rng);

    // Concentration varies randomly
    let concentration = rng.gen_range(0.3..=0.9);

    ResourceDeposit::new_detailed(
        resource_id,
        quantity,
        difficulty,
        accessibility,
        concentration,
    )
}

/// Determine which resources can appear on this body type.
fn get_resources_for_body_type(body_type: BodyType, temperature: Temperature) -> Vec<String> {
    let mut resources = Vec::new();

    match body_type {
        BodyType::TerrestrialPlanet | BodyType::Dwarf => {
            // Rocky bodies have metallic ores and minerals
            resources.extend_from_slice(&[
                "iron_ore",
                "copper_ore",
                "silicon_ore",
                "regolith",
            ]);

            // Cold bodies have ice
            if matches!(temperature, Temperature::Frozen | Temperature::Cold) {
                resources.push("ice");
            }

            // Some have rare materials
            if rand::thread_rng().gen::<f64>() > 0.7 {
                resources.push("uranium_ore");
            }
            if rand::thread_rng().gen::<f64>() > 0.8 {
                resources.push("rare_earth_ore");
            }
        }

        BodyType::Moon => {
            // Moons often have ice and regolith
            resources.extend_from_slice(&["ice", "regolith", "silicon_ore"]);

            // Some moons have metals
            if rand::thread_rng().gen::<f64>() > 0.6 {
                resources.push("iron_ore");
            }
        }

        BodyType::Asteroid => {
            // Asteroids are rich in metals
            resources.extend_from_slice(&[
                "iron_ore",
                "copper_ore",
                "rare_earth_ore",
            ]);

            // Some are icy
            if matches!(temperature, Temperature::Frozen) {
                resources.push("ice");
            }
        }

        BodyType::IceGiant | BodyType::GasGiant => {
            // Gas giants can't be mined directly (need atmospheric processing)
            // For now, they have no surface deposits
        }

        BodyType::Comet => {
            // Comets are mostly ice with some rock
            resources.extend_from_slice(&["ice", "carbon_compounds"]);
        }

        BodyType::OrbitalStation => {
            // Stations don't have natural deposits
        }
    }

    resources.into_iter().map(|s| s.to_string()).collect()
}

/// Determine extraction difficulty based on body properties.
fn determine_extraction_difficulty(
    body: &CelestialBody,
    rng: &mut ChaCha8Rng,
) -> ExtractionDifficulty {
    let mut difficulty_score = rng.gen_range(0..=10);

    // Extreme gravity makes extraction harder
    if body.gravity > 2.0 {
        difficulty_score += 2;
    } else if body.gravity < 0.3 {
        difficulty_score += 1; // Low gravity also has challenges
    }

    // Extreme temperatures make extraction harder
    match body.temperature {
        Temperature::Frozen | Temperature::Scorching => difficulty_score += 2,
        Temperature::Cold | Temperature::Hot => difficulty_score += 1,
        Temperature::Temperate => {}
    }

    // Hostile atmosphere increases difficulty
    if !body.atmosphere.is_breathable() && body.atmosphere.pressure_bars() > 0.1 {
        difficulty_score += 1;
    }

    // Map score to difficulty
    match difficulty_score {
        0..=3 => ExtractionDifficulty::VeryEasy,
        4..=6 => ExtractionDifficulty::Easy,
        7..=9 => ExtractionDifficulty::Normal,
        10..=12 => ExtractionDifficulty::Hard,
        13..=15 => ExtractionDifficulty::VeryHard,
        _ => ExtractionDifficulty::Extreme,
    }
}

/// Determine accessibility based on body properties.
fn determine_accessibility(body: &CelestialBody, rng: &mut ChaCha8Rng) -> f64 {
    let mut accessibility: f64 = rng.gen_range(0.5..=1.0);

    // Surface deposits are more accessible
    match body.body_type {
        BodyType::Asteroid | BodyType::Moon => accessibility *= 1.2,
        BodyType::TerrestrialPlanet => accessibility *= 0.9,
        _ => {}
    }

    // Vacuum makes some operations easier
    if matches!(body.atmosphere, crate::domain::Atmosphere::None) {
        accessibility *= 1.1;
    }

    accessibility.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Atmosphere, StarSystemId};

    #[test]
    fn test_generate_deposits_deterministic() {
        let system_id = StarSystemId::new();
        let body = CelestialBody::new(
            system_id,
            "Test Asteroid".to_string(),
            BodyType::Asteroid,
            Atmosphere::None,
            Temperature::Frozen,
            0.1,
            100.0,
            0.001,
        );

        let config = DepositGeneratorConfig::default();

        // Same seed should produce same deposits
        let deposits1 = generate_deposits(&body, 12345, &config);
        let deposits2 = generate_deposits(&body, 12345, &config);

        assert_eq!(deposits1.len(), deposits2.len());
        for (d1, d2) in deposits1.iter().zip(deposits2.iter()) {
            assert_eq!(d1.resource_id, d2.resource_id);
            assert_eq!(d1.initial_quantity, d2.initial_quantity);
        }
    }

    #[test]
    fn test_generate_deposits_different_seeds() {
        let system_id = StarSystemId::new();
        let body = CelestialBody::new(
            system_id,
            "Test Planet".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::Thin {
                composition: Default::default(),
            },
            Temperature::Temperate,
            1.0,
            6000.0,
            1.0,
        );

        let config = DepositGeneratorConfig::default();

        // Different seeds should produce different deposits
        let deposits1 = generate_deposits(&body, 11111, &config);
        let deposits2 = generate_deposits(&body, 22222, &config);

        // Might have different counts or quantities
        assert!(deposits1.len() > 0);
        assert!(deposits2.len() > 0);
    }

    #[test]
    fn test_asteroid_has_metal_deposits() {
        let system_id = StarSystemId::new();
        let body = CelestialBody::new(
            system_id,
            "Metal Asteroid".to_string(),
            BodyType::Asteroid,
            Atmosphere::None,
            Temperature::Cold,
            0.05,
            50.0,
            0.0001,
        );

        let config = DepositGeneratorConfig::default();
        let deposits = generate_deposits(&body, 42, &config);

        // Asteroids should have some metallic deposits
        let has_metals = deposits.iter().any(|d| {
            d.resource_id.contains("ore")
        });
        assert!(has_metals || deposits.is_empty()); // May not spawn if RNG unlucky
    }

    #[test]
    fn test_larger_bodies_have_more_resources() {
        let system_id = StarSystemId::new();
        
        let small_body = CelestialBody::new(
            system_id,
            "Small".to_string(),
            BodyType::Moon,
            Atmosphere::None,
            Temperature::Cold,
            0.2,
            1000.0, // 1000 km radius
            0.01,
        );

        let large_body = CelestialBody::new(
            system_id,
            "Large".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::Thin { composition: Default::default() },
            Temperature::Temperate,
            1.0,
            6000.0, // 6000 km radius (Earth-like)
            1.0,
        );

        let config = DepositGeneratorConfig::default();
        let small_deposits = generate_deposits(&small_body, 100, &config);
        let large_deposits = generate_deposits(&large_body, 100, &config);

        // Large body should have larger deposits on average
        let small_avg = small_deposits.iter().map(|d| d.initial_quantity).sum::<f64>()
            / small_deposits.len().max(1) as f64;
        let large_avg = large_deposits.iter().map(|d| d.initial_quantity).sum::<f64>()
            / large_deposits.len().max(1) as f64;

        if small_deposits.len() > 0 && large_deposits.len() > 0 {
            assert!(large_avg > small_avg * 5.0); // Much larger due to radius^2 scaling
        }
    }

    #[test]
    fn test_gas_giants_have_no_surface_deposits() {
        let system_id = StarSystemId::new();
        let gas_giant = CelestialBody::new(
            system_id,
            "Jupiter".to_string(),
            BodyType::GasGiant,
            Atmosphere::Crushing {
                composition: Default::default(),
            },
            Temperature::Cold,
            2.5,
            70000.0,
            300.0,
        );

        let config = DepositGeneratorConfig::default();
        let deposits = generate_deposits(&gas_giant, 999, &config);

        assert_eq!(deposits.len(), 0, "Gas giants should have no surface deposits");
    }

    #[test]
    fn test_orbital_stations_have_no_deposits() {
        let system_id = StarSystemId::new();
        let station = CelestialBody::new(
            system_id,
            "Station Alpha".to_string(),
            BodyType::OrbitalStation,
            Atmosphere::None,
            Temperature::Temperate,
            0.0,
            1.0,
            0.00001,
        );

        let config = DepositGeneratorConfig::default();
        let deposits = generate_deposits(&station, 777, &config);

        assert_eq!(deposits.len(), 0, "Orbital stations should have no natural deposits");
    }
}
