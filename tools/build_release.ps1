param(
    [string]$Version,
    [string]$DistDirectory,
    [string]$CargoTargetDirectory
)

$ErrorActionPreference = "Stop"

$root = [System.IO.Path]::GetFullPath(
    (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))
)
if ([string]::IsNullOrWhiteSpace($Version)) {
    $manifest = Get-Content -LiteralPath (Join-Path $root "Cargo.toml") -Raw
    $match = [regex]::Match(
        $manifest,
        '(?ms)\[workspace\.package\].*?^version\s*=\s*"([^"]+)"'
    )
    if (-not $match.Success) {
        throw "Could not read the workspace version from Cargo.toml."
    }
    $Version = $match.Groups[1].Value
}

if ([string]::IsNullOrWhiteSpace($DistDirectory)) {
    $DistDirectory = Join-Path $root "dist"
} elseif (-not [System.IO.Path]::IsPathRooted($DistDirectory)) {
    $DistDirectory = Join-Path $root $DistDirectory
}
$dist = [System.IO.Path]::GetFullPath($DistDirectory)

if ([string]::IsNullOrWhiteSpace($CargoTargetDirectory)) {
    $CargoTargetDirectory = Join-Path $root "build\release-target"
} elseif (-not [System.IO.Path]::IsPathRooted($CargoTargetDirectory)) {
    $CargoTargetDirectory = Join-Path $root $CargoTargetDirectory
}
$cargoTarget = [System.IO.Path]::GetFullPath($CargoTargetDirectory)

$stage = Join-Path $dist "package"
$archive = Join-Path $dist "APL-$Version-windows-x64.zip"
$checksum = "$archive.sha256"

$null = New-Item -ItemType Directory -Path $dist -Force
$resolvedStage = [System.IO.Path]::GetFullPath($stage)
if (-not $resolvedStage.StartsWith($dist + [System.IO.Path]::DirectorySeparatorChar)) {
    throw "Refusing to replace a package directory outside dist: $resolvedStage"
}
if (Test-Path -LiteralPath $resolvedStage) {
    Remove-Item -LiteralPath $resolvedStage -Recurse -Force
}

$previousCargoTarget = $env:CARGO_TARGET_DIR
$env:CARGO_TARGET_DIR = $cargoTarget
Push-Location $root
try {
    cargo build --release -p apl -p apl_studio
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo failed to build the APL release binaries."
    }
} finally {
    Pop-Location
    if ($null -eq $previousCargoTarget) {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_TARGET_DIR = $previousCargoTarget
    }
}

$libraryOutput = Join-Path $stage "libraries"
$librarySourceOutput = Join-Path $libraryOutput "source"
$compilerOutput = Join-Path $stage "compiler"
$compilerSourceOutput = Join-Path $compilerOutput "std"
$nativeRuntimeOutput = Join-Path $compilerOutput "native-runtime"
$exampleOutput = Join-Path $stage "examples"
$documentationOutput = Join-Path $stage "docs"
New-Item -ItemType Directory -Path $librarySourceOutput -Force | Out-Null
New-Item -ItemType Directory -Path $compilerSourceOutput -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $nativeRuntimeOutput "src") -Force | Out-Null
New-Item -ItemType Directory -Path $exampleOutput -Force | Out-Null
New-Item -ItemType Directory -Path $documentationOutput -Force | Out-Null

$cli = Join-Path $cargoTarget "release\apl.exe"
$studio = Join-Path $cargoTarget "release\apl_studio.exe"
Copy-Item -LiteralPath $cli -Destination (Join-Path $stage "apl.exe")
Copy-Item -LiteralPath $studio -Destination (Join-Path $stage "apl_studio.exe")

