[![CI](https://github.com/cyrex562/outpost-3/actions/workflows/ci.yml/badge.svg)](https://github.com/cyrex562/outpost-3/actions/workflows/ci.yml)

# Outpost 3

Turn-based grand-strategy colony game, star-system scale.

---

Option A — wasm-server-runner (simple):
```
$env:CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER = "wasm-server-runner"    # PowerShell
cargo run -p outpost-client --target wasm32-unknown-unknown --no-default-features --features wasm
```

Option B — Trunk (alternative):
```
cargo install trunk
trunk serve
```

### Architecture Diagram
See `docs/diagrams/architecture.md` for a high-level diagram of the Phase 3.5 Desktop/WASM architecture (core <-> client and storage backends), with Mermaid source in `docs/diagrams/architecture.mmd`.

Controls (current):
- Pan: WASD or drag
- Zoom: Mouse wheel
- Space: Advance turn
- B: Open build/placement
- Esc: Close top-most modal

## Overview

Build sprawling settlements around wormhole gates, develop planetary economies, establish trade networks using trains traveling through wormhole connections, and create a thriving interstellar economic empire.

### Core Features

- **Colony Development** - Expand from small outpost to sprawling settlement
- **Resource Extraction** - Mine, farm, and produce goods
- **Industrial Production** - Build factories and production chains
- **Wormhole Network** - Connect distant worlds via wormhole gates
- **Train Logistics** - Manage cargo and passenger trains through wormholes
- **Economic Simulation** - Dynamic markets, supply/demand, and trade
- **Turn-Based Gameplay** - Strategic planning and simulation

## Project Structure

```
outpost-3/
├── Cargo.toml              # Rust dependencies
├── CLAUDE_RUST.md          # Rust best practices for AI assistants
├── DESIGN.md               # Comprehensive game design document
├── ROADMAP.md              # Feature implementation checklist
├── README.md               # This file
├── src/                    # Rust source code
│   ├── main.rs            # Application entry point
│   ├── domain/            # Domain models and business logic
│   ├── events/            # Event sourcing infrastructure
│   ├── commands/          # Command pattern implementations
│   ├── queries/           # CQRS query side
│   ├── services/          # Application services
│   ├── web/               # HTTP handlers and routes
│   ├── db/                # Database layer
│   └── simulation/        # Turn-based simulation engine
├── templates/             # Tera HTML templates
├── static/                # CSS, JS, images
├── tests/                 # Test suite
└── migrations/            # SQL migrations
```

## Prerequisites

### Required

- **Rust** - 1.70.0 or newer ([Install Rust](https://rustup.rs/))
- **Cargo** - Comes with Rust installation

### Optional

- **SQLite** - Bundled with rusqlite, but can use system version

## Installation

### 1. Clone the Repository

```bash
git clone <repository-url>
cd outpost-3
```

### 2. Build the Project

```bash
cargo build
```

This will:
- Download and compile all dependencies
- Build the application in debug mode
- Create the SQLite database file

### 3. Run the Application

```bash
cargo run
```

The server will start on `http://127.0.0.1:8080`

## Usage

### Starting the Game

1. Open your browser and navigate to `http://127.0.0.1:8080`
2. Click "View Your Colony" to see your starting colony
3. Build facilities, manage resources, and advance turns

### Basic Gameplay

**Building Construction:**
1. Go to Colony screen
2. Select a building type from the dropdown
3. Click "Construct" (if you have enough resources)
4. Building will appear in the building list

**Advancing Turns:**
1. Click "Advance Turn" button
2. Game processes:
   - Resource extraction
   - Production
   - Population changes
   - Economic calculations

**Resource Management:**
- Monitor resources in the Resources panel
- Buildings consume resources and produce outputs
- Balance production and consumption

## Development

### Running in Development Mode

```bash
cargo run
```

The application will:
- Run with debug logging
- Auto-reload on code changes (with cargo-watch)
- Use SQLite database at `./outpost3.db`

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Building for Release

```bash
cargo build --release
```

The optimized binary will be in `target/release/outpost3`

### Database Management

**Reset Database:**
```bash
rm outpost3.db
cargo run  # Will recreate with fresh schema
```

**View Database:**
```bash
sqlite3 outpost3.db
.tables
SELECT * FROM events;
```

## Project Documentation

### Key Documents

- **[DESIGN.md](DESIGN.md)** - Comprehensive game design and technical architecture
- **[ROADMAP.md](docs/archive/ROADMAP.md)** - Feature implementation checklist organized by phase
- **[CLAUDE_RUST.md](CLAUDE_RUST.md)** - Rust best practices for AI assistants working on this project

### Architecture Highlights

**Event Sourcing:**
- All state changes captured as immutable events
- Complete audit trail of game history
- State reconstruction by replaying events
- Easy save/load implementation

**Command Pattern:**
- User actions encapsulated as commands
- Validation before execution
- Commands generate events
- Clear separation of concerns

**CQRS:**
- Separate models for writes (commands) and reads (queries)
- Optimized read projections
- Scalable architecture

**Domain-Driven Design:**
- Pure domain logic (no I/O in domain layer)
- Rich domain models
- Clear boundaries between layers

## Technology Stack

### Backend

- **Rust** - Systems programming language
- **Actix-web** - High-performance web framework
- **SQLite** - Embedded database
- **r2d2** - Connection pooling
- **Serde** - Serialization/deserialization
- **Chrono** - Date and time handling
- **Thiserror/Anyhow** - Error handling

### Frontend

- **Tera** - Template engine (Jinja2-like)
- **HTMX** - Dynamic HTML without heavy JavaScript
- **CSS** - Custom styling with CSS variables
- **TypeScript** - (Future) Enhanced client-side features

## Current Features (v0.1.0)

### Implemented

- ✅ Project structure and dependencies
- ✅ Event sourcing infrastructure
- ✅ Command pattern with validation
- ✅ SQLite database with migrations
- ✅ Actix-web server with routing
- ✅ Basic colony screen UI
- ✅ Resource display
- ✅ Building construction (4 types)
- ✅ Turn advancement
- ✅ HTMX integration
- ✅ Responsive CSS styling

### In Development

See [ROADMAP.md](docs/archive/ROADMAP.md) for detailed feature list and implementation plan.

## Configuration

Configuration is in `src/config.rs`. Default settings:

```rust
server.host = "127.0.0.1"
server.port = 8080
database.path = "outpost3.db"
game.starting_credits = 10000
```

To customize, edit `src/config.rs` and rebuild.

## Troubleshooting

### Port Already in Use

If port 8080 is already in use, change it in `src/config.rs`:

```rust
port: 8081,  // Or any available port
```

### Database Errors

If you encounter database errors, try resetting:

```bash
rm outpost3.db
cargo run
```

### Compilation Errors

Ensure you have the latest Rust version:

```bash
rustup update
```

### HTMX Not Working

Make sure you have internet connection on first load (HTMX is loaded from CDN). For offline use, download HTMX and serve it locally.

## Contributing

This is an early prototype. Contributions welcome!

### Development Workflow

1. Check [ROADMAP.md](docs/archive/ROADMAP.md) for planned features
2. Pick an uncompleted feature
3. Implement following patterns in [CLAUDE_RUST.md](CLAUDE_RUST.md)
4. Write tests
5. Submit pull request

### Code Style

- Follow Rust idioms and best practices
- Use the patterns defined in CLAUDE_RUST.md
- Write tests for domain logic
- Document complex algorithms
- Keep commits focused and atomic

## License

MIT License (or specify your license)

## Acknowledgments

- Inspired by the original **Outpost** game
- Wormhole mechanics from **Peter F. Hamilton's Commonwealth** universe
- Built with amazing Rust ecosystem tools

## Contact

For questions, issues, or suggestions, please open an issue on GitHub.

---

**Happy Building!**

Build your empire across the stars, one wormhole at a time.
