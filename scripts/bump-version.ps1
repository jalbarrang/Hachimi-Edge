#!/usr/bin/env pwsh
# =============================================================================
# Bump the release version from conventional-commit history.
#
# Computes the next semver from commits since the last `v*` tag (via git-cliff,
# standard semver — see cliff.toml) and writes it into apps/hachimi/Cargo.toml,
# refreshing Cargo.lock at the same time (via `cargo set-version`).
#
# This only edits files; it does NOT commit, tag, or push. After running:
#   1. review the diff, commit (e.g. `chore(release): vX.Y.Z`), push to main
#   2. trigger the "Create Release" workflow (workflow_dispatch) on GitHub
#
# Prerequisites (one-time):
#   cargo install git-cliff
#   cargo install cargo-edit
#
# Usage:
#   ./scripts/bump-version.ps1
# =============================================================================

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Crate = 'hachimi'
$Manifest = 'apps/hachimi/Cargo.toml'

# --- Resolve repo root so the script works from anywhere -----------------------
$RepoRoot = (git rev-parse --show-toplevel).Trim()
Set-Location $RepoRoot

# --- Preflight: required tools -------------------------------------------------
$missing = $false
if (-not (Get-Command git-cliff -ErrorAction SilentlyContinue)) {
    Write-Host "❌ git-cliff not found. Install it with: cargo install git-cliff"
    $missing = $true
}
cargo set-version --help *> $null
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 'cargo set-version' not found. Install it with: cargo install cargo-edit"
    $missing = $true
}
if ($missing) {
    exit 1
}

# --- Current version -----------------------------------------------------------
$current = $null
$inPackage = $false
foreach ($line in Get-Content $Manifest) {
    if ($line -match '^\[package\]') { $inPackage = $true; continue }
    if ($inPackage -and $line -match '^\s*\[') { break }
    if ($inPackage -and $line -match '^\s*version\s*=\s*"([^"]+)"') {
        $current = $Matches[1]
        break
    }
}
$currentDisplay = if ($current) { $current } else { 'unknown' }
Write-Host "Current version: $currentDisplay"

# --- Compute the next version from conventional commits ------------------------
# `--bumped-version` prints the next version (with a leading 'v').
# It can fail/return empty when every unreleased commit is skipped by the
# commit_parsers (git-cliff #816) — treat that as "no bump warranted".
$newRaw = git cliff --bumped-version 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "ℹ️  git-cliff found no version bump warranted (no qualifying commits). Nothing to do."
    exit 0
}

$newVersion = ($newRaw | Out-String).Trim() -replace '^v', ''

if ([string]::IsNullOrEmpty($newVersion)) {
    Write-Host "ℹ️  No version bump warranted (no qualifying commits since last tag). Nothing to do."
    exit 0
}

if ($newVersion -eq $current) {
    Write-Host "ℹ️  Computed version ($newVersion) matches current. Nothing to do."
    exit 0
}

# --- Apply (edits Cargo.toml AND updates Cargo.lock) ---------------------------
Write-Host "Bumping ${Crate}: ${current} -> ${newVersion}"
cargo set-version -p $Crate $newVersion
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 'cargo set-version' failed."
    exit 1
}

Write-Host ""
Write-Host "✅ Bumped to ${newVersion} (apps/hachimi/Cargo.toml + Cargo.lock)."
Write-Host "   Next: review the diff, commit as 'chore(release): v${newVersion}',"
Write-Host "   push to main, then run the 'Create Release' workflow on GitHub."
