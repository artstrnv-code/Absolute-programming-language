param(
    [string]$PackageDirectory,
    [switch]$NativeSmoke
)

$ErrorActionPreference = "Stop"

$root = [System.IO.Path]::GetFullPath(
    (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))
)
if ([string]::IsNullOrWhiteSpace($PackageDirectory)) {
    $PackageDirectory = Join-Path $root "dist\package"
} elseif (-not [System.IO.Path]::IsPathRooted($PackageDirectory)) {
    $PackageDirectory = Join-Path $root $PackageDirectory
}
$package = [System.IO.Path]::GetFullPath($PackageDirectory)
if (-not (Test-Path -LiteralPath $package -PathType Container)) {
    throw "APL package directory was not found: $package"
}

function Require-File([string]$RelativePath) {
    $path = Join-Path $package $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "APL package is missing '$RelativePath'."
    }
    if ((Get-Item -LiteralPath $path).Length -eq 0) {
        throw "APL package contains an empty '$RelativePath'."
    }
    return $path
}

$requiredFiles = @(
    "apl.exe",
    "apl_studio.exe",
    "apl-distribution.toml",
    "README.md",
    "LICENSE",
    "examples\std_main.apl",
    "docs\installation.md",
    "compiler\std\bootstrap.apl",
    "compiler\std\lexer.apl",
    "compiler\std\parser.apl",
    "compiler\std\checker.apl",
    "compiler\std\linker.apl",
    "compiler\std\vm.apl",
    "compiler\native-runtime\Cargo.toml",
    "compiler\native-runtime\Cargo.lock",
    "compiler\native-runtime\src\lib.rs",
    "compiler\native-runtime\vendor\serde_json\Cargo.toml",
    "compiler\apl_raw_memory\Cargo.toml",
    "compiler\apl_crypto\Cargo.toml",
    "compiler\apl_windows\Cargo.toml"
)
foreach ($requiredFile in $requiredFiles) {
    $null = Require-File $requiredFile
}

$manifestPath = Join-Path $package "apl-distribution.toml"
$manifest = Get-Content -LiteralPath $manifestPath -Raw
$manifestEntries = @(
    'format = 1',
    'name = "APL"',
    'target = "windows-x64"',
    'cli = "apl.exe"',
    'studio = "apl_studio.exe"',
    'self_hosted_source = "compiler/std"',
    'native_runtime = "compiler/native-runtime"',
    'native_vendor = "compiler/native-runtime/vendor"',
    'artifacts = "libraries"',
    'source = "libraries/source"'
)
foreach ($entry in $manifestEntries) {
    if (-not $manifest.Contains($entry)) {
        throw "APL distribution manifest is missing '$entry'."
    }
}
if ($manifest -notmatch '(?m)^version = "[^"\r\n]+"\r?$') {
    throw "APL distribution manifest does not contain a version."
}

$runtimeLockPath = Join-Path $package "compiler\native-runtime\Cargo.lock"
$runtimeLock = Get-Content -LiteralPath $runtimeLockPath -Raw
$packageBlocks = [regex]::Matches(
    $runtimeLock,
    '(?ms)^\[\[package\]\]\s*(.*?)(?=^\[\[package\]\]|\z)'
)
$registryPackages = @()
foreach ($packageBlock in $packageBlocks) {
    if ($packageBlock.Value -notmatch '(?m)^source = "registry\+') {
        continue
    }
    $nameMatch = [regex]::Match($packageBlock.Value, '(?m)^name = "([^"]+)"\r?$')
    if (-not $nameMatch.Success) {
        throw "APL native runtime lockfile contains a registry package without a name."
    }
    $registryPackages += $nameMatch.Groups[1].Value
}
if ($registryPackages.Count -eq 0) {
    throw "APL native runtime lockfile does not contain registry dependencies."
}
foreach ($registryPackage in $registryPackages) {
    $null = Require-File ("compiler\native-runtime\vendor\{0}\Cargo.toml" -f $registryPackage)
    $null = Require-File ("compiler\native-runtime\vendor\{0}\.cargo-checksum.json" -f $registryPackage)
}

