# Android 打包脚本
# 用法: .\scripts\package-android.ps1 [-BuildType release|debug] [-OutputDir dist]

param(
    [string]$BuildType = "release",
    [string]$OutputDir = "dist"
)

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$AndroidDir = Join-Path $ProjectRoot "android"
$PackageDir = Join-Path $ProjectRoot $OutputDir

Write-Host "=== SoundBridge Android Packaging ===" -ForegroundColor Cyan
Write-Host "Build Type: $BuildType"
Write-Host "Output: $PackageDir"
Write-Host ""

# 检查 gradlew 是否存在
$Gradlew = Join-Path $AndroidDir "gradlew"
if (-not (Test-Path $Gradlew)) {
    Write-Host "Error: gradlew not found in $AndroidDir" -ForegroundColor Red
    exit 1
}

# 清理旧的构建
Write-Host "Cleaning old build..." -ForegroundColor Yellow
Push-Location $AndroidDir
& $Gradlew clean
Pop-Location

# 构建 AAB
Write-Host "Building AAB..." -ForegroundColor Green
Push-Location $AndroidDir
if ($BuildType -eq "release") {
    & $Gradlew bundleRelease
} else {
    & $Gradlew bundleDebug
}
Pop-Location

# 构建 APK
Write-Host "Building APK..." -ForegroundColor Green
Push-Location $AndroidDir
if ($BuildType -eq "release") {
    & $Gradlew assembleRelease
} else {
    & $Gradlew assembleDebug
}
Pop-Location

# 复制产物到输出目录
Write-Host "Copying artifacts..." -ForegroundColor Green
New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null

$AabDir = Join-Path $AndroidDir "app\build\outputs\bundle\$BuildType"
$ApkDir = Join-Path $AndroidDir "app\build\outputs\apk\$BuildType"

if (Test-Path $AabDir) {
    Copy-Item "$AabDir\*.aab" $PackageDir -ErrorAction SilentlyContinue
}

if (Test-Path $ApkDir) {
    Copy-Item "$ApkDir\*.apk" $PackageDir -ErrorAction SilentlyContinue
}

# 显示结果
Write-Host ""
Write-Host "=== Packaging Complete ===" -ForegroundColor Cyan
Write-Host "Output directory: $PackageDir"
Write-Host "Artifacts:"
Get-ChildItem $PackageDir -Filter "*.aab" | ForEach-Object {
    $Size = $_.Length / 1MB
    Write-Host "  - $($_.Name) ($([math]::Round($Size, 2)) MB)"
}
Get-ChildItem $PackageDir -Filter "*.apk" | ForEach-Object {
    $Size = $_.Length / 1MB
    Write-Host "  - $($_.Name) ($([math]::Round($Size, 2)) MB)"
}

Write-Host ""
Write-Host "Note: AAB files need to be signed before uploading to Google Play." -ForegroundColor Yellow
Write-Host "Use: jarsigner -verbose -sigalg SHA256withRSA -digestalg SHA-256 -keystore your.keystore app-release.aab your_alias" -ForegroundColor Yellow
