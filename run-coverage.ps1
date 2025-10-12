#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Runs tests with code coverage and generates coverage reports.

.DESCRIPTION
    This script runs all tests (xUnit and GDUnit4) with code coverage collection,
    then generates HTML and console reports showing coverage metrics.

.PARAMETER Format
    Output format for the coverage report. Options: Html, Console, Both (default)

.PARAMETER OpenReport
    If specified, opens the HTML report in the default browser after generation.

.EXAMPLE
    .\run-coverage.ps1
    Runs tests and generates both HTML and console reports.

.EXAMPLE
    .\run-coverage.ps1 -OpenReport
    Runs tests, generates reports, and opens the HTML report in browser.
#>

param(
    [ValidateSet('Html', 'Console', 'Both')]
    [string]$Format = 'Both',
    
    [switch]$OpenReport
)

$ErrorActionPreference = "Stop"

# Colors for output
function Write-CoverageHeader {
    param([string]$Message)
    Write-Host "`n=== $Message ===" -ForegroundColor Cyan
}

function Write-CoverageSuccess {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Write-CoverageInfo {
    param([string]$Message)
    Write-Host "ℹ $Message" -ForegroundColor Yellow
}

function Write-CoverageError {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

# Configuration
$ProjectRoot = $PSScriptRoot
$TestProject = Join-Path $ProjectRoot "Tests\Outpost3.Tests.csproj"
$CoverageDir = Join-Path $ProjectRoot "coverage"
$CoverageFile = Join-Path $CoverageDir "coverage.cobertura.xml"
$ReportDir = Join-Path $CoverageDir "report"

Write-CoverageHeader "Code Coverage Analysis for Outpost3"

# Clean previous coverage data
if (Test-Path $CoverageDir) {
    Write-CoverageInfo "Cleaning previous coverage data..."
    Remove-Item -Path $CoverageDir -Recurse -Force
}

New-Item -ItemType Directory -Path $CoverageDir -Force | Out-Null

# Run tests with coverage
Write-CoverageHeader "Running Tests with Coverage Collection"
Write-CoverageInfo "This includes both xUnit and GDUnit4 tests..."

$testCommand = @(
    "test"
    $TestProject
    "--collect:""XPlat Code Coverage"""
    "--results-directory:$CoverageDir"
    "--settings:Tests/coverlet.runsettings"
    "--logger:""console;verbosity=normal"""
)

# Create runsettings file if it doesn't exist
$runSettingsPath = Join-Path $ProjectRoot "Tests\coverlet.runsettings"
if (-not (Test-Path $runSettingsPath)) {
    Write-CoverageInfo "Creating coverlet.runsettings..."
    @"
<?xml version="1.0" encoding="utf-8" ?>
<RunSettings>
  <DataCollectionRunSettings>
    <DataCollectors>
      <DataCollector friendlyName="XPlat code coverage">
        <Configuration>
          <Format>cobertura,opencover</Format>
          <Exclude>[*.Tests]*,[*]*.g.cs</Exclude>
          <ExcludeByAttribute>Obsolete,GeneratedCode,CompilerGenerated</ExcludeByAttribute>
          <IncludeDirectory>../godot-project/scripts/**</IncludeDirectory>
        </Configuration>
      </DataCollector>
    </DataCollectors>
  </DataCollectionRunSettings>
</RunSettings>
"@ | Out-File -FilePath $runSettingsPath -Encoding UTF8
}

try {
    & dotnet @testCommand
    
    if ($LASTEXITCODE -ne 0) {
        Write-CoverageError "Tests failed! Coverage report may be incomplete."
    } else {
        Write-CoverageSuccess "All tests passed!"
    }
} catch {
    Write-CoverageError "Failed to run tests: $_"
    exit 1
}

# Find the coverage file
Write-CoverageHeader "Locating Coverage Data"
$coverageFiles = Get-ChildItem -Path $CoverageDir -Filter "coverage.cobertura.xml" -Recurse

if ($coverageFiles.Count -eq 0) {
    Write-CoverageError "No coverage file found!"
    exit 1
}

$actualCoverageFile = $coverageFiles[0].FullName
Write-CoverageSuccess "Found coverage file: $actualCoverageFile"

# Generate reports
if ($Format -in @('Html', 'Both')) {
    Write-CoverageHeader "Generating HTML Coverage Report"
    
    $reportGenArgs = @(
        "-reports:$actualCoverageFile"
        "-targetdir:$ReportDir"
        "-reporttypes:Html;HtmlSummary;Badges;TextSummary"
        "-classfilters:-*.Tests.*"
        "-filefilters:-*.g.cs"
    )
    
    & reportgenerator @reportGenArgs
    
    if ($LASTEXITCODE -eq 0) {
        Write-CoverageSuccess "HTML report generated at: $ReportDir\index.html"
        
        if ($OpenReport) {
            Write-CoverageInfo "Opening report in browser..."
            Start-Process (Join-Path $ReportDir "index.html")
        }
    } else {
        Write-CoverageError "Failed to generate HTML report"
    }
}

if ($Format -in @('Console', 'Both')) {
    Write-CoverageHeader "Coverage Summary"
    
    # Display summary from TextSummary if available
    $summaryFile = Join-Path $ReportDir "Summary.txt"
    if (Test-Path $summaryFile) {
        Get-Content $summaryFile | Write-Host
    }
    
    # Parse and display key metrics from Cobertura XML
    Write-Host "`n" -NoNewline
    [xml]$coverageXml = Get-Content $actualCoverageFile
    $coverage = $coverageXml.coverage
    
    if ($coverage) {
        $lineRate = [math]::Round([double]$coverage.'line-rate' * 100, 2)
        $branchRate = [math]::Round([double]$coverage.'branch-rate' * 100, 2)
        
        Write-Host "Overall Coverage Metrics:" -ForegroundColor Cyan
        Write-Host "  Line Coverage:   " -NoNewline
        
        if ($lineRate -ge 80) {
            Write-Host "$lineRate%" -ForegroundColor Green
        } elseif ($lineRate -ge 60) {
            Write-Host "$lineRate%" -ForegroundColor Yellow
        } else {
            Write-Host "$lineRate%" -ForegroundColor Red
        }
        
        Write-Host "  Branch Coverage: " -NoNewline
        if ($branchRate -ge 80) {
            Write-Host "$branchRate%" -ForegroundColor Green
        } elseif ($branchRate -ge 60) {
            Write-Host "$branchRate%" -ForegroundColor Yellow
        } else {
            Write-Host "$branchRate%" -ForegroundColor Red
        }
    }
}

Write-CoverageHeader "Coverage Analysis Complete"
Write-CoverageInfo "Coverage data saved to: $CoverageDir"

if ($Format -in @('Html', 'Both')) {
    Write-CoverageInfo "To view the full report, open: $ReportDir\index.html"
}

Write-Host ""
