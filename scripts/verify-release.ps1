# SoundBridge Pre-Release Verification
# Runs all checks to verify project is ready for release
# Usage: .\scripts\verify-release.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "SoundBridge Pre-Release Verification" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan
Write-Host ""

$checks = @()
$passed = 0
$failed = 0

function Add-Check {
    param([string]$Name, [bool]$Result, [string]$Details = "")
    $script:checks += @{
        Name = $Name
        Result = $Result
        Details = $Details
    }
    if ($Result) {
        $script:passed++
        Write-Host "  ✓ $Name" -ForegroundColor Green
    } else {
        $script:failed++
        Write-Host "  ✗ $Name" -ForegroundColor Red
        if ($Details) {
            Write-Host "    $Details" -ForegroundColor Gray
        }
    }
}

# 1. Check git status
Write-Host "Checking git status..." -ForegroundColor Yellow
$status = git status --porcelain 2>&1 | Out-String
Add-Check "Working directory clean" ($status.Trim().Length -eq 0) "Uncommitted changes detected"

# 2. Check if on main branch
$branch = git rev-parse --abbrev-ref HEAD 2>&1 | Out-String
Add-Check "On master branch" ($branch.Trim() -eq "master") "Current branch: $($branch.Trim())"

# 3. Check Rust tests
Write-Host ""
Write-Host "Running Rust tests..." -ForegroundColor Yellow
Push-Location "$PSScriptRoot\..\rust-core"
try {
    $testOutput = cargo test --workspace 2>&1 | Out-String
    $testPassed = $testOutput -notmatch "(\d+) failed"
    Add-Check "Rust tests pass" $testPassed
    
    # Count tests
    if ($testOutput -match "(\d+) passed") {
        Write-Host "    Tests passed: $($matches[1])" -ForegroundColor Gray
    }
} finally {
    Pop-Location
}

# 4. Check clippy
Write-Host ""
Write-Host "Running clippy..." -ForegroundColor Yellow
Push-Location "$PSScriptRoot\..\rust-core"
try {
    $clippyOutput = cargo clippy --workspace 2>&1 | Out-String
    $clippyClean = $clippyOutput -notmatch "warning:|error:"
    Add-Check "Clippy clean" $clippyClean
} finally {
    Pop-Location
}

# 5. Check formatting
Write-Host ""
Write-Host "Checking formatting..." -ForegroundColor Yellow
Push-Location "$PSScriptRoot\..\rust-core"
try {
    $fmtOutput = cargo fmt -- --check 2>&1 | Out-String
    $fmtClean = $LASTEXITCODE -eq 0
    Add-Check "Formatting clean" $fmtClean
} finally {
    Pop-Location
}

# 6. Check CI config exists
Write-Host ""
Write-Host "Checking project files..." -ForegroundColor Yellow
$requiredFiles = @(
    ".github/workflows/ci.yml",
    ".editorconfig",
    "rustfmt.toml",
    "CHANGELOG.md",
    "CONTRIBUTING.md",
    "AGENTS.md"
)

foreach ($file in $requiredFiles) {
    $exists = Test-Path "$PSScriptRoot\..\$file"
    Add-Check "File exists: $file" $exists
}

# 7. Check docs are up to date
Write-Host ""
Write-Host "Checking documentation..." -ForegroundColor Yellow
$docs = Get-ChildItem "$PSScriptRoot\..\docs\*.md" -ErrorAction SilentlyContinue
Add-Check "Documentation files exist" ($docs.Count -gt 0) "Found $($docs.Count) doc files"

# 8. Check AI_GUIDE.md in each crate
Write-Host ""
Write-Host "Checking AI_GUIDE.md files..." -ForegroundColor Yellow
$crates = Get-ChildItem "$PSScriptRoot\..\rust-core\crates\*" -Directory
$missingGuides = @()
foreach ($crate in $crates) {
    if (!(Test-Path "$crate\AI_GUIDE.md")) {
        $missingGuides += $crate.Name
    }
}
Add-Check "All crates have AI_GUIDE.md" ($missingGuides.Count -eq 0) "Missing: $($missingGuides -join ', ')"

# Summary
Write-Host ""
Write-Host "====================================" -ForegroundColor Cyan
Write-Host "Summary: $passed passed, $failed failed" -ForegroundColor $(if ($failed -gt 0) { "Red" } else { "Green" })
Write-Host ""

if ($failed -gt 0) {
    Write-Host "FAILED: Project is NOT ready for release!" -ForegroundColor Red
    Write-Host "Fix the issues above before releasing." -ForegroundColor Yellow
    exit 1
} else {
    Write-Host "SUCCESS: Project is ready for release!" -ForegroundColor Green
    exit 0
}
