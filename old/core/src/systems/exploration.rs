use strum::IntoEnumIterator;

use crate::domain::*;
use rand::{prelude::*, seq::SliceRandom};
use strum::{Display as StrumDisplay, EnumIter};

// pub const SPECTRAL_CLASSES: Vec<&'static str> = vec!["O", "B", "A", "F", "G", "K", "M"];

#[derive(EnumIter, Debug, Clone, Copy, StrumDisplay, PartialEq, Eq, Hash)]
pub enum SpectralClass {
    O,
    B,
    A,
    F,
    G,
    K,
    M,
}

#[derive(EnumIter, Debug, Clone, Copy, StrumDisplay, PartialEq, Eq, Hash)]
pub enum CelestialBodyType {
    Star,
    Planet,
    AsteroidBelt,
}

pub fn calculate_travel_time(distance_ly: f32) -> f64 {
    // simple modeL: 1 ly = 100 game hours
    (distance_ly as f64) * 100.0
}

// Procedurally generate a star system
// Note: this algorithm assumes a single star in the center of the system.
pub fn generate_system(system_id: StarSystemId) -> StarSystem {
    let mut rng = rand::rng();

    let spectral_classes: Vec<SpectralClass> = SpectralClass::iter().collect();
    let mut body_types: Vec<CelestialBodyType> = CelestialBodyType::iter().collect();
    body_types.retain(|&t| t != CelestialBodyType::Star);
    let spectral_class = spectral_classes.choose(&mut rng).unwrap().to_string();

    let num_bodies = rng.random_range(1..=8); // 1 to 8 bodies
    let bodies = (0..num_bodies)
        .map(|i| CelestialBody {
            id: CelestialBodyId::new(),
            name: format!("Body-{}", i + 1),
            body_type: if i == 0 {
                CelestialBodyType::Star.to_string()
            } else {
                body_types.choose(&mut rng).unwrap().to_string()
            },
        })
        .collect::<Vec<_>>();

    // 1 to 8 bodies
    let name = format!("System-{}", system_id.0.to_string());
    StarSystem {
        id: system_id,
        name,
        spectral_class,
        bodies,
    }
}
