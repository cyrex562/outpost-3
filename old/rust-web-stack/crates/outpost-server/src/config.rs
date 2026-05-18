use serde::{Deserialize, Serialize};
use std::path::Path;

/// Application configuration for Outpost 3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub game: GameConfig,
}

/// Web server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

/// Game simulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    // Legacy settings (will be migrated to V5 model)
    pub starting_credits: i64,
    #[serde(default = "default_turn_duration")]
    pub turn_duration_seconds: u32,
    
    // V5 Time System Settings
    /// Milliseconds per game tick (default: 60000 = 1 minute real-time per tick)
    /// Each tick represents ~1 game hour
    #[serde(default = "default_tick_rate_ms")]
    pub tick_rate_ms: u64,
    
    /// Default speed multiplier on game start (1x, 2x, 5x, 10x)
    #[serde(default = "default_speed_multiplier")]
    pub default_speed_multiplier: u8,
    
    /// Maximum allowed speed multiplier
    #[serde(default = "default_max_speed")]
    pub max_speed_multiplier: u8,
    
    // V5 Idle Safety Settings
    /// Enable idle safety mode (prevents catastrophic failures when player is away)
    #[serde(default = "default_idle_safety")]
    pub idle_safety_enabled: bool,
    
    /// Suppress auto-pause events when idle safety is enabled
    #[serde(default = "default_suppress_autopause")]
    pub suppress_autopause_in_idle_mode: bool,
    
    // V5 Auto-Save Settings
    /// Auto-save interval in game ticks (0 = disabled)
    #[serde(default = "default_autosave_interval")]
    pub auto_save_interval_ticks: u64,
    
    /// Save directory path (relative to game data directory)
    #[serde(default = "default_save_directory")]
    pub save_directory: String,
    
    /// Maximum number of auto-save slots to keep
    #[serde(default = "default_max_autosaves")]
    pub max_autosaves: usize,
}

// Default value functions for serde
fn default_turn_duration() -> u32 { 60 }
fn default_tick_rate_ms() -> u64 { 60000 } // 1 minute real-time = 1 game hour
fn default_speed_multiplier() -> u8 { 1 }
fn default_max_speed() -> u8 { 10 }
fn default_idle_safety() -> bool { true }
fn default_suppress_autopause() -> bool { true }
fn default_autosave_interval() -> u64 { 10 } // Every 10 ticks = 10 game hours
fn default_save_directory() -> String { "saves".to_string() }
fn default_max_autosaves() -> usize { 5 }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            game: GameConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8083,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "outpost3.db".to_string(),
        }
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            starting_credits: 10000,
            turn_duration_seconds: default_turn_duration(),
            tick_rate_ms: default_tick_rate_ms(),
            default_speed_multiplier: default_speed_multiplier(),
            max_speed_multiplier: default_max_speed(),
            idle_safety_enabled: default_idle_safety(),
            suppress_autopause_in_idle_mode: default_suppress_autopause(),
            auto_save_interval_ticks: default_autosave_interval(),
            save_directory: default_save_directory(),
            max_autosaves: default_max_autosaves(),
        }
    }
}

impl AppConfig {
    /// Load configuration from file, environment variables, and defaults
    /// Priority: Environment Variables > Config File > Defaults
    pub fn load() -> anyhow::Result<Self> {
        Self::load_from_file("config.toml")
    }
    
    /// Load configuration from a specific file path
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path_ref = path.as_ref();
        
        let builder = config::Config::builder()
            // Start with defaults
            .add_source(config::Config::try_from(&Self::default())?)
            // Add config file if it exists
            .add_source(
                config::File::from(path_ref)
                    .required(false)
            )
            // Add environment variables with prefix "OUTPOST3_"
            // Example: OUTPOST3_SERVER__PORT=8080
            .add_source(
                config::Environment::with_prefix("OUTPOST3")
                    .separator("__")
                    .try_parsing(true)
            );
        
        let config = builder.build()?;
        let app_config: AppConfig = config.try_deserialize()?;
        
        // Validate configuration
        app_config.validate()?;
        
        Ok(app_config)
    }
    
    /// Validate configuration values
    fn validate(&self) -> anyhow::Result<()> {
        // Validate speed multipliers
        if self.game.default_speed_multiplier < 1 {
            anyhow::bail!("default_speed_multiplier must be at least 1");
        }
        if self.game.max_speed_multiplier < self.game.default_speed_multiplier {
            anyhow::bail!("max_speed_multiplier must be >= default_speed_multiplier");
        }
        if self.game.max_speed_multiplier > 100 {
            anyhow::bail!("max_speed_multiplier cannot exceed 100");
        }
        
        // Validate tick rate
        if self.game.tick_rate_ms == 0 {
            anyhow::bail!("tick_rate_ms must be greater than 0");
        }
        if self.game.tick_rate_ms > 3600000 {
            anyhow::bail!("tick_rate_ms cannot exceed 1 hour (3600000ms)");
        }
        
        // Validate server port
        if self.server.port == 0 {
            anyhow::bail!("server port must be greater than 0");
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config_is_valid() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_default_v5_settings() {
        let config = GameConfig::default();
        
        assert_eq!(config.tick_rate_ms, 60000); // 1 minute real-time
        assert_eq!(config.default_speed_multiplier, 1);
        assert_eq!(config.max_speed_multiplier, 10);
        assert_eq!(config.idle_safety_enabled, true);
        assert_eq!(config.suppress_autopause_in_idle_mode, true);
        assert_eq!(config.auto_save_interval_ticks, 10);
        assert_eq!(config.save_directory, "saves");
        assert_eq!(config.max_autosaves, 5);
    }
    
    #[test]
    fn test_invalid_speed_multiplier() {
        let mut config = AppConfig::default();
        config.game.default_speed_multiplier = 0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_invalid_max_speed_less_than_default() {
        let mut config = AppConfig::default();
        config.game.default_speed_multiplier = 5;
        config.game.max_speed_multiplier = 2;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_invalid_tick_rate_zero() {
        let mut config = AppConfig::default();
        config.game.tick_rate_ms = 0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_invalid_tick_rate_too_high() {
        let mut config = AppConfig::default();
        config.game.tick_rate_ms = 7200000; // 2 hours
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("tick_rate_ms"));
        assert!(toml_str.contains("idle_safety_enabled"));
        assert!(toml_str.contains("auto_save_interval_ticks"));
    }
    
    #[test]
    fn test_load_from_nonexistent_file_uses_defaults() {
        // Loading from a non-existent file should succeed with defaults
        let config = AppConfig::load_from_file("nonexistent_config.toml").unwrap();
        assert_eq!(config.game.tick_rate_ms, 60000);
        assert_eq!(config.game.idle_safety_enabled, true);
        assert_eq!(config.server.port, 8083);
    }
    
    #[test]
    fn test_config_roundtrip() {
        // Serialize to TOML and deserialize back
        let original = AppConfig::default();
        let toml_str = toml::to_string(&original).unwrap();
        let deserialized: AppConfig = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(original.game.tick_rate_ms, deserialized.game.tick_rate_ms);
        assert_eq!(original.game.idle_safety_enabled, deserialized.game.idle_safety_enabled);
        assert_eq!(original.game.auto_save_interval_ticks, deserialized.game.auto_save_interval_ticks);
    }
}
