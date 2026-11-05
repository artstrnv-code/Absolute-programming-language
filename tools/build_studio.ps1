param(
    [string]$DistributionDirectory,
    [string]$CargoTargetDirectory
)

$ErrorActionPreference = "Stop"

$root = [System.IO.Path]::GetFullPath(
    (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))
)
if ([string]::IsNullOrWhiteSpace($DistributionDirectory)) {
    $DistributionDirectory = Join-Path $root "dist\package"
} elseif (-not [System.IO.Path]::IsPathRooted($DistributionDirectory)) {
    $DistributionDirectory = Join-Path $root $DistributionDirectory
}
$distribution = [System.IO.Path]::GetFullPath($DistributionDirectory)

if ([string]::IsNullOrWhiteSpace($CargoTargetDirectory)) {
    $CargoTargetDirectory = Join-Path $root "build\studio-target"
} elseif (-not [System.IO.Path]::IsPathRooted($CargoTargetDirectory)) {
    $CargoTargetDirectory = Join-Path $root $CargoTargetDirectory
}
$cargoTarget = [System.IO.Path]::GetFullPath($CargoTargetDirectory)

$previousCargoTarget = $env:CARGO_TARGET_DIR
$env:CARGO_TARGET_DIR = $cargoTarget
Push-Location $root
try {
    cargo build --release -p apl -p apl_studio
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo failed to build APL Studio."
    }

    $cargoStudio = Join-Path $cargoTarget "release\apl_studio.exe"
    $cargoCli = Join-Path $cargoTarget "release\apl.exe"
    New-Item -ItemType Directory -Path $distribution -Force | Out-Null
    $studio = Join-Path $distribution "apl_studio.exe"
    Copy-Item -LiteralPath $cargoStudio -Destination $studio -Force
    Copy-Item -LiteralPath $cargoCli -Destination (Join-Path $distribution "apl.exe") -Force
    & (Join-Path $root "tools\build_libraries.ps1") `
        -OutputDirectory (Join-Path $distribution "libraries") `
        -CliPath $cargoCli
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to build the standard libraries for APL Studio."
    }
    Write-Host "APL Studio: $studio"
} finally {
    Pop-Location
    if ($null -eq $previousCargoTarget) {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_TARGET_DIR = $previousCargoTarget
    }
}
