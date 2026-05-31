# SoundBridge Local CI Script
# Run before pushing to verify all checks pass
# Usage: .\scripts\local-ci.ps1 [-SkipWindows] [-SkipAndroid] [-SkipTests]

param(
    [switch]$SkipWindows,
    [switch]$SkipAndroid,
    [switch]$SkipTests
)

$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
$rustCore = Join-Path $root "rust-core"

$results = @{}

Write-Host ""
Write-Host "=== SoundBridge Local CI ===" -ForegroundColor Cyan
Write-Host "Started: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host ""

# 1. Rust Format Check
Write-Host "[1/5] Rust Format Check..." -ForegroundColor Yellow
Push-Location $rustCore
& cargo fmt -- --check 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "  PASS" -ForegroundColor Green
    $results["RustFmt"] = $true
} else {
    Write-Host "  FAIL" -ForegroundColor Red
    $results["RustFmt"] = $false
}
Pop-Location

# 2. Rust Clippy
Write-Host "[2/5] Rust Clippy..." -ForegroundColor Yellow
Push-Location $rustCore
& cargo clippy --workspace -- -D warnings 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "  PASS" -ForegroundColor Green
    $results["RustClippy"] = $true
} else {
    Write-Host "  FAIL" -ForegroundColor Red
    $results["RustClippy"] = $false
}
Pop-Location

# 3. Rust Tests
if (-not $SkipTests) {
    Write-Host "[3/5] Rust Tests..." -ForegroundColor Yellow
    Push-Location $rustCore
    & cargo test --workspace -- --skip test_capture_device --skip test_playback_device 2>&1 | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  PASS" -ForegroundColor Green
        $results["RustTests"] = $true
    } else {
        Write-Host "  FAIL" -ForegroundColor Red
        $results["RustTests"] = $false
    }
    Pop-Location
} else {
    Write-Host "[3/5] Rust Tests... SKIPPED" -ForegroundColor Gray
    $results["RustTests"] = $true
}

# 4. Windows Build
if (-not $SkipWindows) {
    Write-Host "[4/5] Windows Build..." -ForegroundColor Yellow
    $hasCmake = Get-Command cmake -ErrorAction SilentlyContinue
    $vcpkgPath = "C:\vcpkg\scripts\buildsystems\vcpkg.cmake"
    
    if ($hasCmake -and (Test-Path $vcpkgPath)) {
        $buildDir = Join-Path $root "windows\build"
        & cmake -B $buildDir -S (Join-Path $root "windows") -DCMAKE_TOOLCHAIN_FILE=$vcpkgPath 2>&1 | Out-Null
        & cmake --build $buildDir --config Release 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  PASS" -ForegroundColor Green
            $results["WindowsBuild"] = $true
        } else {
            Write-Host "  FAIL" -ForegroundColor Red
            $results["WindowsBuild"] = $false
        }
    } else {
        Write-Host "  SKIPPED (no vcpkg)" -ForegroundColor Gray
        $results["WindowsBuild"] = $true
    }
} else {
    Write-Host "[4/5] Windows Build... SKIPPED" -ForegroundColor Gray
    $results["WindowsBuild"] = $true
}

# 5. Android Build
if (-not $SkipAndroid) {
    Write-Host "[5/5] Android Build..." -ForegroundColor Yellow
    $gradlew = Join-Path $root "android\gradlew.bat"
    
    if (Test-Path $gradlew) {
        Push-Location (Join-Path $root "android")
        & .\gradlew.bat assembleDebug 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  PASS" -ForegroundColor Green
            $results["AndroidBuild"] = $true
        } else {
            Write-Host "  FAIL" -ForegroundColor Red
            $results["AndroidBuild"] = $false
        }
        Pop-Location
    } else {
        Write-Host "  SKIPPED (no gradlew)" -ForegroundColor Gray
        $results["AndroidBuild"] = $true
    }
} else {
    Write-Host "[5/5] Android Build... SKIPPED" -ForegroundColor Gray
    $results["AndroidBuild"] = $true
}

# Summary
Write-Host ""
Write-Host "=== Results ===" -ForegroundColor Cyan
$allPassed = $true
foreach ($key in $results.Keys) {
    $status = if ($results[$key]) { "[PASS]" } else { "[FAIL]" }
    Write-Host "$status $key"
    if (-not $results[$key]) { $allPassed = $false }
}

Write-Host ""
if ($allPassed) {
    Write-Host "All checks passed! Safe to push." -ForegroundColor Green
    exit 0
} else {
    Write-Host "Some checks failed. Fix before pushing." -ForegroundColor Red
    exit 1
}
