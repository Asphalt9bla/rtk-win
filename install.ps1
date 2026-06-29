# rtk-win installer - https://github.com/rtk-ai/rtk
# Usage: powershell -c "irm https://raw.githubusercontent.com/rtk-ai/rtk/master/install.ps1 | iex"

param(
    [string]$InstallDir = "$env:USERPROFILE\.cargo\bin",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$BinaryName = "rtk.exe"

function Write-Info($msg) { Write-Host "[INFO] $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Write-Error($msg) { Write-Host "[ERROR] $msg" -ForegroundColor Red; exit 1 }

# Check for cargo
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Error "cargo not found. Install Rust from https://rustup.rs"
}

Write-Info "Installing rtk to: $InstallDir"

if (-not $SkipBuild) {
    $repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
    if (-not $repoRoot) { $repoRoot = Get-Location }

    $binaryPath = Join-Path $repoRoot "target\release\$BinaryName"
    $needsBuild = $true

    if (Test-Path $binaryPath) {
        $binaryTime = (Get-Item $binaryPath).LastWriteTime
        $sourceTime = (Get-Item (Join-Path $repoRoot "Cargo.toml")).LastWriteTime
        $lockTime = (Get-Item (Join-Path $repoRoot "Cargo.lock")).LastWriteTime
        if ($binaryTime -gt $sourceTime -and $binaryTime -gt $lockTime) {
            $needsBuild = $false
        }
    }

    if ($needsBuild) {
        Write-Info "Building rtk (release)..."
        Push-Location $repoRoot
        try {
            cargo build --release
            if ($LASTEXITCODE -ne 0) { Write-Error "Build failed" }
        } finally {
            Pop-Location
        }
    } else {
        Write-Info "Binary is up to date"
    }
}

# Ensure install directory exists
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

# Copy binary
$source = if (-not $SkipBuild) {
    Join-Path $repoRoot "target\release\$BinaryName"
} else {
    # When SkipBuild is set, expect binary in current directory or download it
    ".\$BinaryName"
}

if (-not (Test-Path $source)) {
    Write-Error "Binary not found at: $source"
}

Copy-Item -Path $source -Destination (Join-Path $InstallDir $BinaryName) -Force
Write-Info "Installed: $InstallDir\$BinaryName"

# Add to PATH if not already present
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$InstallDir*") {
    $newPath = "$InstallDir;$userPath"
    [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
    Write-Info "Added $InstallDir to user PATH"
    Write-Warn "Restart your terminal for PATH changes to take effect"
} else {
    Write-Info "$InstallDir is already in PATH"
}

Write-Info "Installation complete! Run 'rtk --help' to get started."
