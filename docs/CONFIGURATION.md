# Outpost 3 Configuration Guide

## Configuration Priority

Outpost 3 loads configuration from multiple sources with the following priority (highest to lowest):

1. **Environment Variables** (highest priority)
2. **Configuration File** (`config.toml`)
3. **Default Values** (lowest priority)

## Configuration File

Create a `config.toml` file in the project root directory. See `config.toml.example` for a complete reference.

### Example

```toml
[server]
host = "127.0.0.1"
port = 8081

[database]
path = "outpost3.db"

[game]
tick_rate_ms = 60000
idle_safety_enabled = true
auto_save_interval_ticks = 10
```

## Environment Variables

All configuration values can be overridden using environment variables with the prefix `OUTPOST3_`.

Use double underscores (`__`) to separate nested configuration sections.

### Format

```
OUTPOST3_<SECTION>__<KEY>=<VALUE>
```

### Examples

```bash
# Server configuration
export OUTPOST3_SERVER__HOST="0.0.0.0"
export OUTPOST3_SERVER__PORT=3000

# Database configuration
export OUTPOST3_DATABASE__PATH="/var/lib/outpost3/data.db"

# Game configuration
export OUTPOST3_GAME__TICK_RATE_MS=30000
export OUTPOST3_GAME__IDLE_SAFETY_ENABLED=false
export OUTPOST3_GAME__AUTO_SAVE_INTERVAL_TICKS=20
export OUTPOST3_GAME__MAX_SPEED_MULTIPLIER=100
```

### Type Conversion

Environment variables are automatically converted to the appropriate type:

- Booleans: `true`, `false` (case-insensitive)
- Numbers: Standard integer/float parsing
- Strings: Used as-is

## Configuration Reference

### [server]

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `host` | String | `"127.0.0.1"` | Server bind address |
| `port` | u16 | `8081` | Server port number |

### [database]

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `path` | String | `"outpost3.db"` | SQLite database file path |

### [game]

#### Legacy Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `starting_credits` | i64 | `10000` | Starting credits for new games |
| `turn_duration_seconds` | u32 | `60` | **Deprecated:** Use `tick_rate_ms` instead |

#### V5 Time System

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `tick_rate_ms` | u64 | `60000` | 1-3600000 | Milliseconds per game tick (1 minute real-time = 1 game hour) |
| `default_speed_multiplier` | u8 | `1` | ≥1 | Starting simulation speed |
| `max_speed_multiplier` | u8 | `10` | 1-100 | Maximum allowed speed multiplier |

**Speed Multiplier Values:**

- `1` = Normal speed (1x)
- `2` = Double speed (2x)
- `5` = 5x speed
- `10` = 10x speed

**Tick Rate Examples:**

- `60000` ms (1 minute) = default, balanced for active play
- `30000` ms (30 seconds) = faster progression, more responsive
- `120000` ms (2 minutes) = slower, more contemplative

#### V5 Idle Safety

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `idle_safety_enabled` | bool | `true` | Enable idle safety mode to prevent catastrophic failures |
| `suppress_autopause_in_idle_mode` | bool | `true` | Queue auto-pause events instead of pausing when idle safety is on |

**Idle Safety Features:**

- No colonist deaths from starvation, suffocation, or exposure
- Critical resource shortages trigger automatic rationing/conservation
- Production stops gracefully when inputs are exhausted (no building damage)
- Events requiring player choices are queued for later
- Auto-pause triggers are suppressed (events accumulate)

#### V5 Auto-Save

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `auto_save_interval_ticks` | u64 | `10` | Auto-save every N ticks (0 = disabled) |
| `save_directory` | String | `"saves"` | Save file directory path |
| `max_autosaves` | usize | `5` | Maximum number of auto-save slots to retain |

**Auto-Save Timing:**
With default settings (`tick_rate_ms=60000`, `auto_save_interval_ticks=10`):

- Auto-save occurs every **10 ticks**
- Each tick = 60 seconds real-time
- Auto-save interval = **10 minutes** real-time
- Represents **10 game hours** elapsed

## Validation

The configuration is validated on load. Invalid values will cause the server to fail to start with a descriptive error message.

### Validation Rules

- `tick_rate_ms` must be greater than 0 and not exceed 3,600,000 (1 hour)
- `default_speed_multiplier` must be at least 1
- `max_speed_multiplier` must be ≥ `default_speed_multiplier`
- `max_speed_multiplier` cannot exceed 100
- `server.port` must be greater than 0

## Running with Custom Configuration

### Using Config File

```bash
# Place config.toml in project root
cargo run -p outpost-server
```

### Using Environment Variables

```bash
export OUTPOST3_SERVER__PORT=3000
export OUTPOST3_GAME__IDLE_SAFETY_ENABLED=false
cargo run -p outpost-server
```

### Docker Example

```dockerfile
ENV OUTPOST3_SERVER__HOST=0.0.0.0
ENV OUTPOST3_SERVER__PORT=8080
ENV OUTPOST3_DATABASE__PATH=/data/outpost3.db
ENV OUTPOST3_GAME__TICK_RATE_MS=30000
ENV OUTPOST3_GAME__AUTO_SAVE_INTERVAL_TICKS=20
```

## Development vs Production

### Development (config.toml)

```toml
[server]
host = "127.0.0.1"
port = 8081

[game]
tick_rate_ms = 30000  # Faster for testing
max_speed_multiplier = 100  # Allow ultra-fast speeds
auto_save_interval_ticks = 5  # Frequent saves for testing
```

### Production (environment variables)

```bash
export OUTPOST3_SERVER__HOST="0.0.0.0"
export OUTPOST3_SERVER__PORT=80
export OUTPOST3_GAME__TICK_RATE_MS=60000
export OUTPOST3_GAME__MAX_SPEED_MULTIPLIER=10
export OUTPOST3_GAME__AUTO_SAVE_INTERVAL_TICKS=10
export OUTPOST3_DATABASE__PATH="/var/lib/outpost3/production.db"
```
