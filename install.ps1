#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$repo = "Poseidoncode/Idlekiller"
$ref = if ($env:IDLEKILLER_REF) { $env:IDLEKILLER_REF } else { "main" }
$zipUrl = "https://github.com/$repo/archive/refs/heads/$ref.zip"
$tmp = Join-Path $env:TEMP ("Idlekiller-" + [System.Guid]::NewGuid())
$sourceDir = Join-Path $tmp "Idlekiller-$ref"

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

    Expand-Archive -Path $zip -DestinationPath $tmp -Force

    Push-Location $sourceDir
    try {
        & cargo build --release
    } finally {
        Pop-Location
    }

    $installDir = Join-Path $env:USERPROFILE ".local\bin"
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    Copy-Item (Join-Path $sourceDir "target\release\idlekiller.exe") -Destination $installDir -Force

    $resolvedDir = (Resolve-Path $installDir).Path.TrimEnd('\')
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $normalizedPath = ($currentPath -replace '\\+$', '').TrimEnd('\')
    if ($normalizedPath -notlike "*$resolvedDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$currentPath;$installDir", "User")
        Write-Host "Added $installDir to your user PATH. Restart your terminal to use 'idlekiller'."
    }

    Write-Host "Installed idlekiller.exe to $installDir"
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
