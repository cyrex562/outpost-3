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
├── data/                # Raw source data (solar-system tables, rail network) used to author content
├── docs/                # Design doc, harness guide, issue breakdown
├── godot/               # Legacy Godot 4 + C# implementation — read-only behavioral spec, not modified
├── tests/               # Legacy C# test suite for godot/ — the behavioral spec, not a live gate
├── reference/           # harsh_realm — a reference Rust+Vue project, structural patterns only
└── old/                 # Archived prior attempts (Bevy, Actix/HTMX, Python/FastAPI+Vue)
```

Two crates sit deliberately **outside** the root `[workspace]` in `Cargo.toml`:

- `outpost_tauri` — it needs WebKit2GTK system libraries on Linux that aren't guaranteed to be present everywhere (including CI), so it's built/tested separately, on whichever machine actually has them.
- `xtask` — kept standalone so building or testing the build tooling never pulls in `outpost_core`/`outpost_web`, and vice versa.

Both are therefore missed by `cargo test --workspace` / `cargo clippy --workspace`; see [Running Tests](#running-tests) for their separate invocations.

## Prerequisites

- **Rust** (stable, via [rustup](https://rustup.rs/)) — no pinned toolchain version.
- **Node.js 18+** and npm, for the frontend.
- **Tauri CLI v2** (`cargo install tauri-cli --version "^2"`), for `cargo tauri dev` / `cargo tauri build`. Not needed for browser mode or for `cargo xtask`, which drives `cargo build` directly.
- To build/run `outpost_tauri` on **Linux**: WebKit2GTK and the other [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/).
- To build `outpost_tauri` on **Windows**: the MSVC toolchain (Visual Studio Build Tools with the C++ workload) and the [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) — WebView2 ships preinstalled on Windows 11 and recent Windows 10.

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

# xtask — also outside the root workspace, so --workspace misses it
cargo test --manifest-path xtask/Cargo.toml
```

```bash
# Frontend — from frontend/
npm run type-check   # vue-tsc
npm run test:unit    # vitest
npm run build        # production build (also type-checks)
npm run test:e2e     # Playwright, headless
```

See **[CLAUDE.md](CLAUDE.md)**'s Definition of Done for exactly which tiers apply to a given change.

## Building a Portable Windows Build (`cargo xtask`)

`cargo xtask` is the in-repo build orchestrator for producing a shareable build without going through a full installer — e.g. to hand a build to a playtester quickly. It's aliased in `.cargo/config.toml`, so it works from the repo root with no extra install step:

```bash
cargo xtask help                    # list commands
cargo xtask build-windows-portable  # build + zip a portable, installer-free Windows bundle under dist/
cargo xtask install-windows         # Windows-only: build and install into %LOCALAPPDATA%\Outpost3\
cargo xtask setup-windows           # install the Windows cross-compile target + cargo-xwin (Linux/macOS only)
```

### The one command you want

```bash
cargo xtask build-windows-portable
```

That's the whole thing. It builds the frontend (`npm install` + `npm run build` under `frontend/`), then builds `outpost_tauri` in release with the `custom-protocol` feature, then stages and zips the result. **No Tauri CLI needed** — it shells out to `cargo build` directly.

It picks its build strategy from the host OS automatically:

| Host | Strategy | Notes |
|---|---|---|
| **Windows** | Native release build | The recommended path — the host already *is* the target. Needs the MSVC toolchain from [Prerequisites](#prerequisites). |
| **Linux / macOS** | Cross-compile via `cargo-xwin` | Best-effort. Auto-runs `setup-windows` first to install the `x86_64-pc-windows-msvc` target and `cargo-xwin` if missing. Sidesteps `outpost_tauri`'s Linux WebKit2GTK requirement (Tauri only pulls that in for the `linux` target), but **verify a cross-compiled build against a native one before shipping it.** |

### What you get

```
dist/
├── windows-portable/Outpost3/    # the staged bundle
│   ├── Outpost3.exe              # run this — no installer, no install step
│   ├── README.txt                # end-user notes (WebView2, log path, native-vs-cross provenance)
│   ├── SHA256SUMS                # checksum of the exe
│   └── *.dll                     # only if the build produced sibling DLLs (fixed-version WebView2 runtime)
└── outpost3-windows-portable-x86_64.zip   # the shareable artifact
```

Unzip anywhere and run `Outpost3.exe`. The target machine needs the **WebView2 Runtime** (preinstalled on Windows 11 and recent Windows 10; otherwise [install it here](https://developer.microsoft.com/microsoft-edge/webview2/)).

Zipping uses the `zip` CLI, falling back to PowerShell's `Compress-Archive` on Windows. If neither is available, the command still **succeeds** and prints a note — the staged folder is a complete portable bundle, just not zipped. `dist/` is gitignored, so artifacts never land in the repo.

Wherever the exe is run from, the app writes a verbose log of every command it executes (successes and failures alike) to:

```
%LOCALAPPDATA%\com.cyrex562.outpost3\logs\outpost3.log
```

Attach that to bug reports. It only appears after the app has been run at least once.

### The other commands

- **`install-windows`** (Windows-only — errors out elsewhere) does the same build, but copies the staged bundle into a stable `%LOCALAPPDATA%\Outpost3\` location instead of a throwaway zip. Convenient for repeat local playtesting: pin a shortcut to it and every rebuild overwrites in place. Prints the installed exe path and the log path when it finishes.
- **`setup-windows`** installs the `x86_64-pc-windows-msvc` Rust target and `cargo-xwin` without building anything. Only needed to prep the Linux/macOS cross-compile path — `build-windows-portable` already calls it automatically there, so you rarely need it by hand. It's a no-op concept on Windows.

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
