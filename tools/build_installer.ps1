param(
    [string]$Version,
    [switch]$SkipRelease
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

if (-not $SkipRelease) {
    & (Join-Path $root "tools\build_release.ps1") -Version $Version
    if ($LASTEXITCODE -ne 0) {
        throw "APL release build failed."
    }
}

$package = Join-Path $root "dist\package"
& (Join-Path $root "tools\verify_package.ps1") -PackageDirectory $package
if ($LASTEXITCODE -ne 0) {
    throw "APL package verification failed."
}

$candidates = @(
    (Get-Command ISCC.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1),
    (Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe")
) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
$compiler = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $compiler) {
    throw "Inno Setup 6 was not found. Install it, then run this script again."
}

$script = Join-Path $root "installer\windows\apl.iss"
& $compiler "/DAppVersion=$Version" $script
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup failed to build the APL installer."
}

Write-Host "APL installer directory: $(Join-Path $root 'dist\installer')"
