$ErrorActionPreference = "Stop"

Write-Host "Building TFM2 Editor Development release..."
cargo build --release --features dev

$exe = Join-Path $PSScriptRoot "target\release\tfm2_editor.exe"
if (-not (Test-Path $exe)) {
    throw "Build finished but executable was not found at: $exe"
}

Write-Host "Development build successful: $exe"
