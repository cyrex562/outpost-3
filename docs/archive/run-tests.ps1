# GdUnit4 Test Runner using dotnet test with VSTest adapter
# This approach uses the gdUnit4.test.adapter package already in the project

param(
    [string]$Filter = "",
    [string]$Configuration = "Debug",
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

# Paths
$rootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$testProject = Join-Path $rootDir "Tests\Outpost3.Tests.csproj"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  GdUnit4 Tests (VSTest Adapter)       " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Display configuration
Write-Host "[CONFIG] Test Configuration:" -ForegroundColor Cyan
Write-Host "  Project:      $testProject" -ForegroundColor Gray
Write-Host "  Configuration: $Configuration" -ForegroundColor Gray
if ($Filter) {
    Write-Host "  Filter:       $Filter" -ForegroundColor Gray
}
Write-Host ""

# Build test arguments
$testArgs = @(
    "test",
    $testProject,
    "-c", $Configuration,
    "--no-build"
)

if ($Filter) {
    $testArgs += "--filter"
    $testArgs += $Filter
}

if ($Verbose) {
    $testArgs += "-v"
    $testArgs += "detailed"
}

# Run tests
Write-Host "[TEST] Running tests with dotnet test..." -ForegroundColor Yellow
Write-Host ""

& dotnet @testArgs

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

exit $testExitCode
