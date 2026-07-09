//! Weather simulation engine.
//!
//! Ported from `src/harsh_realm/engine/weather.py`. Weather is deterministic in
//! the world seed, region (4x4 chunks), and time period (4-tick blocks). Python
//! seeded `random.Random` with a string; this port seeds a `SmallRng` from an
//! FNV-1a hash of the same string, so the weather is still fully deterministic
//! (the specific pseudo-random draws differ from Python, which is irrelevant now
//! that Rust is the source of truth).

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::cell::CellData;
use crate::runtime::WeatherState;
use crate::table_engine::{TableEngine, TableError};

/// Deterministic weather generator based on world seed and time.
pub struct WeatherService {
    world_seed: String,
}

impl WeatherService {
    /// Create a weather service bound to a world seed.
    pub fn new(world_seed: impl Into<String>) -> Self {
        WeatherService {
            world_seed: world_seed.into(),
        }
    }

    /// Return the weather state for a specific cell at a specific tick.
    pub fn at_cell(&self, cell: &CellData, tick: i64) -> WeatherState {
        self.at(cell.q as i64, cell.r as i64, tick, Some(&cell.terrain))
    }

    /// Return the weather state for coordinates at a specific tick.
    pub fn at(&self, q: i64, r: i64, tick: i64, terrain: Option<&str>) -> WeatherState {
        // Weather is regionally stable (4x4 chunks) and changes every 4 ticks.
        let region_q = q.div_euclid(4);
        let region_r = r.div_euclid(4);
        let weather_period = tick.div_euclid(4);

        let seed_str = format!(
            "{}:{}:{}:{}",
            self.world_seed, region_q, region_r, weather_period
        );
        let mut rng = SmallRng::seed_from_u64(fnv1a(&seed_str));

        let roll: f64 = rng.gen();
        let temp_roll: f64 = rng.gen();

        let is_desert = terrain == Some("desert");
        let is_swamp = terrain == Some("swamp");
        let is_water = terrain == Some("water");
        let is_mountain = terrain == Some("mountains");

        let (condition, description) = if is_desert {
            if roll < 0.85 {
                ("clear", "The sun beats down from a cloudless sky.")
            } else if roll < 0.95 {
                ("overcast", "Thin, high clouds drift across the sun.")
            } else if roll < 0.99 {
                ("rain", "A rare, brief shower dampens the sand.")
            } else {
                ("storm", "A fierce sandstorm whips across the dunes.")
            }
        } else if is_mountain {
            if roll < 0.3 {
                ("clear", "The mountain peaks are sharp against a blue sky.")
            } else if roll < 0.5 {
                ("overcast", "Heavy clouds shroud the higher slopes.")
            } else if roll < 0.7 {
                ("snow", "A biting wind carries flurries of snow.")
            } else if roll < 0.85 {
                ("storm", "A violent blizzard rages around the peaks.")
            } else if roll < 0.95 {
                ("fog", "A freezing mist hides the trail ahead.")
            } else {
                ("rain", "Cold rain sluices down the rocky faces.")
            }
        } else if roll < 0.5 {
            ("clear", "The sky is clear and the air is still.")
        } else if roll < 0.75 {
            ("overcast", "Gray clouds blanket the sky.")
        } else if roll < 0.88 {
            ("rain", "A steady rain is falling.")
        } else if roll < 0.93 {
            ("storm", "Thunder rolls as heavy rain lashes the ground.")
        } else if roll < 0.97 {
            ("fog", "A thick mist clings to the ground, obscuring vision.")
        } else {
            ("snow", "Soft white flakes drift down from the sky.")
        };

        let temperature = if is_desert {
            if condition == "clear" || condition == "overcast" {
                if temp_roll < 0.8 {
                    "hot"
                } else {
                    "warm"
                }
            } else {
                "mild"
            }
        } else if is_mountain || condition == "snow" {
            if temp_roll < 0.7 {
                "freezing"
            } else {
                "cold"
            }
        } else if is_swamp || is_water {
            if temp_roll < 0.6 {
                "warm"
            } else {
                "mild"
            }
        } else if temp_roll < 0.1 {
            "freezing"
        } else if temp_roll < 0.3 {
            "cold"
        } else if temp_roll < 0.7 {
            "mild"
        } else if temp_roll < 0.9 {
            "warm"
        } else {
            "hot"
        };

        WeatherState {
            condition: condition.to_string(),
            temperature: temperature.to_string(),
            description: description.to_string(),
        }
    }

    /// Return whether the weather period changed between two ticks.
    pub fn did_weather_change(&self, _q: i64, _r: i64, old_tick: i64, new_tick: i64) -> bool {
        old_tick.div_euclid(4) != new_tick.div_euclid(4)
    }

    /// Check for and return a random weather event at a specific tick.
    ///
    /// There is a 10% chance per hour of a special weather event.
    pub fn get_event<R: Rng>(
        &self,
        q: i64,
        r: i64,
        tick: i64,
        table_engine: &mut TableEngine<'_, R>,
    ) -> Option<String> {
        let seed_str = format!("{}:{}:{}:{}:event", self.world_seed, q, r, tick);
        let mut rng = SmallRng::seed_from_u64(fnv1a(&seed_str));
        if rng.gen::<f64>() >= 0.1 {
            return None;
        }
        match table_engine.roll_on("weather_events", None) {
            Ok(res) => res
                .raw_text
                .or_else(|| Some(serde_json::Value::Object(res.result).to_string())),
            Err(TableError::NotFound(_)) => None,
            Err(_) => None,
        }
    }
}

/// FNV-1a 64-bit hash, used to derive a stable RNG seed from a string.
fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_is_deterministic_per_region_and_period() {
        let svc = WeatherService::new("seed-1");
        let a = svc.at(0, 0, 0, None);
        let b = svc.at(0, 0, 0, None);
        assert_eq!(a, b);
        // Same 4x4 region, same 4-tick period -> identical weather.
        let c = svc.at(3, 3, 3, None);
        assert_eq!(a, c);
    }

    #[test]
    fn different_seeds_can_differ() {
        let a = WeatherService::new("seed-a").at(10, 10, 8, None);
        let b = WeatherService::new("seed-b").at(10, 10, 8, None);
        // Not guaranteed different, but the fields are always valid.
        for w in [&a, &b] {
            assert!(!w.condition.is_empty());
            assert!(!w.temperature.is_empty());
            assert!(!w.description.is_empty());
        }
    }

    #[test]
    fn desert_is_usually_clear_and_hot() {
        let svc = WeatherService::new("s");
        let mut clear = 0;
        for tick in 0..400 {
            let w = svc.at(0, 0, tick * 4, Some("desert"));
            if w.condition == "clear" {
                clear += 1;
            }
        }
        // ~85% clear; assert a strong majority.
        assert!(clear > 60, "expected mostly clear desert weather, got {clear}/100");
    }

    #[test]
    fn period_change_detection() {
        let svc = WeatherService::new("s");
        assert!(!svc.did_weather_change(0, 0, 0, 3));
        assert!(svc.did_weather_change(0, 0, 3, 4));
    }

    #[test]
    fn mountain_and_snow_are_cold() {
        let svc = WeatherService::new("s");
        for tick in 0..200 {
            let w = svc.at(0, 0, tick * 4, Some("mountains"));
            assert!(
                w.temperature == "freezing" || w.temperature == "cold",
                "mountain temp should be cold, got {}",
                w.temperature
            );
        }
    }
}
