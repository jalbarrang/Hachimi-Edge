<#
.SYNOPSIS
    Copy release-built Hachimi core and training-tracker plugin into the game directory.

.DESCRIPTION
    - Copies target\release\hachimi.dll → <GameDir>\cri_mana_vpx.dll (proxy)
    - Copies target\release\hachimi_training_tracker.dll → <GameDir>\
    - Never modifies cri_mana_vpx.dll.backup

.PARAMETER GameDir
    Uma Musume install folder (contains the game exe and cri_mana_vpx.dll).
    Defaults to $env:HACHIMI_GAME_DIR or the standard Steam path.

.PARAMETER Build
    Run `cargo build --release` for hachimi and hachimi-training-tracker before copying.

.EXAMPLE
    .\scripts\deploy-windows.ps1
    .\scripts\deploy-windows.ps1 -Build
    .\scripts\deploy-windows.ps1 -GameDir "D:\Games\UmamusumePrettyDerby"

.EXAMPLE
    $env:HACHIMI_GAME_DIR = "C:\Program Files (x86)\Steam\steamapps\common\UmamusumePrettyDerby"
    .\scripts\deploy-windows.ps1 -Build
#>

param(
    [string]$GameDir = $(if ($env:HACHIMI_GAME_DIR) { $env:HACHIMI_GAME_DIR } else {
        "${env:ProgramFiles(x86)}\Steam\steamapps\common\UmamusumePrettyDerby"
    }),
    [switch]$Build
)

$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$TargetDir = Join-Path $RepoRoot "target\release"
$HostDll = Join-Path $TargetDir "hachimi.dll"
$PluginDll = Join-Path $TargetDir "hachimi_training_tracker.dll"
$ProxyName = "cri_mana_vpx.dll"
$BackupName = "cri_mana_vpx.dll.backup"

function Require-File {
    param([string]$Path, [string]$Hint)
    if (-not (Test-Path -LiteralPath $Path)) {
        Write-Error "Missing: $Path`n$Hint"
    }
}

if ($Build) {
    Write-Host "Building release artifacts..." -ForegroundColor Cyan
    Push-Location $RepoRoot
    try {
        cargo build --release -p hachimi
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        cargo build --release -p hachimi-training-tracker
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    finally {
        Pop-Location
    }
}

Require-File $HostDll "Run: cargo build --release -p hachimi`nOr pass -Build"
Require-File $PluginDll "Run: cargo build --release -p hachimi-training-tracker`nOr pass -Build"

$GameDir = $GameDir.TrimEnd('\')
if (-not (Test-Path -LiteralPath $GameDir -PathType Container)) {
    Write-Error @"
Game directory not found: $GameDir

Set -GameDir or env:HACHIMI_GAME_DIR to your UmamusumePrettyDerby folder.
"@
}

$ProxyPath = Join-Path $GameDir $ProxyName
$BackupPath = Join-Path $GameDir $BackupName
$PluginDest = Join-Path $GameDir "hachimi_training_tracker.dll"

if (-not (Test-Path -LiteralPath $BackupPath)) {
    Write-Warning @"
$BackupName not found in the game folder.

Before the first proxy install, back up the stock DLL, e.g.:
  Copy-Item -LiteralPath '$ProxyPath' -Destination '$BackupPath'
(Only if '$ProxyName' is still the original game file.)

This script will NOT create or modify $BackupName.
"@
}

Write-Host ""
Write-Host "Deploying to: $GameDir" -ForegroundColor Green
Write-Host ""

Copy-Item -LiteralPath $HostDll -Destination $ProxyPath -Force
Write-Host "  hachimi.dll  ->  $ProxyName"

Copy-Item -LiteralPath $PluginDll -Destination $PluginDest -Force
Write-Host "  hachimi_training_tracker.dll  ->  hachimi_training_tracker.dll"

# Skill-evaluation resource (read at runtime by the training-tracker eval engine).
$SkillGradesSrc = Join-Path $PSScriptRoot "..\plugins\training-tracker\assets\skill_grades.json"
if (Test-Path -LiteralPath $SkillGradesSrc) {
  Copy-Item -LiteralPath $SkillGradesSrc -Destination (Join-Path $GameDir "skill_grades.json") -Force
  Write-Host "  skill_grades.json  ->  skill_grades.json"
} else {
  Write-Host "  (skill_grades.json missing; run scripts/gen-skill-grades.mjs)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Done. Ensure config.json lists the plugin under windows.load_libraries:" -ForegroundColor Cyan
Write-Host '  "load_libraries": ["hachimi_training_tracker.dll"]'
Write-Host ""
Write-Host "Launch the game yourself to verify (this script does not start the game)." -ForegroundColor DarkGray
