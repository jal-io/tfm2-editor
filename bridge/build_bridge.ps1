param(
    [string]$Project = $PSScriptRoot,
    [string]$SdkDir = (Join-Path $PSScriptRoot "..\..\mod-sdk")
)

$ErrorActionPreference = "Stop"

$sdk = Resolve-Path -LiteralPath $SdkDir
$depsDir = Join-Path $sdk "deps"
$nativeDir = Join-Path $sdk "native"

$pinned = Select-String -LiteralPath (Join-Path $sdk "rust-toolchain.toml") -Pattern '^\s*channel\s*=\s*"([^"]+)"' -ErrorAction SilentlyContinue |
    ForEach-Object { $_.Matches[0].Groups[1].Value } |
    Select-Object -First 1
if ($pinned) { $env:RUSTUP_TOOLCHAIN = $pinned }

$modApi = Get-ChildItem -LiteralPath $depsDir -Filter "libmod_api-*.rlib" | Select-Object -First 1
if (-not $modApi) { Write-Error "libmod_api .rlib not found in $depsDir" }

$gameCore = Get-ChildItem -LiteralPath $depsDir -Filter "libgame_core-*.rlib" | Select-Object -First 1
if (-not $gameCore) { Write-Error "libgame_core .rlib not found in $depsDir" }

$projectPath = Resolve-Path -LiteralPath $Project
$manifest = Join-Path $projectPath "Cargo.toml"
if (-not (Test-Path -LiteralPath $manifest)) { Write-Error "Cargo.toml not found: $manifest" }

$modRoot = Split-Path -Parent $manifest
$modId = Split-Path -Leaf $modRoot
$targetDir = Join-Path $modRoot "target"

$metadataText = cargo metadata --no-deps --format-version 1 --manifest-path $manifest
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$metadata = $metadataText | ConvertFrom-Json
$target = $metadata.packages[0].targets | Where-Object {
    $_.kind -contains "lib" -or $_.kind -contains "rlib" -or $_.kind -contains "dylib" -or $_.kind -contains "cdylib"
} | Select-Object -First 1
if (-not $target) { Write-Error "Cargo.toml must define a library target." }

$flags = @(
    "-L", "dependency=$depsDir",
    "--extern", "mod_api=$($modApi.FullName)",
    "--extern", "game_core=$($gameCore.FullName)"
)
if (Test-Path -LiteralPath $nativeDir) { $flags += @("-L", "native=$nativeDir") }
$env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]31

cargo rustc --release --manifest-path $manifest --target-dir $targetDir --lib -- --crate-type cdylib
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$dllName = ($target.name -replace "-", "_") + ".dll"
$builtDll = Join-Path (Join-Path $targetDir "release") $dllName
if (-not (Test-Path -LiteralPath $builtDll)) { Write-Error "Cargo build finished, but expected DLL was not found: $builtDll" }

$outDll = Join-Path $modRoot "$modId.dll"
Copy-Item -LiteralPath $builtDll -Destination $outDll -Force
Write-Host "Build successful: $outDll"
