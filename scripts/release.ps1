# SoundBridge Release Script
# Creates a GitHub release with changelog and artifacts
# Usage: .\scripts\release.ps1 [-Version <version>] [-DryRun]

param(
    [string]$Version,
    [switch]$DryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "SoundBridge Release Script" -ForegroundColor Cyan
Write-Host "=========================" -ForegroundColor Cyan
Write-Host ""

# Get current version from Cargo.toml if not specified
if (!$Version) {
    $cargoToml = Get-Content "$PSScriptRoot\..\rust-core\Cargo.toml" -Raw
    if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
        $Version = $matches[1]
    } else {
        Write-Host "Could not determine version from Cargo.toml" -ForegroundColor Red
        exit 1
    }
}

Write-Host "Version: $Version" -ForegroundColor Gray
Write-Host ""

# Check if tag already exists
$existingTag = git tag -l "v$Version" 2>$null
if ($existingTag) {
    Write-Host "Tag v$Version already exists!" -ForegroundColor Red
    exit 1
}

# Check for uncommitted changes
$status = git status --porcelain
if ($status) {
    Write-Host "Working directory not clean. Commit or stash changes first." -ForegroundColor Red
    exit 1
}

# Run tests
Write-Host "Running tests..." -ForegroundColor Yellow
Push-Location "$PSScriptRoot\..\rust-core"
try {
    $testOutput = cargo test --workspace 2>&1 | Out-String
    if ($testOutput -match "(\d+) failed") {
        if ([int]$matches[1] -gt 0) {
            Write-Host "Tests failed! Fix before releasing." -ForegroundColor Red
            exit 1
        }
    }
    Write-Host "Tests passed." -ForegroundColor Green
} finally {
    Pop-Location
}

# Extract changelog for this version
$changelog = Get-Content "$PSScriptRoot\..\CHANGELOG.md" -Raw
$versionPattern = "## \[$Version\].*?(?=## \[|\z)"
if ($changelog -match $versionPattern) {
    $releaseNotes = $matches[0]
} else {
    Write-Host "Warning: Could not extract changelog for v$Version" -ForegroundColor Yellow
    $releaseNotes = "Release v$Version"
}

# Build artifacts
Write-Host "Building release artifacts..." -ForegroundColor Yellow

# Build Rust library
Push-Location "$PSScriptRoot\..\rust-core"
try {
    cargo build --release --workspace 2>&1 | Out-Null
    Write-Host "Rust library built." -ForegroundColor Green
} finally {
    Pop-Location
}

# Build Windows C++ (if possible)
$buildScript = "$PSScriptRoot\build-windows.ps1"
if (Test-Path $buildScript) {
    Write-Host "Building Windows C++..." -ForegroundColor Yellow
    & $buildScript -Release
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Windows C++ built." -ForegroundColor Green
    } else {
        Write-Host "Windows C++ build failed (non-fatal)." -ForegroundColor Yellow
    }
}

# Create release
if ($DryRun) {
    Write-Host ""
    Write-Host "DRY RUN - would create:" -ForegroundColor Yellow
    Write-Host "  Tag: v$Version" -ForegroundColor Gray
    Write-Host "  Release notes:" -ForegroundColor Gray
    Write-Host $releaseNotes
    Write-Host ""
    Write-Host "Artifacts:" -ForegroundColor Gray
    Get-ChildItem "$PSScriptRoot\..\rust-core\target\release\*.dll" -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host "  - $($_.Name)" -ForegroundColor Gray
    }
} else {
    # Create tag
    Write-Host "Creating tag v$Version..." -ForegroundColor Yellow
    git tag -a "v$Version" -m "Release v$Version"
    git push origin "v$Version"
    
    # Create GitHub release
    Write-Host "Creating GitHub release..." -ForegroundColor Yellow
    gh release create "v$Version" --title "SoundBridge v$Version" --notes $releaseNotes
    
    Write-Host ""
    Write-Host "Release v$Version created!" -ForegroundColor Green
    Write-Host "Check: https://github.com/ICU-bit/soundbridge/releases/tag/v$Version" -ForegroundColor Cyan
}
