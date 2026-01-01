// Database schema definitions
// Tables are created via migrations, but type definitions can go here

pub const EVENTS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS events (
    event_id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    turn_number INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL
);
"#;

pub const COLONIES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS colonies (
    colony_id INTEGER PRIMARY KEY,
    planet_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    founded_at TEXT NOT NULL,
    population INTEGER NOT NULL DEFAULT 100,
    morale REAL NOT NULL DEFAULT 75.0,
    pollution_level REAL NOT NULL DEFAULT 0.0
);
"#;

pub const BUILDINGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS buildings (
    building_id INTEGER PRIMARY KEY,
    colony_id INTEGER NOT NULL,
    building_type TEXT NOT NULL,
    state TEXT NOT NULL,
    workers_assigned INTEGER NOT NULL DEFAULT 0,
    level INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (colony_id) REFERENCES colonies(colony_id)
);
"#;

pub const PLANETS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS planets (
    planet_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    planet_type TEXT NOT NULL,
    atmosphere TEXT NOT NULL,
    temperature TEXT NOT NULL,
    gravity REAL NOT NULL,
    size INTEGER NOT NULL,
    hazard_level INTEGER NOT NULL,
    description TEXT NOT NULL
);
"#;

pub const GAME_STATE_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS game_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    current_turn INTEGER NOT NULL DEFAULT 1,
    credits INTEGER NOT NULL DEFAULT 10000
);
"#;

pub const RESOURCE_STOCKPILES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS resource_stockpiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    colony_id INTEGER NOT NULL,
    resource_type TEXT NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (colony_id) REFERENCES colonies(colony_id),
    UNIQUE(colony_id, resource_type)
);
"#;
