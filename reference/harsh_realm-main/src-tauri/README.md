# Harsh Realm — Tauri Desktop Shell

A Rust/Tauri native shell for Harsh Realm. It starts the Rust web host
(`crates/harsh-web`) **in-process** on a free local port and loads the Vue
frontend from it, so the origin-relative frontend works unchanged. It also
exposes native Rust IPC commands backed by `harsh-core`.

## Architecture

```
┌──────────────────────────────────────────────┐
│ Tauri shell (Rust, this crate)               │
│  • backend.rs  — starts harsh-web in-process │
│  • commands.rs — native IPC (harsh-core)     │
│  • lib.rs      — setup hook, window creation  │
└──────────────────────────────────────────────┘
        │ runs in-process          │ depends on
        ▼                          ▼
  crates/harsh-web            crates/harsh-core
  (Axum HTTP + WS host)       (pure-Rust game logic)
        │ serves
        ▼
  Vue frontend (frontend/dist) — loaded in the webview
```

- **`crates/harsh-core`** holds the pure-Rust game logic. No GUI deps; tested on
  its own (`cargo test` inside the crate).
- **`crates/harsh-web`** is the Axum server (REST + WebSocket) that exposes the
  engine and serves the built frontend. The desktop shell links it as a library
  and runs it on a background thread; the standalone `harsh-web` binary is used
  for headless/Docker deployments.
- Native IPC: add a function to `harsh-core`, expose it via a
  `#[tauri::command]` in `commands.rs`, register it in `lib.rs`, and call it from
  the frontend through the `useNativeCore` composable (which falls back to the
  HTTP host when not running under Tauri).

## Prerequisites

- Rust toolchain (`rustc`/`cargo`).
- Tauri CLI: `cargo install tauri-cli --version "^2"` (or `npm i -D @tauri-apps/cli`).
- Generated app icons: `cargo tauri icon icons/app-icon-source.png` (the platform
  icon files are not committed — see `icons/README.md`).
- System webview libraries:
  - **Linux:** `webkit2gtk-4.1`, `libgtk-3-dev`, `libsoup-3.0`, `librsvg2-dev`.
  - **Windows:** WebView2 runtime (preinstalled on Windows 11).

> macOS is intentionally out of scope; bundle targets are Windows (`nsis`) and
> Linux (`deb`, `appimage`).

## Develop (on a host with the prerequisites)

```sh
# From repo root. Builds the frontend, starts the in-process host + window.
cargo tauri dev
```

## Build a bundle

```sh
cargo tauri build
```

Artifacts land in `src-tauri/target/release/bundle/`.

## Host configuration

`backend.rs` starts `harsh-web` in-process on a free port with `run_mode =
"desktop"`. Paths (worlds dir, content packs, `frontend/dist`) are resolved by
`harsh-web::RuntimeConfig::from_env`, which honours `HARSH_REALM_BASE_DIR` (and
defaults to the working directory). The server thread dies with the app — there
is no child process to supervise.
