#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$repo = "Poseidoncode/Idlekiller"
$ref = if ($env:IDLEKILLER_REF) { $env:IDLEKILLER_REF } else { "main" }
if ($ref -notmatch '^[A-Za-z0-9._/-]{1,128}$' -or $ref -like '*..*') {
    throw "Invalid IDLEKILLER_REF: '$ref' (allowed: letters, digits, . _ - /)"
}
# /archive/<ref> resolves branches, tags and commit SHAs alike.
# For a reproducible install pin a tag or commit and set IDLEKILLER_SHA256.
$zipUrl = "https://github.com/$repo/archive/$ref.zip"
$tmp = Join-Path $env:TEMP ("Idlekiller-" + [System.Guid]::NewGuid())
$extractDir = Join-Path $tmp "src"

function Test-Command($cmd) {
    $null -ne (Get-Command $cmd -ErrorAction SilentlyContinue)
}

if (-not (Test-Command "cargo")) {
    throw "Rust/Cargo not found. Please install Rust from https://rustup.rs"
}

New-Item -ItemType Directory -Path $tmp -Force | Out-Null

try {
    $zip = Join-Path $tmp "idlekiller.zip"
    Invoke-WebRequest -Uri $zipUrl -OutFile $zip -UseBasicParsing

    if ($env:IDLEKILLER_SHA256) {
        $hash = (Get-FileHash -Path $zip -Algorithm SHA256).Hash.ToLower()
        $expected = $env:IDLEKILLER_SHA256.ToLower()
        if ($hash -ne $expected) {
            throw "Checksum mismatch: expected $expected but got $hash"
        }
    }

    Expand-Archive -Path $zip -DestinationPath $extractDir -Force

    # The top-level folder name depends on the ref, so locate it.
    $dirs = @(Get-ChildItem -Path $extractDir -Directory)
    if ($dirs.Count -ne 1 -or -not (Test-Path (Join-Path $dirs[0].FullName "Cargo.toml"))) {
        throw "Unexpected archive layout for ref '$ref'"
    }
    $sourceDir = $dirs[0].FullName

    Push-Location $sourceDir
    try {
        & cargo build --release --locked
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }

    $installDir = Join-Path $env:USERPROFILE ".local\bin"
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    Copy-Item (Join-Path $sourceDir "target\release\idlekiller.exe") -Destination $installDir -Force

    $resolvedDir = (Resolve-Path $installDir).Path.TrimEnd('\')
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $entries = @(($currentPath -split ';') | Where-Object { $_ } |
        ForEach-Object { $_.TrimEnd('\') })
    if ($entries -notcontains $resolvedDir) {
        $newPath = if ($currentPath) { ($currentPath.TrimEnd(';')) + ";$resolvedDir" } else { $resolvedDir }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "Added $installDir to your user PATH. Restart your terminal to use 'idlekiller'."
    }

    Write-Host "Installed idlekiller.exe to $installDir"
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
