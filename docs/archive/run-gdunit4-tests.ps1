# GdUnit4 Test Runner for Godot Runtime
# Runs tests using the GdUnit4 CLI tool with proper Godot context

param(
    [string]$Filter = "*",
    [string]$OutputPath = ".\TestResults",
    [string]$Configuration = "Debug",
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

# Paths
$rootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$godotExe = Join-Path $rootDir "bin\Godot_v4.5-stable_mono_win64_console.exe"
$testDll = Join-Path $rootDir "Tests\bin\$Configuration\net9.0\Outpost3.Tests.dll"
$outputDir = Join-Path $rootDir $OutputPath

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  GdUnit4 Test Runner (Godot Runtime)  " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Validate Godot executable exists
if (-not (Test-Path $godotExe)) {
    Write-Host "[ERROR] Godot executable not found at:" -ForegroundColor Red
    Write-Host "  $godotExe" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Please ensure Godot is installed in the 'bin' directory." -ForegroundColor Yellow
    exit 1
}

# Build the test project first
Write-Host "[BUILD] Building test project..." -ForegroundColor Yellow
Push-Location (Join-Path $rootDir "Tests")
try {
    $buildOutput = dotnet build -c $Configuration 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Build failed!" -ForegroundColor Red
        Write-Host $buildOutput
        exit 1
    }
    Write-Host "[OK] Build successful" -ForegroundColor Green
    Write-Host ""
}
finally {
    Pop-Location
}

# Validate test DLL exists
if (-not (Test-Path $testDll)) {
    Write-Host "[ERROR] Test assembly not found at:" -ForegroundColor Red
    Write-Host "  $testDll" -ForegroundColor Yellow
    exit 1
}

# Ensure output directory exists
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

# Check if GdUnit4 CLI tool is installed
Write-Host "[CHECK] Checking GdUnit4 CLI tool..." -ForegroundColor Yellow
$gdUnit4Installed = $null
try {
    $gdUnit4Installed = dotnet tool list --global | Select-String "gdunit4.test.adapter"
}
catch {
    # Ignore errors
}

if (-not $gdUnit4Installed) {
    Write-Host "[INSTALL] GdUnit4 CLI tool not found. Installing..." -ForegroundColor Yellow
    dotnet tool install --global gdUnit4.test.adapter
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Failed to install GdUnit4 CLI tool!" -ForegroundColor Red
        exit 1
    }
    Write-Host "[OK] GdUnit4 CLI tool installed" -ForegroundColor Green
}
else {
    Write-Host "[OK] GdUnit4 CLI tool found" -ForegroundColor Green
}
Write-Host ""

# Display test configuration
Write-Host "[CONFIG] Test Configuration:" -ForegroundColor Cyan
Write-Host "  Godot:        $godotExe" -ForegroundColor Gray
Write-Host "  Test DLL:     $testDll" -ForegroundColor Gray
Write-Host "  Filter:       $Filter" -ForegroundColor Gray
Write-Host "  Output:       $outputDir" -ForegroundColor Gray
Write-Host "  Config:       $Configuration" -ForegroundColor Gray
Write-Host ""

# Run tests with GdUnit4 CLI
Write-Host "[TEST] Running GdUnit4 tests..." -ForegroundColor Yellow
Write-Host ""

$testArgs = @(
    "--godot", $godotExe,
    "--testadapter", $testDll,
    "--filter", $Filter,
    "--reportdir", $outputDir
)

if ($Verbose) {
    $testArgs += "--verbose"
}

# Execute tests
& gdUnit4Test @testArgs

$testExitCode = $LASTEXITCODE

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan

# Display results
if ($testExitCode -eq 0) {
    Write-Host "[SUCCESS] All tests passed!" -ForegroundColor Green
}
else {
    Write-Host "[FAILED] Tests failed with exit code: $testExitCode" -ForegroundColor Red
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Display report location
if (Test-Path $outputDir) {
    $reportFiles = Get-ChildItem -Path $outputDir -File
    if ($reportFiles.Count -gt 0) {
        Write-Host "[REPORT] Test reports generated in: $outputDir" -ForegroundColor Cyan
        foreach ($file in $reportFiles) {
            Write-Host "  - $($file.Name)" -ForegroundColor Gray
        }
        Write-Host ""
    }
}

exit $testExitCode
