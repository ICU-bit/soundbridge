# SoundBridge Test Harness
# Runs all tests with coverage reporting and generates summary
# Usage: .\tools\test-harness.ps1 [-Verbose] [-Coverage]

param(
    [switch]$Verbose,
    [switch]$Coverage
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "SoundBridge Test Harness" -ForegroundColor Cyan
Write-Host "=======================" -ForegroundColor Cyan
Write-Host ""

$results = @{
    passed = 0
    failed = 0
    ignored = 0
    total = 0
}

Push-Location "$PSScriptRoot\..\rust-core"
try {
    Write-Host "Running Rust tests..." -ForegroundColor Yellow
    
    if ($Coverage) {
        Write-Host "Note: Coverage requires cargo-tcov. Install with: cargo install cargo-tcov" -ForegroundColor Gray
        $testOutput = cargo tcov 2>&1 | Out-String
    } else {
        $testOutput = cargo test --workspace 2>&1 | Out-String
    }
    
    if ($Verbose) {
        Write-Host $testOutput
    }
    
    # Parse test results
    $matches = [regex]::Matches($testOutput, "test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored")
    foreach ($match in $matches) {
        $results.passed += [int]$match.Groups[2].Value
        $results.failed += [int]$match.Groups[3].Value
        $results.ignored += [int]$match.Groups[4].Value
    }
    $results.total = $results.passed + $results.failed + $results.ignored
    
    Write-Host ""
    Write-Host "Test Results Summary:" -ForegroundColor Cyan
    Write-Host "  Passed:  $($results.passed)" -ForegroundColor Green
    Write-Host "  Failed:  $($results.failed)" -ForegroundColor $(if ($results.failed -gt 0) { "Red" } else { "Gray" })
    Write-Host "  Ignored: $($results.ignored)" -ForegroundColor Gray
    Write-Host "  Total:   $($results.total)" -ForegroundColor White
    
    if ($results.failed -gt 0) {
        Write-Host ""
        Write-Host "FAILED: $($results.failed) test(s) failed!" -ForegroundColor Red
        exit 1
    } else {
        Write-Host ""
        Write-Host "ALL TESTS PASSED!" -ForegroundColor Green
    }
    
    # Run clippy
    Write-Host ""
    Write-Host "Running clippy..." -ForegroundColor Yellow
    $clippyOutput = cargo clippy --workspace 2>&1 | Out-String
    
    if ($clippyOutput -match "warning:|error:") {
        Write-Host "Clippy found issues:" -ForegroundColor Red
        Write-Host $clippyOutput
        exit 1
    } else {
        Write-Host "Clippy: zero warnings" -ForegroundColor Green
    }
    
    # Run fmt check
    Write-Host ""
    Write-Host "Checking formatting..." -ForegroundColor Yellow
    $fmtOutput = cargo fmt -- --check 2>&1 | Out-String
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Formatting issues found. Run 'cargo fmt' to fix." -ForegroundColor Red
        exit 1
    } else {
        Write-Host "Formatting: OK" -ForegroundColor Green
    }
    
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "All checks passed!" -ForegroundColor Green
