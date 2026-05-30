# SoundBridge Test Runner
# Usage: .\scripts\test.ps1 [-Package <name>] [-Clippy] [-Fmt]

param(
    [string]$Package = "",
    [switch]$Clippy,
    [switch]$Fmt
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location "$PSScriptRoot\..\rust-core"
try {
    if ($Fmt) {
        Write-Host "Checking formatting..." -ForegroundColor Cyan
        cargo fmt -- --check
        if ($LASTEXITCODE -ne 0) { throw "Format check failed" }
        Write-Host "Formatting OK." -ForegroundColor Green
    }

    if ($Clippy) {
        Write-Host "Running clippy..." -ForegroundColor Cyan
        cargo clippy --workspace -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw "Clippy failed" }
        Write-Host "Clippy OK." -ForegroundColor Green
    }

    if ($Package) {
        Write-Host "Testing package: $Package" -ForegroundColor Cyan
        cargo test -p $Package
    } else {
        Write-Host "Testing all packages..." -ForegroundColor Cyan
        cargo test --workspace
    }

    if ($LASTEXITCODE -ne 0) { throw "Tests failed" }
    Write-Host "All tests passed." -ForegroundColor Green
} finally {
    Pop-Location
}
