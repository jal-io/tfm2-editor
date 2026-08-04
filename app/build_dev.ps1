$ErrorActionPreference = "Stop"

$cargoToml = Join-Path $PSScriptRoot "Cargo.toml"
$cargoText = Get-Content $cargoToml -Raw
if ($cargoText -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    throw "Could not read package version from: $cargoToml"
}
$cargoVersion = $Matches[1]
$version = $cargoVersion
if ($cargoText -match '(?m)^dev_version\s*=\s*"([^"]+)"') {
    $version = $Matches[1]
}

Write-Host "Building TFM2 Editor Development v$version-dev..."
cargo build --release --features dev
if ($LASTEXITCODE -ne 0) {
    throw "Cargo Development build failed with exit code $LASTEXITCODE"
}

$sourceExe = Join-Path $PSScriptRoot "target\release\tfm2_editor.exe"
if (-not (Test-Path $sourceExe)) {
    throw "Build finished but executable was not found at: $sourceExe"
}

$distDir = Join-Path $PSScriptRoot "dist"
New-Item -ItemType Directory -Force -Path $distDir | Out-Null

$outputExe = Join-Path $distDir "tfm2_editor_${version}_dev.exe"
Copy-Item -Force $sourceExe $outputExe

# Development builds copy every JSON locale, including en-US.json. The external
# English file is a hot-reloadable override used for translation work and diagnostics.
$localeSource = Join-Path $PSScriptRoot "locales"
$localeDest = Join-Path $distDir "locales"
if (Test-Path $localeDest) {
    Remove-Item -Recurse -Force $localeDest
}
if (Test-Path $localeSource) {
    $localeFiles = @(Get-ChildItem -Path $localeSource -Filter "*.json" -File)
    if ($localeFiles.Count -gt 0) {
        New-Item -ItemType Directory -Force -Path $localeDest | Out-Null
        $localeFiles | Copy-Item -Destination $localeDest -Force
    }
}

Write-Host "Development build successful: $outputExe"
Write-Host "Development locales copied to: $localeDest"
Write-Host "Built-in English fallback remains embedded in the executable."
