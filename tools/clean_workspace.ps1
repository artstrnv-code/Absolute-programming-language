$ErrorActionPreference = "Stop"

$root = [System.IO.Path]::GetFullPath(
    (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))
)
$targets = @(
    (Join-Path $root "build"),
    (Join-Path $root "target")
)

foreach ($target in $targets) {
    $resolved = [System.IO.Path]::GetFullPath($target)
    if (-not $resolved.StartsWith($root + [System.IO.Path]::DirectorySeparatorChar)) {
        throw "Refusing to remove path outside the repository: $resolved"
    }
    if (Test-Path -LiteralPath $resolved) {
        Remove-Item -LiteralPath $resolved -Recurse -Force
        Write-Host "Removed $resolved"
    }
}

$temporarySource = Join-Path $root ".apl_gui_run.apl"
if (Test-Path -LiteralPath $temporarySource) {
    Remove-Item -LiteralPath $temporarySource -Force
    Write-Host "Removed $temporarySource"
}
