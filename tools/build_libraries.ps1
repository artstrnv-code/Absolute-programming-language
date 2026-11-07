param(
    [string]$OutputDirectory,
    [string]$CliPath
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

if ([string]::IsNullOrWhiteSpace($CliPath)) {
    $CliPath = Join-Path $root "target\release\apl.exe"
} elseif (-not [System.IO.Path]::IsPathRooted($CliPath)) {
    $CliPath = Join-Path $root $CliPath
}
$sourceCli = [System.IO.Path]::GetFullPath($CliPath)
if (-not (Test-Path -LiteralPath $sourceCli)) {
    if ($PSBoundParameters.ContainsKey("CliPath")) {
        throw "APL CLI was not found at '$sourceCli'."
    }
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
$compilerHost = [System.IO.Path]::GetFullPath(
    (Join-Path (Split-Path -Parent $sourceCli) "library-compiler")
)
$compilerLibraries = Join-Path $compilerHost "libraries"
New-Item -ItemType Directory -Path $compilerHost -Force | Out-Null
if (Test-Path -LiteralPath $compilerLibraries) {
    Remove-Item -LiteralPath $compilerLibraries -Recurse -Force
}
New-Item -ItemType Directory -Path $compilerLibraries -Force | Out-Null
$cli = Join-Path $compilerHost "apl.exe"
Copy-Item -LiteralPath $sourceCli -Destination $cli -Force

$libraries = Get-ChildItem -LiteralPath (Join-Path $root "libraries") -Filter "*.apl" -File |
    Sort-Object Name |
    ForEach-Object { $_.BaseName }
foreach ($library in $libraries) {
    $source = Join-Path $root "libraries\$library.apl"
    $artifact = Join-Path $OutputDirectory "$library.aplmod"
    & $cli build-lib $source $artifact
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to compile library '$library'."
    }
    Copy-Item -LiteralPath $artifact -Destination $compilerLibraries -Force
    $metadata = Join-Path $root "libraries\$library.metadata.toml"
    if (Test-Path -LiteralPath $metadata) {
        $metadataOutput = Join-Path $OutputDirectory "$library.metadata.toml"
        if ([System.IO.Path]::GetFullPath($metadata) -ne [System.IO.Path]::GetFullPath($metadataOutput)) {
            Copy-Item -LiteralPath $metadata -Destination $metadataOutput -Force
        }
        Copy-Item -LiteralPath $metadata -Destination $compilerLibraries -Force
    }
    Write-Host "APL library: $artifact"
}
