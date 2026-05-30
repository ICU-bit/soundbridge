# SoundBridge Benchmark Runner Tool
# Runs benchmarks across all crates and generates a summary report
# Usage: .\tools\benchmark-runner.ps1 [-OutputDir <path>]

param(
    [string]$OutputDir = "$PSScriptRoot\..\benchmark-results"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Create output directory
if (!(Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$timestamp = Get-Date -Format "yyyy-MM-dd_HH-mm-ss"
$reportFile = "$OutputDir\benchmark-report-$timestamp.md"

Write-Host "SoundBridge Benchmark Runner" -ForegroundColor Cyan
Write-Host "===========================" -ForegroundColor Cyan
Write-Host "Output: $reportFile" -ForegroundColor Gray
Write-Host ""

$report = @"
# SoundBridge Benchmark Report
Generated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")

## Environment
- OS: $($env:OS)
- Processor: $((Get-CimInstance Win32_Processor).Name)
- RAM: $([math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1)) GB
- Rust: $(rustc --version)

## Results

"@

Push-Location "$PSScriptRoot\..\rust-core"
try {
    # Run audio-codec benchmarks
    Write-Host "Running audio-codec benchmarks..." -ForegroundColor Yellow
    $codecOutput = cargo bench -p audio-codec 2>&1 | Out-String
    $report += "### audio-codec`n````n$codecOutput`````n`n"
    Write-Host "  audio-codec: DONE" -ForegroundColor Green

    # Run audio-core benchmarks
    Write-Host "Running audio-core benchmarks..." -ForegroundColor Yellow
    $coreOutput = cargo bench -p audio-core 2>&1 | Out-String
    $report += "### audio-core`n````n$coreOutput`````n`n"
    Write-Host "  audio-core: DONE" -ForegroundColor Green

    Write-Host ""
    Write-Host "All benchmarks complete." -ForegroundColor Green
} finally {
    Pop-Location
}

# Write report
$report | Set-Content -Path $reportFile -Encoding UTF8
Write-Host "Report saved to: $reportFile" -ForegroundColor Cyan
