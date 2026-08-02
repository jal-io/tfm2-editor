$ErrorActionPreference = "Stop"

$cargoToml = Join-Path $PSScriptRoot "Cargo.toml"
$cargoText = Get-Content $cargoToml -Raw
if ($cargoText -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    throw "Could not read package version from: $cargoToml"
}
$cargoVersion = $Matches[1]
$version = $cargoVersion
if ($cargoText -match '(?m)^community_version\s*=\s*"([^"]+)"') {
    $version = $Matches[1]
}

Write-Host "Building TFM2 Editor Community v$version..."
cargo build --release
if ($LASTEXITCODE -ne 0) {
    throw "Cargo Community build failed with exit code $LASTEXITCODE"
}

$sourceExe = Join-Path $PSScriptRoot "target\release\tfm2_editor.exe"
if (-not (Test-Path $sourceExe)) {
    throw "Build finished but executable was not found at: $sourceExe"
}

$distDir = Join-Path $PSScriptRoot "dist"
New-Item -ItemType Directory -Force -Path $distDir | Out-Null

$outputExe = Join-Path $distDir "tfm2_editor_$version.exe"
Copy-Item -Force $sourceExe $outputExe

Write-Host "Community build successful: $outputExe"
