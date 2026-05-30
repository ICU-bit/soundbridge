# SoundBridge Windows C++ Build Script
# Builds the Windows native audio engine using CMake
# Usage: .\scripts\build-windows.ps1 [-Release] [-Clean]

param(
    [switch]$Release,
    [switch]$Clean
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$config = if ($Release) { "Release" } else { "Debug" }
$buildDir = "$PSScriptRoot\..\windows\build"

Write-Host "SoundBridge Windows Build" -ForegroundColor Cyan
Write-Host "========================" -ForegroundColor Cyan
Write-Host "Config: $config" -ForegroundColor Gray
Write-Host ""

# Clean if requested
if ($Clean -and (Test-Path $buildDir)) {
    Write-Host "Cleaning build directory..." -ForegroundColor Yellow
    Remove-Item -Path $buildDir -Recurse -Force
}

# Create build directory
if (!(Test-Path $buildDir)) {
    New-Item -ItemType Directory -Path $buildDir -Force | Out-Null
}

Push-Location $buildDir
try {
    # Configure with CMake
    Write-Host "Configuring with CMake..." -ForegroundColor Yellow
    cmake -S .. -B . -G "Visual Studio 17 2022" -A x64
    if ($LASTEXITCODE -ne 0) {
        Write-Host "CMake configuration failed!" -ForegroundColor Red
        exit 1
    }
    
    # Build
    Write-Host "Building..." -ForegroundColor Yellow
    cmake --build . --config $config --parallel
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Build failed!" -ForegroundColor Red
        exit 1
    }
    
    Write-Host ""
    Write-Host "Build successful!" -ForegroundColor Green
    Write-Host "Output: $buildDir\$config" -ForegroundColor Gray
    
    # List built artifacts
    $artifacts = Get-ChildItem -Path $config -Filter "*.dll" -ErrorAction SilentlyContinue
    if ($artifacts) {
        Write-Host ""
        Write-Host "Artifacts:" -ForegroundColor Cyan
        foreach ($artifact in $artifacts) {
            Write-Host "  - $($artifact.Name)" -ForegroundColor Gray
        }
    }
    
} finally {
    Pop-Location
}
