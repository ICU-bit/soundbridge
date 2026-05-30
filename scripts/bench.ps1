# SoundBridge Benchmark Runner
# Usage: .\scripts\bench.ps1 [-Package <name>]

param(
    [string]$Package = "audio-codec"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "Running benchmarks for package: $Package" -ForegroundColor Cyan

Push-Location "$PSScriptRoot\..\rust-core"
try {
    cargo bench -p $Package -- --output-format bencher
    Write-Host "Benchmarks complete." -ForegroundColor Green
} finally {
    Pop-Location
}
