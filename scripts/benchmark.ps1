# rtk-win Real-World Benchmark (Windows/PowerShell)
# Measures token savings on large-output commands
# Usage: .\scripts\benchmark.ps1 [-ProjectPath <dir>]

param([string]$ProjectPath = (Get-Location).Path)

$ErrorActionPreference = "Stop"
$results = @()

function Write-Step($msg) { Write-Host "[*] $msg" -ForegroundColor Cyan }

function Measure-Savings($name, [scriptblock]$rawBlock, [string[]]$rtkArgs) {
    # warmup
    & $rawBlock 2>$null | Out-Null
    & rtk @rtkArgs 2>$null | Out-Null

    $raw = & $rawBlock 2>&1 | Out-String
    $filtered = & rtk @rtkArgs 2>&1 | Out-String

    $rawLen = $raw.Length
    $filteredLen = $filtered.Length
    $savings = if ($rawLen -gt 0) { [math]::Round(($rawLen - $filteredLen) / $rawLen * 100, 1) } else { 0 }

    $script:results += [PSCustomObject]@{Name=$name; Raw=$rawLen; Rtk=$filteredLen; Savings=$savings}

    $color = if ($savings -ge 70) {"Green"} elseif ($savings -ge 30) {"Yellow"} else {"Red"}
    $savingsStr = if ($savings -ge 0) { " $savings%" } else { "  x" }
    Write-Host "    $("$name".PadRight(24)) $($rawLen.ToString('N0').PadLeft(8)) -> $($filteredLen.ToString('N0').PadLeft(8))  $savingsStr" -ForegroundColor $color
}

function Try-Measure($name, [scriptblock]$rawBlock, [string[]]$rtkArgs) {
    try { Measure-Savings $name $rawBlock $rtkArgs }
    catch { Write-Host "    $("$name".PadRight(24)) [SKIP] $_" -ForegroundColor DarkGray }
}

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "  rtk-win Real-World Benchmark (Windows)" -ForegroundColor Cyan
Write-Host "  Project: $ProjectPath" -ForegroundColor Cyan
Write-Host "  $(Get-Date -Format 'yyyy-MM-dd HH:mm')" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan
Write-Host ""

# ── 1. ls: native RTK vs raw dir ──
Write-Step "1. ls — Rust-native vs PowerShell Get-ChildItem"

Try-Measure "ls (project src)" `
    { Get-ChildItem $ProjectPath\src -Recurse -File | Select-Object Name, Length, LastWriteTime | Out-String } `
    @('ls', $ProjectPath + '\src')

Try-Measure "ls (drivers large)" `
    { Get-ChildItem C:\Windows\System32\drivers -Recurse -File | Select-Object Name, Length | Out-String } `
    @('ls', 'C:\Windows\System32\drivers')

Try-Measure "ls (drivers wc)" `
    { Get-ChildItem C:\Windows\System32\drivers\etc\* | Select-Object Name | Out-String } `
    @('ls', 'C:\Windows\System32\drivers\etc')

Write-Host ""

# ── 2. wc: files count ──
Write-Step "2. wc — Rust-native vs PowerShell"

Try-Measure "wc (src .rs files)" `
    { Get-ChildItem $ProjectPath\src\*.rs -Recurse | Select-Object Length | Out-String } `
    @('wc', $ProjectPath + '\src\*.rs')

Try-Measure "wc (drivers folder)" `
    { Get-ChildItem C:\Windows\System32\drivers\etc\* | Select-Object Length | Out-String } `
    @('wc', 'C:\Windows\System32\drivers\etc\*')

Write-Host ""

# ── 3. find: search ──
Write-Step "3. find — Rust-native vs PowerShell"

Try-Measure "find (src .rs)" `
    { Get-ChildItem $ProjectPath -Recurse -Filter *.rs -Name | Out-String } `
    @('find', $ProjectPath, '-name', '*.rs')

Try-Measure "find (src .toml)" `
    { Get-ChildItem $ProjectPath -Recurse -Filter *.toml -Name | Out-String } `
    @('find', $ProjectPath, '-name', '*.toml')

Write-Host ""

# ── 4. tree ──
Write-Step "4. tree — Rust-native vs cmd tree /F"

Try-Measure "tree (src dirs)" `
    { cmd /c "tree $ProjectPath\src /F 2>nul" | Out-String } `
    @('tree', $ProjectPath + '\src')

Write-Host ""

# ── 5. Git operations ──
if (Get-Command git -ErrorAction SilentlyContinue) {
    Write-Step "5. Git — raw vs RTK-filtered"

    Try-Measure "git status" `
        { git -C $ProjectPath status 2>&1 | Out-String } `
        @('git', '-C', $ProjectPath, 'status')

    Try-Measure "git log (all)" `
        { git -C $ProjectPath log --oneline --all 2>&1 | Out-String } `
        @('git', '-C', $ProjectPath, 'log', '--oneline', '--all')

    Try-Measure "git diff (HEAD~1)" `
        { git -C $ProjectPath diff HEAD~1..HEAD --stat 2>&1 | Out-String } `
        @('git', '-C', $ProjectPath, 'diff', 'HEAD~1..HEAD', '--stat')
    Write-Host ""
}

# ── 6. Package managers ──
Write-Step "6. Package managers — raw vs RTK-filtered"

if (Get-Command winget -ErrorAction SilentlyContinue) {
    Try-Measure "winget list" `
        { winget list --accept-source-agreements 2>$null | Out-String } `
        @('winget', 'list')
}

if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Try-Measure "cargo tree" `
        { cargo tree --manifest-path "$ProjectPath\Cargo.toml" 2>&1 | Out-String } `
        @('cargo', 'tree', '--manifest-path', "$ProjectPath\Cargo.toml")
}
Write-Host ""

# ── 7. Passthrough (no filter) ──
Write-Step "7. Passthrough — raw vs RTK (TOML-filtered)"

Try-Measure "systeminfo" `
    { systeminfo 2>$null | Out-String } `
    @('systeminfo')

Try-Measure "env vars" `
    { Get-ChildItem Env: | Out-String } `
    @('cmd', '/c', 'set')

Write-Host ""

# ── Summary ──
Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "  RESULTS" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan

$totalRaw = ($results | Measure-Object Raw -Sum).Sum
$totalRtk = ($results | Measure-Object Rtk -Sum).Sum
$overall = if ($totalRaw -gt 0) { [math]::Round(($totalRaw - $totalRtk) / $totalRaw * 100, 1) } else { 0 }

$results | Sort-Object Savings -Descending | Format-Table @{L='Command';E={$_.Name};A='Left';W=24},
    @{L='Raw';E={$_.Raw.ToString('N0')};A='Right';W=10},
    @{L='RTK';E={$_.Rtk.ToString('N0')};A='Right';W=10},
    @{L='Saved';E={if ($_.Savings -ge 0) {"$($_.Savings)%"} else {"(overhead)"}};A='Right';W=14} -AutoSize

Write-Host ""
Write-Host "  OVERALL: $($totalRaw.ToString('N0')) chars -> $($totalRtk.ToString('N0')) chars  |  $overall% saved" -ForegroundColor White
Write-Host ""
Write-Host "  Key: Green >=70%, Yellow 30-69%, Red <30% or overhead" -ForegroundColor Gray
Write-Host "  (overhead) means RTK adds more chars than it removes" -ForegroundColor Gray
Write-Host ""
