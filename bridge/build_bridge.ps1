param(
    [string]$Project = $PSScriptRoot,
    [string]$SdkDir = (Join-Path $PSScriptRoot "..\..\mod-sdk")
)

$ErrorActionPreference = "Stop"
$expectedSdkVersion = "0.5.4"

$sdk = Resolve-Path -LiteralPath $SdkDir
$depsDir = Join-Path $sdk "deps"
$nativeDir = Join-Path $sdk "native"
$baseVersionFile = Join-Path $sdk "base_version.txt"
$toolchainFile = Join-Path $sdk "rust-toolchain.toml"

if (-not (Test-Path -LiteralPath $baseVersionFile)) {
    Write-Error "TFM2 Mod SDK base_version.txt not found: $baseVersionFile"
}

$sdkVersion = (Get-Content -LiteralPath $baseVersionFile -Raw).Trim()
if ($sdkVersion -ne $expectedSdkVersion) {
    Write-Error "This bridge build targets TFM2 Mod SDK $expectedSdkVersion, but found SDK $sdkVersion at $sdk"
}
Write-Host "Using TFM2 Mod SDK $sdkVersion"

$pinned = Select-String -LiteralPath $toolchainFile -Pattern '^\s*channel\s*=\s*"([^"]+)"' -ErrorAction SilentlyContinue |
    ForEach-Object { $_.Matches[0].Groups[1].Value } |
    Select-Object -First 1
if ($pinned) {
    $env:RUSTUP_TOOLCHAIN = $pinned
    Write-Host "Using Rust toolchain $pinned"
}

# TFM2 0.5.4 classic SDK rlibs contain LLVM bitcode objects. MSVC link.exe
# cannot consume those archives directly (LNK1107). Use the rust-lld shipped
# with the SDK-pinned Rust toolchain so the linker can perform LLVM LTO.
$sysroot = (& rustc --print sysroot).Trim()
if ($LASTEXITCODE -ne 0 -or -not $sysroot) {
    Write-Error "Could not determine Rust sysroot. Ensure Rust/rustup is installed."
}

$rustLld = Join-Path $sysroot "lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe"
if (-not (Test-Path -LiteralPath $rustLld)) {
    Write-Error "rust-lld.exe not found for the pinned Rust toolchain: $rustLld"
}
Write-Host "Using Rust LLD linker: $rustLld"

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
    "--extern", "game_core=$($gameCore.FullName)",
    "-C", "linker=$rustLld",
    "-C", "linker-flavor=lld-link"
)
if (Test-Path -LiteralPath $nativeDir) { $flags += @("-L", "native=$nativeDir") }
$env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]31

# Avoid stale artifacts from older SDK builds while testing the 0.5.4 bridge.
if (Test-Path -LiteralPath $targetDir) {
    Write-Host "Cleaning previous bridge target directory..."
    Remove-Item -LiteralPath $targetDir -Recurse -Force
}

cargo rustc --release --manifest-path $manifest --target-dir $targetDir --lib -- --crate-type cdylib
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$dllName = ($target.name -replace "-", "_") + ".dll"
$builtDll = Join-Path (Join-Path $targetDir "release") $dllName
if (-not (Test-Path -LiteralPath $builtDll)) { Write-Error "Cargo build finished, but expected DLL was not found: $builtDll" }

$outDll = Join-Path $modRoot "$modId.dll"
Copy-Item -LiteralPath $builtDll -Destination $outDll -Force
Write-Host "Build successful: $outDll"
