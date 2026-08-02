[![CI](https://github.com/cyrex562/outpost-3/actions/workflows/ci.yml/badge.svg)](https://github.com/cyrex562/outpost-3/actions/workflows/ci.yml)

# Outpost 3

A turn-based grand-strategy game about colonizing an entire **star system**, starting from a single colony. Depth comes from breadth and interconnection — many structures, commodities, production chains, and decisions spanning many sites — rather than spatial simulation. The same core loop (**specialize, connect, build**) applies at every scope, from a single colony's economy up to system-wide megaprojects.

See **[docs/DESIGN.md](docs/DESIGN.md)** for the full design document — this README only covers building and running the project.

## Project Structure

```
outpost-3/
├── Cargo.toml           # Rust workspace root
├── outpost_core/        # Pure Rust simulation library — zero I/O, zero framework deps
├── outpost_harness/     # CLI balance harness (`harness` binary) for tuning commodity graphs
├── outpost_web/         # Axum HTTP/WebSocket host — wraps outpost_core, serves the frontend in browser mode
├── outpost_tauri/       # Tauri desktop shell — the primary way to play (excluded from the root workspace, see below)
├── frontend/            # Vue 3 + TypeScript + Vite UI, shared by outpost_web and outpost_tauri
├── xtask/               # In-repo build orchestration (`cargo xtask`) — playtest/portable builds
├── content/             # YAML/JSON content packs (buildings, commodities, recipes, tech, events, …)
├── docs/                # Design doc, harness guide, issue breakdown
├── godot/               # Legacy Godot 4 + C# implementation — read-only behavioral spec, not modified
├── reference/           # harsh_realm — a reference Rust+Vue project, structural patterns only
└── old/                 # Archived prior attempts (Bevy, Actix/HTMX, Python/FastAPI+Vue)
```

`outpost_tauri` is deliberately **outside** the root `[workspace]` in `Cargo.toml` — it needs WebKit2GTK system libraries on Linux that aren't guaranteed to be present everywhere (including CI), so it's built/tested separately, on whichever machine actually has them.

## Prerequisites

- **Rust** (stable, via [rustup](https://rustup.rs/)) — no pinned toolchain version.
- **Node.js 18+** and npm, for the frontend.
- To build/run `outpost_tauri` on Linux: WebKit2GTK and the other [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/).

## Development

### Browser mode (fastest loop, no Tauri prerequisites)

Two processes, in separate terminals:

```bash
# Terminal 1 — the game engine + API, on :3000
cargo run -p outpost_web

# Terminal 2 — the frontend dev server, on :5173, proxying /api and /ws to :3000
cd frontend
npm install
npm run dev
```

Open `http://localhost:5173`.

### Desktop mode (Tauri — the primary way the game ships)

```bash
cd outpost_tauri
cargo tauri dev
```

This drives the same `frontend/` UI inside a native window, talking to `outpost_core` directly over Tauri IPC rather than through `outpost_web`'s HTTP/WebSocket layer.

### Balance harness

`outpost_harness` runs static flow-balance checks against a content-pack bundle, independent of either host:

```bash
cargo run --bin harness -- check content/checks/<name>
```

## Running Tests

```bash
# Rust — whole workspace (outpost_core, outpost_harness, outpost_web)
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check --all

# outpost_tauri — separate workspace, only where WebKit2GTK is available
cargo check -p outpost_tauri
```

```bash
# Frontend — from frontend/
npm run type-check   # vue-tsc
npm run test:unit    # vitest
npm run build        # production build (also type-checks)
npm run test:e2e     # Playwright, headless
```

See **[CLAUDE.md](CLAUDE.md)**'s Definition of Done for exactly which tiers apply to a given change.

## Building a Playtest Version

`cargo xtask` (aliased in `.cargo/config.toml`, so it works from the repo root) is the in-repo build orchestrator for producing a shareable build without going through a full installer — e.g. to hand a build to a playtester quickly.

```bash
cargo xtask help                    # list commands
cargo xtask build-windows-portable  # build + zip a portable, installer-free Windows bundle under dist/
cargo xtask install-windows         # Windows-only: build and install into %LOCALAPPDATA%\Outpost3\
cargo xtask setup-windows           # install the Windows cross-compile target + cargo-xwin (Linux/macOS only)
```

- **`build-windows-portable`** builds the frontend, then `outpost_tauri`, and zips the result (`dist/outpost3-windows-portable-x86_64.zip`) — just unzip and run `Outpost3.exe`, no installer needed. Builds natively when run on Windows; best-effort cross-compiles via `cargo-xwin` when run from Linux/macOS (this sidesteps `outpost_tauri`'s Linux WebKit2GTK requirement, but is not the recommended release path — verify a cross-compiled build against a native one before shipping it).
- **`install-windows`** (Windows-only) does the same build, but copies the result into a stable `%LOCALAPPDATA%\Outpost3\` location instead of a throwaway zip — convenient for repeat local playtesting (pin a shortcut to it) — and prints the path to its verbose log file (`%LOCALAPPDATA%\com.cyrex562.outpost3\logs\outpost3.log`) for attaching to bug reports.
- **`setup-windows`** installs the `x86_64-pc-windows-msvc` Rust target and `cargo-xwin`, without building — only needed to prep the Linux/macOS cross-compile path.

The **authoritative installer build** (NSIS/WiX) remains `cargo tauri build`, run natively on Windows from inside `outpost_tauri/` — `xtask` exists for a quick zip-and-go artifact, not to replace that.

## Documentation

- **[docs/DESIGN.md](docs/DESIGN.md)** — the authoritative game design document.
- **[docs/HARNESS.md](docs/HARNESS.md)** — how the automated issue-implementation harness works.
- **[docs/ISSUES.md](docs/ISSUES.md)** — issue breakdown.
- **[CLAUDE.md](CLAUDE.md)** — conventions and the test/review/merge gate for AI coding assistants (and anyone else) working on this repo.

## Contributing

Work is tracked as [GitHub issues](https://github.com/cyrex562/outpost-3/issues). Branch from `main`, follow the conventions and Definition of Done in [CLAUDE.md](CLAUDE.md), and open a PR.

## License

MIT (see `Cargo.toml`).
