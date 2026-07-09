//! Procedural dungeon generator.
//!
//! Ported from `src/harsh_realm/generators/dungeon_gen.py`. Room and trap types
//! are content (loaded via [`super::content_tables`]); a caller may override the
//! room types for a setting/biome.

use rand::Rng;
use uuid::Uuid;

use crate::runtime::{JsonObject, JsonValue};
use crate::scene_data::{DungeonConnection, DungeonRoom};

use super::content_tables::load_table_results;

const DIRECTIONS: [&str; 4] = ["north", "south", "east", "west"];

/// Generates procedural dungeon layouts.
pub struct DungeonGenerator<R: Rng> {
    rng: R,
    room_types: Vec<String>,
    trap_types: Vec<String>,
}

impl<R: Rng> DungeonGenerator<R> {
    /// Create a generator with an RNG and optional room-type override.
    pub fn new(rng: R, room_types: Option<Vec<String>>) -> Self {
        let room_types = room_types.unwrap_or_else(|| {
            load_table_results("tables/dungeon_room_types.yaml", &["chamber".to_string()])
        });
        let trap_types = load_table_results(
            "tables/dungeon_trap_types.yaml",
            &["pit".to_string(), "dart".to_string(), "gas".to_string()],
        );
        DungeonGenerator {
            rng,
            room_types,
            trap_types,
        }
    }

    /// Generate a random dungeon layout, returning `(rooms, connections)`.
    pub fn generate(
        &mut self,
        dungeon_id: Option<&str>,
        num_rooms: i32,
    ) -> (Vec<DungeonRoom>, Vec<DungeonConnection>) {
        let _dungeon_id = dungeon_id
            .map(str::to_string)
            .unwrap_or_else(|| format!("dungeon_{}", &Uuid::new_v4().simple().to_string()[..8]));

        let mut rooms: Vec<DungeonRoom> = Vec::new();
        let mut connections: Vec<DungeonConnection> = Vec::new();

        rooms.push(make_room(
            "entrance",
            "Dungeon Entrance",
            "entrance",
            "A cold draft blows from the depths. The surface is just behind you.",
        ));

        let mut prev_room_id = "entrance".to_string();
        let mut available: Vec<&str> = DIRECTIONS.to_vec();

        for i in 1..num_rooms {
            let room_id = format!("room_{i}");
            let rtype = self.room_types[self.rng.gen_range(0..self.room_types.len())].clone();

            let mut room = make_room(
                &room_id,
                &format!("{} {i}", capitalize(&rtype)),
                &rtype,
                &format!("A damp {rtype} with stone walls."),
            );

            if self.rng.gen::<f64>() < 0.3 {
                let trap = self.trap_types[self.rng.gen_range(0..self.trap_types.len())].clone();
                let mut payload = JsonObject::new();
                payload.insert("type".into(), JsonValue::from(trap));
                payload.insert("notice_diff".into(), JsonValue::from(8));
                payload.insert("avoid_diff".into(), JsonValue::from(12));
                payload.insert("damage".into(), JsonValue::from("1d6"));
                room.traps = vec![payload];
            }

            if self.rng.gen::<f64>() < 0.4 {
                let mut item = JsonObject::new();
                item.insert("name".into(), JsonValue::from("Old Coin"));
                item.insert("type".into(), JsonValue::from("misc"));
                item.insert("data".into(), JsonValue::Object(JsonObject::new()));
                let mut payload = JsonObject::new();
                payload.insert("item".into(), JsonValue::Object(item));
                payload.insert("difficulty".into(), JsonValue::from(9));
                room.hidden_loot = vec![payload];
            }

            rooms.push(room);

            if available.is_empty() {
                available = DIRECTIONS.to_vec();
            }
            let idx = self.rng.gen_range(0..available.len());
            let dir_from = available.remove(idx);

            connections.push(DungeonConnection {
                from_room: prev_room_id.clone(),
                to_room: room_id.clone(),
                direction: dir_from.to_string(),
            });
            prev_room_id = room_id;
        }

        (rooms, connections)
    }
}

fn make_room(id: &str, name: &str, rtype: &str, description: &str) -> DungeonRoom {
    DungeonRoom {
        id: id.to_string(),
        name: name.to_string(),
        r#type: rtype.to_string(),
        x: None,
        y: None,
        w: None,
        h: None,
        center_q: None,
        center_r: None,
        editor_pos: None,
        description: Some(description.to_string()),
        loot: Vec::new(),
        hidden_loot: Vec::new(),
        traps: Vec::new(),
        encounter: None,
        data: JsonObject::new(),
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn generates_rooms_and_connections() {
        let mut gen = DungeonGenerator::new(
            SmallRng::seed_from_u64(1),
            Some(vec!["cell".to_string(), "hall".to_string()]),
        );
        let (rooms, connections) = gen.generate(Some("d1"), 5);
        assert_eq!(rooms.len(), 5);
        assert_eq!(rooms[0].id, "entrance");
        // Each non-entrance room is connected to the previous.
        assert_eq!(connections.len(), 4);
        assert_eq!(connections[0].from_room, "entrance");
        assert_eq!(connections[0].to_room, "room_1");
        for c in &connections {
            assert!(DIRECTIONS.contains(&c.direction.as_str()));
        }
    }

    #[test]
    fn single_room_has_no_connections() {
        let mut gen = DungeonGenerator::new(SmallRng::seed_from_u64(2), Some(vec!["cell".into()]));
        let (rooms, connections) = gen.generate(None, 1);
        assert_eq!(rooms.len(), 1);
        assert!(connections.is_empty());
    }

    #[test]
    fn room_names_are_capitalized() {
        let mut gen = DungeonGenerator::new(SmallRng::seed_from_u64(3), Some(vec!["crypt".into()]));
        let (rooms, _) = gen.generate(None, 2);
        assert_eq!(rooms[1].name, "Crypt 1");
        assert_eq!(rooms[1].r#type, "crypt");
    }
}
