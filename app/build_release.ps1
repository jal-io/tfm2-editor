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

# Community builds use the English locale embedded in the EXE. Only additional
# languages are distributed beside it. Clean the destination first so an old
# development en-US.json cannot accidentally leak into a release package.
$localeSource = Join-Path $PSScriptRoot "locales"
$localeDest = Join-Path $distDir "locales"
if (Test-Path $localeDest) {
    Remove-Item -Recurse -Force $localeDest
}

$additionalLocales = @()
if (Test-Path $localeSource) {
    $additionalLocales = @(
        Get-ChildItem -Path $localeSource -Filter "*.json" -File |
            Where-Object { $_.Name -ine "en-US.json" }
    )
}

if ($additionalLocales.Count -gt 0) {
    New-Item -ItemType Directory -Force -Path $localeDest | Out-Null
    $additionalLocales | Copy-Item -Destination $localeDest -Force
    Write-Host "Additional locales copied to: $localeDest"
} else {
    Write-Host "No external locales bundled. Community build uses embedded English only."
}

Write-Host "Community build successful: $outputExe"
