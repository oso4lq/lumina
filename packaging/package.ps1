# Сборка portable-дистрибутива Lumina в dist/Lumina-<версия>-x64.zip
# Запуск:  pwsh -File packaging/package.ps1 -ExiftoolDir "C:\path\to\exiftool-standalone"
param(
    [string]$VcpkgBin = "$env:VCPKG_ROOT\installed\x64-windows\bin",
    [Parameter(Mandatory=$true)][string]$ExiftoolDir  # папка с exiftool.exe + exiftool_files\
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# Версия из Cargo.toml
$version = (Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value

Write-Host "cargo build --release..."
cargo build --release

$stage = Join-Path $root "dist\Lumina"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Path $stage | Out-Null
New-Item -ItemType Directory -Path (Join-Path $stage "licenses") | Out-Null

# 1) exe
Copy-Item "target\release\lumina.exe" $stage

# 2) нативные DLL
foreach ($dll in @("heif.dll","libde265.dll","libx265.dll")) {
    Copy-Item (Join-Path $VcpkgBin $dll) $stage
}

# 3) exiftool standalone (+ exiftool_files)
Copy-Item (Join-Path $ExiftoolDir "exiftool.exe") $stage
Copy-Item (Join-Path $ExiftoolDir "exiftool_files") $stage -Recurse

# 4) README
Copy-Item "packaging\README.txt" $stage

# 5) zip
$zip = Join-Path $root "dist\Lumina-$version-x64.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip
Write-Host "готово: $zip"
