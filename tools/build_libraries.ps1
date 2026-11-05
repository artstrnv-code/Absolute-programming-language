param(
    [string]$OutputDirectory
)

$ErrorActionPreference = "Stop"

$root = [System.IO.Path]::GetFullPath(
    (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))
)
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $root "build\libraries"
} elseif (-not [System.IO.Path]::IsPathRooted($OutputDirectory)) {
    $OutputDirectory = Join-Path $root $OutputDirectory
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)

$cli = Join-Path $root "target\release\apl.exe"
if (-not (Test-Path -LiteralPath $cli)) {
    Push-Location $root
    try {
        cargo build --release -p apl
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo failed to build the APL CLI."
        }
    } finally {
        Pop-Location
    }
}

New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$libraries = Get-ChildItem -LiteralPath (Join-Path $root "libraries") -Filter "*.apl" -File |
    Sort-Object Name |
    ForEach-Object { $_.BaseName }
foreach ($library in $libraries) {
    $source = Join-Path $root "libraries\$library.apl"
    $artifact = Join-Path $OutputDirectory "$library.aplmod"
    & $cli emit-module $source $artifact
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to compile library '$library'."
    }
    $metadata = Join-Path $root "libraries\$library.metadata.toml"
    if (Test-Path -LiteralPath $metadata) {
        $metadataOutput = Join-Path $OutputDirectory "$library.metadata.toml"
        if ([System.IO.Path]::GetFullPath($metadata) -ne [System.IO.Path]::GetFullPath($metadataOutput)) {
            Copy-Item -LiteralPath $metadata -Destination $metadataOutput -Force
        }
    }
    Write-Host "APL library: $artifact"
}
