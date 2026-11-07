$ErrorActionPreference = "Stop"

$root = [System.IO.Path]::GetFullPath(
    (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))
)

function Invoke-Git([string[]]$Arguments) {
    $output = & git -C $root @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Git command failed: git $($Arguments -join ' ')"
    }
    return @($output)
}

$requiredFiles = @(
    "README.md",
    "APL_CONTEXT.md",
    "docs\README.md",
    "docs\status.md",
    "docs\language.md",
    "docs\architecture.md",
    "docs\roadmap.md",
    "docs\installation.md",
    "tools\build_release.ps1",
    "tools\verify_package.ps1",
    "tests\regressions\open\README.md"
)
foreach ($relativePath in $requiredFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relativePath) -PathType Leaf)) {
        throw "Repository is missing '$relativePath'."
    }
}

$trackedFiles = Invoke-Git @("ls-files")
$generatedPattern = '(?i)(^|/)(target|build|dist|\.verification-target)/|\.(exe|dll|pdb|zip|sha256|aplmod|bak[0-9]*|tmp)$'
$trackedGenerated = @($trackedFiles | Where-Object { $_ -match $generatedPattern })
if ($trackedGenerated.Count -ne 0) {
    throw "Generated files are tracked by Git: $($trackedGenerated -join ', ')"
}

$sourceRoots = @(
    "crates",
    "std",
    "libraries",
    "examples",
    "tests",
    "tools",
    "docs",
    "installer"
)
$backupFiles = @()
foreach ($sourceRoot in $sourceRoots) {
    $directory = Join-Path $root $sourceRoot
    if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
        continue
    }
    $backupFiles += Get-ChildItem -LiteralPath $directory -Recurse -File | Where-Object {
        $_.Name -match '(?i)\.(bak[0-9]*|tmp|old)$'
    }
}
if ($backupFiles.Count -ne 0) {
    $relativeBackups = $backupFiles | ForEach-Object {
        $_.FullName.Substring($root.Length + 1)
    }
    throw "Source tree contains backup files: $($relativeBackups -join ', ')"
}

$markdownFiles = Invoke-Git @("ls-files", "--", "*.md")
$brokenLinks = @()
foreach ($markdownRelativePath in $markdownFiles) {
    $markdownPath = Join-Path $root $markdownRelativePath
    $markdown = Get-Content -LiteralPath $markdownPath -Raw -Encoding UTF8
    $links = [regex]::Matches($markdown, '\[[^\]]*\]\(([^)]+)\)')
    foreach ($link in $links) {
        $target = $link.Groups[1].Value.Trim()
        if ($target.StartsWith('<') -and $target.EndsWith('>')) {
            $target = $target.Substring(1, $target.Length - 2)
        }
        if ($target -match '^(?i)(https?://|mailto:|#)') {
            continue
        }
        $pathPart = ($target -split '#', 2)[0]
        if ([string]::IsNullOrWhiteSpace($pathPart)) {
            continue
        }
        $decodedPath = [uri]::UnescapeDataString($pathPart).Replace(
            '/',
            [System.IO.Path]::DirectorySeparatorChar
        )
        $resolvedTarget = [System.IO.Path]::GetFullPath(
            (Join-Path (Split-Path -Parent $markdownPath) $decodedPath)
        )
        if (-not (Test-Path -LiteralPath $resolvedTarget)) {
            $brokenLinks += "$markdownRelativePath -> $target"
        }
    }
}
if ($brokenLinks.Count -ne 0) {
    throw "Broken local Markdown links: $($brokenLinks -join ', ')"
}

$libraryDirectory = Join-Path $root "libraries"
$librarySources = @(Get-ChildItem -LiteralPath $libraryDirectory -Filter "*.apl" -File)
foreach ($metadataPath in Get-ChildItem -LiteralPath $libraryDirectory -Filter "*.metadata.toml" -File) {
    $metadata = Get-Content -LiteralPath $metadataPath.FullName -Raw
    $dependencyMatch = [regex]::Match($metadata, '(?ms)^dependencies\s*=\s*\[(.*?)\]')
    if (-not $dependencyMatch.Success) {
        throw "Library metadata '$($metadataPath.Name)' has no dependencies array."
    }
    foreach ($dependency in [regex]::Matches($dependencyMatch.Groups[1].Value, '"([^"]+)"')) {
        $dependencyName = $dependency.Groups[1].Value
        if (-not (Test-Path -LiteralPath (Join-Path $libraryDirectory "$dependencyName.apl") -PathType Leaf)) {
            throw "Library '$($metadataPath.BaseName)' depends on missing '$dependencyName'."
        }
    }
}

$openRegressionDirectory = Join-Path $root "tests\regressions\open"
$openRegressionReadme = Get-Content -LiteralPath (Join-Path $openRegressionDirectory "README.md") -Raw
$openRegressions = @(Get-ChildItem -LiteralPath $openRegressionDirectory -Filter "*.apl" -File)
foreach ($regression in $openRegressions) {
    if (-not $openRegressionReadme.Contains("``$($regression.Name)``")) {
        throw "Open regression '$($regression.Name)' is not listed in its README."
    }
}
$listedRegressions = [regex]::Matches($openRegressionReadme, '`([^`]+\.apl)`')
foreach ($listedRegression in $listedRegressions) {
    $listedName = $listedRegression.Groups[1].Value
    if (-not (Test-Path -LiteralPath (Join-Path $openRegressionDirectory $listedName) -PathType Leaf)) {
        throw "Open regression README lists missing '$listedName'."
    }
}

$ignoreFile = Get-Content -LiteralPath (Join-Path $root ".gitignore") -Raw
foreach ($ignoreEntry in @("/target/", "/build/", "/dist/", "*.bak*", "/libraries/*.aplmod")) {
    if (-not $ignoreFile.Contains($ignoreEntry)) {
        throw "Repository .gitignore is missing '$ignoreEntry'."
    }
}

Write-Host "APL repository verification: OK"
Write-Host "Tracked files: $($trackedFiles.Count)"
Write-Host "Markdown files: $($markdownFiles.Count)"
Write-Host "Library sources: $($librarySources.Count)"
Write-Host "Open regressions: $($openRegressions.Count)"
