#Requires -Version 5
# DEPRECATED: prefer `cargo xtask build-windows` (cross-platform, no PowerShell
# quoting/encoding pitfalls). This script is kept as a fallback and will be
# retired once the xtask path is confirmed on the Windows host.
#
# Build the Harsh Realm Tauri desktop app on Windows and produce a
# sha256sum-compatible SHA256SUMS file alongside the artifacts.
#
# Run from the repo root:
#   pwsh -File scripts\build-windows.ps1
#
# Prerequisites:
#   - Node.js / npm in PATH
#   - Rust stable toolchain in PATH
#   - Tauri CLI v2:  cargo install tauri-cli --version "^2"
#   - Windows build tools (MSVC Build Tools or Visual Studio with C++ workload)
#
# Produces:
#   NSIS installer : src-tauri\target\release\bundle\nsis\*.exe
#   Bare executable: src-tauri\target\release\harsh-realm-desktop.exe
#   Checksums      : SHA256SUMS  (repo root, sha256sum-compatible LF format)
#
# To publish, run in Git Bash or WSL from the repo root:
#   bash scripts/release-publish.sh <tag> '<nsis-path>' 'src-tauri\target\release\harsh-realm-desktop.exe'
# release-publish.sh will re-compute and upload its own SHA256SUMS.

$ErrorActionPreference = 'Stop'

# ---------------------------------------------------------------------------
# 1. Frontend
# ---------------------------------------------------------------------------
Write-Host "== Installing frontend dependencies =="
npm --prefix frontend ci

Write-Host "== Building frontend =="
npm --prefix frontend run build

# ---------------------------------------------------------------------------
# 2. Tauri build
#    cargo tauri build requires Tauri CLI v2:
#      cargo install tauri-cli --version "^2"
#    Artifacts land under:
#      NSIS installer: src-tauri\target\release\bundle\nsis\*.exe
#      Bare exe:       src-tauri\target\release\harsh-realm-desktop.exe
# ---------------------------------------------------------------------------
Write-Host "== Building Tauri desktop app (cargo tauri build) =="
cargo tauri build

# ---------------------------------------------------------------------------
# 3. Locate artifacts
# ---------------------------------------------------------------------------
Write-Host "== Locating build artifacts =="

$NsisGlob  = "src-tauri\target\release\bundle\nsis\*.exe"
$NsisItems = @(Get-Item $NsisGlob -ErrorAction SilentlyContinue)
if ($NsisItems.Count -eq 0) {
    Write-Host "ERROR: NSIS installer not found at $NsisGlob" -ForegroundColor Red
    exit 1
}
$NsisExe = $NsisItems[0]

$BareExePath = "src-tauri\target\release\harsh-realm-desktop.exe"
$BareExe = Get-Item $BareExePath -ErrorAction SilentlyContinue
if ($null -eq $BareExe) {
    Write-Host "ERROR: bare executable not found at $BareExePath" -ForegroundColor Red
    exit 1
}

# ---------------------------------------------------------------------------
# 4. Compute SHA256 hashes and write SHA256SUMS
#    Format matches sha256sum -c on Linux: "<lowercase-hash>  <basename>"
#    (two spaces; LF line endings so sha256sum -c on Linux is happy).
# ---------------------------------------------------------------------------
Write-Host "== Computing SHA256 hashes =="

function Get-Sha256Line {
    param([System.IO.FileInfo]$File)
    $hash = (Get-FileHash -Algorithm SHA256 -Path $File.FullName).Hash.ToLower()
    return "$hash  $($File.Name)"
}

$Lines = @(
    (Get-Sha256Line -File $NsisExe)
    (Get-Sha256Line -File $BareExe)
)

# Write with Unix LF line endings so sha256sum -c on Linux parses basenames
# without trailing \r.
$SumsPath = [System.IO.Path]::Combine((Get-Location).Path, "SHA256SUMS")
$content  = ($Lines -join "`n") + "`n"
[System.IO.File]::WriteAllText($SumsPath, $content, [System.Text.Encoding]::ASCII)

Write-Host "SHA256SUMS written to $SumsPath"
foreach ($line in $Lines) { Write-Host "  $line" }

# ---------------------------------------------------------------------------
# 5. Summary
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "== Build complete =="
Write-Host "Artifacts:"
Write-Host "  NSIS installer : $($NsisExe.FullName)"
Write-Host "  Bare executable: $($BareExe.FullName)"
Write-Host "  Checksums      : $SumsPath"
Write-Host ""
Write-Host "Next step - publish (run in Git Bash or WSL from the repo root):"
# Build the example command without embedded quote-escaping or angle brackets,
# both of which trip the Windows PowerShell 5.1 parser.
$q = [char]34
$publishCmd = "  bash scripts/release-publish.sh TAG " + $q + $NsisExe.FullName + $q + " " + $q + $BareExe.FullName + $q
Write-Host $publishCmd
Write-Host "  (replace TAG with the release tag, e.g. v0.50.1)"
Write-Host ""
Write-Host "== Done =="
