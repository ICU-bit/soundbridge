# SoundBridge 打包脚本
# 用法: .\scripts\package.ps1 [-Platform all|windows|android] [-Config release|debug]

param(
    [string]$Platform = "all",
    [string]$Config = "release"
)

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$DistDir = Join-Path $ProjectRoot "dist"

Write-Host "=== SoundBridge Packaging ===" -ForegroundColor Cyan
Write-Host "Platform: $Platform"
Write-Host "Config: $Config"
Write-Host ""

# 清理旧的分发目录
if (Test-Path $DistDir) {
    Write-Host "Cleaning old dist..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force $DistDir
}

# 打包 Windows
if ($Platform -eq "all" -or $Platform -eq "windows") {
    Write-Host ""
    Write-Host "--- Windows Packaging ---" -ForegroundColor Green
    & (Join-Path $PSScriptRoot "package-windows.ps1") -Config $Config -OutputDir "dist\windows"
}

# 打包 Android
if ($Platform -eq "all" -or $Platform -eq "android") {
    Write-Host ""
    Write-Host "--- Android Packaging ---" -ForegroundColor Green
    & (Join-Path $PSScriptRoot "package-android.ps1") -BuildType $Config -OutputDir "dist\android"
}

# 显示总结
Write-Host ""
Write-Host "=== Packaging Summary ===" -ForegroundColor Cyan
Write-Host "Distribution directory: $DistDir"
Write-Host ""

if (Test-Path $DistDir) {
    Get-ChildItem $DistDir -Recurse -File | ForEach-Object {
        $RelativePath = $_.FullName.Substring($DistDir.Length + 1)
        $Size = $_.Length / 1MB
        Write-Host "  $RelativePath ($([math]::Round($Size, 2)) MB)"
    }
}

Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "  1. Test the packages locally"
Write-Host "  2. Create a git tag: git tag v1.0.0"
Write-Host "  3. Push the tag: git push origin v1.0.0"
Write-Host "  4. GitHub Actions will automatically create a release"
