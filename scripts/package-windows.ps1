# Windows 打包脚本
# 用法: .\scripts\package-windows.ps1 [-Config Release|Debug] [-OutputDir dist]

param(
    [string]$Config = "Release",
    [string]$OutputDir = "dist"
)

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$BuildDir = Join-Path $ProjectRoot "windows\build"
$PackageDir = Join-Path $ProjectRoot $OutputDir
$AppDir = Join-Path $PackageDir "SoundBridge"

Write-Host "=== SoundBridge Windows Packaging ===" -ForegroundColor Cyan
Write-Host "Config: $Config"
Write-Host "Output: $AppDir"
Write-Host ""

# 清理旧的构建
if (Test-Path $BuildDir) {
    Write-Host "Cleaning old build..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force $BuildDir
}

# 创建构建目录
New-Item -ItemType Directory -Path $BuildDir -Force | Out-Null

# 配置 CMake
Write-Host "Configuring CMake..." -ForegroundColor Green
$VcpkgToolchain = "C:\vcpkg\scripts\buildsystems\vcpkg.cmake"
if (Test-Path $VcpkgToolchain) {
    cmake -B $BuildDir -S (Join-Path $ProjectRoot "windows") `
        -DCMAKE_TOOLCHAIN_FILE=$VcpkgToolchain `
        -DCMAKE_BUILD_TYPE=$Config
} else {
    Write-Host "Warning: vcpkg not found, building without opus/spdlog" -ForegroundColor Yellow
    cmake -B $BuildDir -S (Join-Path $ProjectRoot "windows") `
        -DCMAKE_BUILD_TYPE=$Config
}

# 构建
Write-Host "Building..." -ForegroundColor Green
cmake --build $BuildDir --config $Config

# 创建打包目录
Write-Host "Packaging..." -ForegroundColor Green
if (Test-Path $AppDir) {
    Remove-Item -Recurse -Force $AppDir
}
New-Item -ItemType Directory -Path $AppDir -Force | Out-Null

# 复制可执行文件和 DLL
$BinDir = Join-Path $BuildDir "bin\$Config"
if (Test-Path $BinDir) {
    Copy-Item "$BinDir\*.exe" $AppDir -ErrorAction SilentlyContinue
    Copy-Item "$BinDir\*.dll" $AppDir -ErrorAction SilentlyContinue
}

# 复制 Rust DLL
$RustReleaseDir = Join-Path $ProjectRoot "rust-core\target\release"
if (Test-Path $RustReleaseDir) {
    Copy-Item "$RustReleaseDir\*.dll" $AppDir -ErrorAction SilentlyContinue
}

# 复制资源文件
$AssetsDir = Join-Path $ProjectRoot "windows\assets"
if (Test-Path $AssetsDir) {
    Copy-Item -Recurse $AssetsDir (Join-Path $AppDir "assets")
}

# 创建 ZIP 包
$ZipFile = Join-Path $PackageDir "SoundBridge-Windows-x64.zip"
if (Test-Path $ZipFile) {
    Remove-Item -Force $ZipFile
}

Write-Host "Creating ZIP archive..." -ForegroundColor Green
Compress-Archive -Path $AppDir -DestinationPath $ZipFile -CompressionLevel Optimal

$ZipSize = (Get-Item $ZipFile).Length / 1MB
Write-Host ""
Write-Host "=== Packaging Complete ===" -ForegroundColor Cyan
Write-Host "Package: $ZipFile ($([math]::Round($ZipSize, 2)) MB)"
Write-Host "Contents:"
Get-ChildItem $AppDir | ForEach-Object {
    Write-Host "  - $($_.Name)"
}
