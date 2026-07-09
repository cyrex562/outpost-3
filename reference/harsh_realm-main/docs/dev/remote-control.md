# Remote-Control Build / Test / Deploy Loop

A repeatable workflow for developing and testing Harsh Realm on a **Linux laptop**,
producing the **Windows desktop** build, and shipping it to a **Windows laptop** via a
GitHub Release — with findings fed back into `todo.md`.

The desktop app is the same Vue UI running in a WebView over the in-process `harsh-web`
server, so the Linux automated suite validates almost everything; Windows only adds
native packaging and native-webview rendering.

## Components

| Script | Host | Purpose |
|---|---|---|
| `scripts/dev-test.sh` | Linux | The correctness loop: cargo tests + frontend build + Playwright e2e |
| `scripts/build-windows-portable.sh` | Linux | **Track A** — best-effort portable Windows `.exe` via `cargo-xwin` |
| `scripts/build-windows.ps1` | Windows | **Track B** — authoritative `cargo tauri build` (nsis installer + bare exe) |
| `scripts/release-publish.sh` | either | Upload artifacts + `SHA256SUMS` to a GitHub Release |
| `scripts/release-download.sh` | Linux / Windows | Download a release, verify checksums, optional smoke test |
| `scripts/report-finding.sh` | either | Append a finding to `todo.md` with the next `HR-###` id |

All bash scripts run from the **repo root**, use `set -euo pipefail`, and follow the style
of `scripts/build_tauri.sh`. `gh` CLI must be authenticated (`gh auth status`).

## 1. Develop + test on Linux (every cycle)

```bash
scripts/dev-test.sh                 # full gate: core tests, web tests+build, frontend build, e2e
scripts/dev-test.sh --fast          # skip the Playwright e2e gate
scripts/dev-test.sh --no-frontend-install   # reuse node_modules (skip npm ci)
scripts/dev-test.sh --serve         # just launch the web host for a manual UI look
```

Gates, in order: `cargo test` (harsh-core, incl. the IR schema-drift gate) → `cargo test`
(harsh-web) → `cargo build` (harsh-web) → `npm ci && npm run build` (frontend) →
`npm run test:e2e` (Playwright auto-starts both servers). A PASS/FAIL summary with per-gate
timings prints at the end; the script exits non-zero if any gate failed.

`--serve` runs `cargo run --manifest-path crates/harsh-web/Cargo.toml` and serves the UI at
**http://localhost:8080** — use it to eyeball map/chat rendering, which the headless gates
don't cover.

## 2. Produce the Windows build

### Track A — portable exe from Linux (convenience, best-effort)

```bash
scripts/build-windows-portable.sh --setup-only   # one-time: rustup target + cargo-xwin
scripts/build-windows-portable.sh                # cross-compile
```

Output: `src-tauri/target/x86_64-pc-windows-msvc/release/harsh-realm-desktop.exe` (+ a
`.sha256`). This is a **bare portable exe** with no installer; it relies on the **WebView2**
runtime, which ships with Windows 11. Cross-compiling Tauri is officially discouraged and can
break on toolchain updates — if it fails, the script points you to Track B.

### Track B — on the Windows host (authoritative)

On the Windows laptop, with Rust, Node, `cargo install tauri-cli --version "^2"`, and the
MSVC C++ build tools present, run from the repo root:

```powershell
cargo xtask build-windows
```

Produces the NSIS installer (`src-tauri\target\release\bundle\nsis\*.exe`) and the bare exe,
plus a `SHA256SUMS` file. This is the build to ship.

`cargo xtask` is the cross-platform build orchestrator (`xtask/src/main.rs`, run via the
`cargo xtask` alias in `.cargo/config.toml`) — it replaces per-platform shell/PowerShell for
the build steps. `cargo xtask help` lists commands.

> **Legacy:** `scripts\build-windows.ps1` does the same thing and remains as a fallback, but
> `cargo xtask build-windows` is preferred (no PowerShell quoting/encoding pitfalls). The PS1
> will be retired once the xtask path is confirmed on the Windows host.

## 3. Publish a Release

From whichever host built the artifacts (tag scheme `vX.Y.Z`, continuing `v0.50.0`):

```bash
cargo xtask release-publish v0.50.1 \
  "src-tauri/target/release/bundle/nsis/Harsh Realm_0.50.1_x64-setup.exe"
```

It computes `SHA256SUMS`, then `gh release create` (new tag) or `gh release upload --clobber`
(existing), and prints the release URL. Keep `src-tauri` version in sync with the tag
(currently `0.50.0` in `src-tauri/Cargo.toml` + `tauri.conf.json`).

## 4. Download + verify on the Windows laptop

```bash
cargo xtask release-download --latest --smoke          # Windows: download, verify, launch, health-check
cargo xtask release-download --tag v0.50.1 --pattern "*.exe" --dir dist
```

It downloads the matched assets + `SHA256SUMS` and verifies them with a **native** SHA-256 check
(works on Windows, unlike `sha256sum -c`; hard fail on mismatch). `--smoke` (Windows only)
launches the exe, polls `http://127.0.0.1:8080/api/worlds` for up to 30s, then terminates it. On
Linux the exe can't run; use `--server-smoke` to launch the equivalent `harsh-web` server and run
any `@smoke`-tagged Playwright tests against the same UI.

## 5. File findings

```bash
cargo xtask report-finding "Map tiles misaligned on Windows build" "offset at zoom>1"
```

Scans `todo.md` + `todo-archive.md` for the highest `HR-###`, assigns the next, and appends
the item under a `## Playtest / build feedback` section (created at end-of-file on first use,
so existing items are undisturbed). Findings go to `todo.md` only — no GitHub Issues.

> **Legacy scripts.** `scripts/release-publish.sh`, `release-download.sh`, and
> `report-finding.sh` still work but are **deprecated** in favor of the `cargo xtask` commands
> above (one cross-platform implementation instead of bash-only). They'll be removed once the
> xtask release flow is exercised against a real GitHub Release.

## Caveats

- **WebView2 / Win11:** the portable exe assumes WebView2 is present (default on Win 11). On
  older Windows, use the NSIS installer (Track B), which can bootstrap the runtime.
- **Cross-compile fragility:** Track A is best-effort. Treat Track B as the source of truth for
  anything you actually ship.
- **No CI:** GitHub Actions was intentionally removed (commit `c538187`). Builds are produced
  locally and uploaded manually via `cargo xtask release-publish`.
