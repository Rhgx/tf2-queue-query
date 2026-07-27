param(
    [string]$Version = "0.1.0",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$DistDirectory = Join-Path $ProjectRoot "dist"
$Executable = Join-Path $ProjectRoot "target\release\tf2-queue-query.exe"
$PackageName = "tf2-queue-query-v$Version-windows-x86_64"
$StageDirectory = Join-Path $DistDirectory $PackageName
$Archive = Join-Path $DistDirectory "$PackageName.zip"
$Checksum = "$Archive.sha256"

$Metadata = cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $ProjectRoot "Cargo.toml") | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed"
}
$ManifestVersion = $Metadata.packages[0].version
if ($Version -ne $ManifestVersion) {
    throw "Requested package version $Version does not match Cargo.toml version $ManifestVersion"
}

if (-not $SkipBuild) {
    cargo build --release --locked --manifest-path (Join-Path $ProjectRoot "Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed"
    }
}

if (-not (Test-Path -LiteralPath $Executable -PathType Leaf)) {
    throw "Release executable was not found at $Executable"
}

New-Item -ItemType Directory -Force -Path $DistDirectory | Out-Null
if (Test-Path -LiteralPath $StageDirectory) {
    Remove-Item -LiteralPath $StageDirectory -Recurse -Force
}
New-Item -ItemType Directory -Path $StageDirectory | Out-Null

Copy-Item -LiteralPath $Executable -Destination $StageDirectory
Copy-Item -LiteralPath (Join-Path $ProjectRoot "README.md") -Destination $StageDirectory
Copy-Item -LiteralPath (Join-Path $ProjectRoot "LICENSE") -Destination $StageDirectory
Copy-Item -LiteralPath (Join-Path $ProjectRoot "CHANGELOG.md") -Destination $StageDirectory

if (Test-Path -LiteralPath $Archive) {
    Remove-Item -LiteralPath $Archive -Force
}

Add-Type -AssemblyName System.IO.Compression
$ArchiveStream = [IO.File]::Open($Archive, [IO.FileMode]::CreateNew)
$Zip = [IO.Compression.ZipArchive]::new(
    $ArchiveStream,
    [IO.Compression.ZipArchiveMode]::Create,
    $false
)
try {
    Get-ChildItem -LiteralPath $StageDirectory -File |
        Sort-Object -Property Name |
        ForEach-Object {
            $EntryName = "$PackageName/$($_.Name)"
            $Entry = $Zip.CreateEntry(
                $EntryName,
                [IO.Compression.CompressionLevel]::Optimal
            )
            $InputStream = [IO.File]::OpenRead($_.FullName)
            $EntryStream = $Entry.Open()
            try {
                $InputStream.CopyTo($EntryStream)
            } finally {
                $EntryStream.Dispose()
                $InputStream.Dispose()
            }
        }
} finally {
    $Zip.Dispose()
    $ArchiveStream.Dispose()
}
Remove-Item -LiteralPath $StageDirectory -Recurse -Force

$Hash = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$Hash  $([IO.Path]::GetFileName($Archive))" | Out-File -LiteralPath $Checksum -Encoding ascii

Write-Output $Archive
Write-Output $Checksum
