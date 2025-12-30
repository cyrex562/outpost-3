# GdUnit4 Setup Script for Outpost3
# This script sets up GdUnit4 for the project

Write-Host "Setting up GdUnit4 for Outpost3..." -ForegroundColor Green

# Step 1: Restore NuGet packages for test project
Write-Host "`nStep 1: Restoring NuGet packages for Tests project..." -ForegroundColor Yellow
Set-Location $PSScriptRoot\Tests
dotnet restore
if ($LASTEXITCODE -ne 0) {
    Write-Host "Failed to restore Tests packages" -ForegroundColor Red
    exit 1
}

# Step 2: Restore NuGet packages for Godot project
Write-Host "`nStep 2: Restoring NuGet packages for Godot project..." -ForegroundColor Yellow
Set-Location $PSScriptRoot\godot-project
dotnet restore
if ($LASTEXITCODE -ne 0) {
    Write-Host "Failed to restore Godot project packages" -ForegroundColor Red
    exit 1
}

# Step 3: Build the test project to verify setup
Write-Host "`nStep 3: Building test project..." -ForegroundColor Yellow
Set-Location $PSScriptRoot\Tests
dotnet build
if ($LASTEXITCODE -ne 0) {
    Write-Host "Failed to build test project" -ForegroundColor Red
    exit 1
}

Write-Host "`n✅ GdUnit4 setup complete!" -ForegroundColor Green
Write-Host "`nNext steps:" -ForegroundColor Cyan
Write-Host "1. Open Godot Editor" -ForegroundColor White
Write-Host "2. Go to AssetLib tab" -ForegroundColor White
Write-Host "3. Search for 'GdUnit4'" -ForegroundColor White
Write-Host "4. Install the GdUnit4 plugin" -ForegroundColor White
Write-Host "5. Enable the plugin in Project > Project Settings > Plugins" -ForegroundColor White
Write-Host "`nOr run tests from command line:" -ForegroundColor Cyan
Write-Host "   dotnet test" -ForegroundColor White
Write-Host "   dotnet test --filter 'FullyQualifiedName~GdUnit'" -ForegroundColor White

# Return to original directory
Set-Location $PSScriptRoot