# The CLI resolves installed modules from a libraries directory beside itself.
# Populate an isolated compiler-side directory as modules are built so release
# compilation never succeeds by accidentally finding stale workspace artifacts.
$compilerLibraryOutput = [System.IO.Path]::GetFullPath(
    (Join-Path (Split-Path -Parent $cli) "libraries")
)
$cargoTargetPrefix = $cargoTarget.TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
) + [System.IO.Path]::DirectorySeparatorChar
if (-not $compilerLibraryOutput.StartsWith(
    $cargoTargetPrefix,
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Refusing to replace compiler libraries outside the Cargo target: $compilerLibraryOutput"
}
if (Test-Path -LiteralPath $compilerLibraryOutput) {
    Remove-Item -LiteralPath $compilerLibraryOutput -Recurse -Force
}
New-Item -ItemType Directory -Path $compilerLibraryOutput -Force | Out-Null

$libraries = Get-ChildItem -LiteralPath (Join-Path $root "libraries") -Filter "*.apl" -File |
    Sort-Object Name |
    ForEach-Object { $_.BaseName }
foreach ($library in $libraries) {
    $source = Join-Path $root "libraries\$library.apl"
    $artifact = Join-Path $libraryOutput "$library.aplmod"
    & $cli emit-module $source $artifact
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to compile release library '$library'."
    }
    Copy-Item -LiteralPath $artifact -Destination $compilerLibraryOutput -Force
    $sourceDestination = Join-Path $librarySourceOutput "$library.apl"
    [System.IO.File]::Copy($source, $sourceDestination, $true)
    $metadata = Join-Path $root "libraries\$library.metadata.toml"
    if (Test-Path -LiteralPath $metadata) {
        Copy-Item -LiteralPath $metadata -Destination (Join-Path $libraryOutput "$library.metadata.toml") -Force
        Copy-Item -LiteralPath $metadata -Destination (Join-Path $librarySourceOutput "$library.metadata.toml") -Force
        Copy-Item -LiteralPath $metadata -Destination $compilerLibraryOutput -Force
    }
}

Get-ChildItem -LiteralPath (Join-Path $root "std") -Filter "*.apl" -File |
    Copy-Item -Destination $compilerSourceOutput
Copy-Item -LiteralPath (Join-Path $root "crates\apl_native_runtime\Cargo.toml") -Destination $nativeRuntimeOutput
Copy-Item -LiteralPath (Join-Path $root "crates\apl_native_runtime\Cargo.lock") -Destination $nativeRuntimeOutput
Copy-Item -LiteralPath (Join-Path $root "crates\apl_native_runtime\src\lib.rs") -Destination (Join-Path $nativeRuntimeOutput "src")
$runtimePathDependencies = @("apl_raw_memory", "apl_crypto", "apl_windows")
foreach ($dependency in $runtimePathDependencies) {
    $source = Join-Path $root "crates\$dependency"
    $destination = Join-Path $compilerOutput $dependency
    New-Item -ItemType Directory -Path (Join-Path $destination "src") -Force | Out-Null
    $manifest = Get-Content -LiteralPath (Join-Path $source "Cargo.toml") -Raw
    $manifest = $manifest.Replace("edition.workspace = true", 'edition = "2021"')
    $manifest = $manifest.Replace("license.workspace = true", 'license = "MIT"')
    $manifest = $manifest.Replace("version.workspace = true", "version = `"$Version`"")
    Set-Content -LiteralPath (Join-Path $destination "Cargo.toml") -Value ($manifest + "`n[workspace]`n") -Encoding ascii
    Copy-Item -Path (Join-Path $source "src\*") -Destination (Join-Path $destination "src") -Recurse -Force
}
$runtimeManifest = Join-Path $nativeRuntimeOutput "Cargo.toml"
Add-Content -LiteralPath $runtimeManifest -Value "`n[workspace]" -Encoding ascii
$vendorOutput = Join-Path $nativeRuntimeOutput "vendor"
cargo vendor --locked --offline --manifest-path $runtimeManifest $vendorOutput | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "Cargo failed to vendor the locked portable native runtime dependencies offline."
}

Get-ChildItem -LiteralPath (Join-Path $root "examples") -Filter "*.apl" -File |
    Copy-Item -Destination $exampleOutput
Get-ChildItem -LiteralPath (Join-Path $root "docs") -Filter "*.md" -File |
    Copy-Item -Destination $documentationOutput
Copy-Item -LiteralPath (Join-Path $root "README.md") -Destination $stage
Copy-Item -LiteralPath (Join-Path $root "LICENSE") -Destination $stage

$distributionManifest = @"
format = 1
name = "APL"
version = "$Version"
target = "windows-x64"

[tools]
cli = "apl.exe"
studio = "apl_studio.exe"

[compiler]
self_hosted_source = "compiler/std"
native_runtime = "compiler/native-runtime"
native_vendor = "compiler/native-runtime/vendor"

[libraries]
artifacts = "libraries"
source = "libraries/source"
"@
Set-Content -LiteralPath (Join-Path $stage "apl-distribution.toml") -Value $distributionManifest -Encoding ascii

if (Test-Path -LiteralPath $archive) {
    Remove-Item -LiteralPath $archive -Force
}
if (Test-Path -LiteralPath $checksum) {
    Remove-Item -LiteralPath $checksum -Force
}
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $archive -CompressionLevel Optimal
$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
Set-Content -LiteralPath $checksum -Value "$hash  $([System.IO.Path]::GetFileName($archive))" -Encoding ascii

Write-Host "APL package: $stage"
Write-Host "APL archive: $archive"
Write-Host "SHA-256: $hash"
