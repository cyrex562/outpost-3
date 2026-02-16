use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub game: GameConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub starting_credits: i64,
    pub turn_duration_seconds: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8083,
            },
            database: DatabaseConfig {
                path: "outpost3.db".to_string(),
            },
            game: GameConfig {
                starting_credits: 10000,
                turn_duration_seconds: 60,
            },
        }
    }
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        // For now, just use defaults. Later, can load from file
        Ok(Self::default())
    }
}
