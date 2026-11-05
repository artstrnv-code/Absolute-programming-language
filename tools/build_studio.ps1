$ErrorActionPreference = "Stop"

$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Push-Location $root
try {
    cargo build --release -p apl -p apl_studio
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo failed to build APL Studio."
    }

    $cargoStudio = Join-Path $root "target\release\apl_studio.exe"
    $cargoCli = Join-Path $root "target\release\apl.exe"
    $distribution = Join-Path $root "dist\package"
    New-Item -ItemType Directory -Path $distribution -Force | Out-Null
    $studio = Join-Path $distribution "apl_studio.exe"
    Copy-Item -LiteralPath $cargoStudio -Destination $studio -Force
    Copy-Item -LiteralPath $cargoCli -Destination (Join-Path $distribution "apl.exe") -Force
    & (Join-Path $root "tools\build_libraries.ps1") -OutputDirectory "dist\package\libraries"
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to build the standard libraries for APL Studio."
    }
    Write-Host "APL Studio: $studio"
} finally {
    Pop-Location
}