$libraryDirectory = Join-Path $package "libraries"
$librarySourceDirectory = Join-Path $libraryDirectory "source"
if (-not (Test-Path -LiteralPath $librarySourceDirectory -PathType Container)) {
    throw "APL package is missing the standard library source directory."
}
$librarySources = @(Get-ChildItem -LiteralPath $librarySourceDirectory -Filter "*.apl" -File)
$libraryArtifacts = @(Get-ChildItem -LiteralPath $libraryDirectory -Filter "*.aplmod" -File)
if ($librarySources.Count -eq 0 -or $libraryArtifacts.Count -eq 0) {
    throw "APL package does not contain standard library sources and artifacts."
}
foreach ($source in $librarySources) {
    $null = Require-File ("libraries\{0}.aplmod" -f $source.BaseName)
    $metadata = Join-Path $librarySourceDirectory "$($source.BaseName).metadata.toml"
    if (Test-Path -LiteralPath $metadata -PathType Leaf) {
        $null = Require-File ("libraries\{0}.metadata.toml" -f $source.BaseName)
    }
}
foreach ($artifact in $libraryArtifacts) {
    $null = Require-File ("libraries\source\{0}.apl" -f $artifact.BaseName)
}
if ($librarySources.Count -ne $libraryArtifacts.Count) {
    throw "APL package library source/artifact counts differ: $($librarySources.Count) sources, $($libraryArtifacts.Count) artifacts."
}

$forbiddenFiles = @(Get-ChildItem -LiteralPath $package -Recurse -File | Where-Object {
    $_.Name -match '(?i)\.(bak[0-9]*|tmp|pdb)$'
})
if ($forbiddenFiles.Count -ne 0) {
    $relative = $forbiddenFiles | ForEach-Object {
        $_.FullName.Substring($package.Length + 1)
    }
    throw "APL package contains temporary files: $($relative -join ', ')"
}

$cli = Join-Path $package "apl.exe"
$example = Join-Path $package "examples\std_main.apl"
Push-Location $package
try {
    $checkOutput = (& $cli check $example 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Packaged APL CLI check failed: $checkOutput"
    }
    if ($checkOutput -notmatch 'ok: portable linked program') {
        throw "Packaged APL CLI returned an unexpected check result: $checkOutput"
    }

    $runOutput = (& $cli run $example 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Packaged APL CLI run failed: $runOutput"
    }
    if ($runOutput -ne "hello world") {
        throw "Packaged APL CLI returned unexpected output: '$runOutput'"
    }
} finally {
    Pop-Location
}

if ($NativeSmoke) {
    $temp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    $smokeDirectory = [System.IO.Path]::GetFullPath(
        (Join-Path $temp ("apl-package-native-smoke-{0}" -f [guid]::NewGuid().ToString("N")))
    )
    $tempPrefix = $temp.TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    ) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $smokeDirectory.StartsWith(
        $tempPrefix,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Refusing to create a native smoke directory outside the system temp directory."
    }

    try {
        Push-Location $package
        try {
            & $cli compile-native $example $smokeDirectory
            if ($LASTEXITCODE -ne 0) {
                throw "Packaged APL CLI failed to compile the native smoke executable."
            }
        } finally {
            Pop-Location
        }

        $nativeExecutable = Join-Path $smokeDirectory "target\debug\std_main_compiled_native.exe"
        if (-not (Test-Path -LiteralPath $nativeExecutable -PathType Leaf)) {
            throw "Native smoke executable was not produced at '$nativeExecutable'."
        }
        $nativeOutput = (& $nativeExecutable 2>&1 | Out-String).Trim()
        if ($LASTEXITCODE -ne 0) {
            throw "Native smoke executable failed with exit code $LASTEXITCODE."
        }
        if ($nativeOutput -ne "hello world") {
            throw "Native smoke executable returned unexpected output: '$nativeOutput'"
        }
    } finally {
        if (Test-Path -LiteralPath $smokeDirectory) {
            Remove-Item -LiteralPath $smokeDirectory -Recurse -Force
        }
    }
}

Write-Host "APL package verification: OK"
Write-Host "Package: $package"
Write-Host "Libraries: $($libraryArtifacts.Count)"
Write-Host "Vendored crates: $($registryPackages.Count)"
Write-Host "Native smoke: $([bool]$NativeSmoke)"
