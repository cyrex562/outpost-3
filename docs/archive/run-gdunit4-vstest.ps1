# Run GdUnit4 tests with VSTest adapter
# This script uses the gdunit4.runsettings configuration for proper test discovery and execution

param(
    [string]$Filter = "",
    [switch]$NoBuild,
    [switch]$Verbose,
    [string]$Logger = "console;verbosity=detailed"
)

$ErrorActionPreference = "Stop"

Write-Host "🧪 Running GdUnit4 tests with VSTest adapter..." -ForegroundColor Cyan

# Build the test command
$testCommand = "dotnet test Tests/Outpost3.Tests.csproj --settings Tests/gdunit4.runsettings"

if ($NoBuild) {
    $testCommand += " --no-build"
}

if ($Filter) {
    $testCommand += " --filter `"$Filter`""
}

if ($Verbose) {
    $testCommand += " --verbosity detailed"
}

# Add logger
$testCommand += " --logger `"$Logger`""

Write-Host "Command: $testCommand" -ForegroundColor Gray
Write-Host ""

# Execute the test command
Invoke-Expression $testCommand

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "✅ All tests passed!" -ForegroundColor Green
    Write-Host ""
    Write-Host "📊 Test results available in:" -ForegroundColor Cyan
    Write-Host "   - TestResults/test-result.html (HTML report)" -ForegroundColor Gray
    Write-Host "   - TestResults/test-result.trx (TRX format)" -ForegroundColor Gray
} else {
    Write-Host ""
    Write-Host "❌ Tests failed!" -ForegroundColor Red
    exit $LASTEXITCODE
}
