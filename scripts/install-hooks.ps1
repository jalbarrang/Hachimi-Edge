<#
.SYNOPSIS
    Install git pre-commit hook for Windows development.
    Run once: .\scripts\install-hooks.ps1
#>

$hookDir = git rev-parse --show-toplevel | ForEach-Object { Join-Path $_ ".git/hooks" }
$hookFile = Join-Path $hookDir "pre-commit"

$hookContent = @'
#!/usr/bin/env bash
# Pre-commit hook: quick quality gates (fmt + clippy only)
# Installed by scripts/install-hooks.ps1
# To skip: git commit --no-verify

set -euo pipefail

echo "🔍 Running pre-commit quality gates..."

STAGED_RS=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' || true)

if [[ -z "$STAGED_RS" ]]; then
    echo "  No Rust files staged, skipping checks."
    exit 0
fi

echo "  → rustfmt..."
cargo fmt --check 2>/dev/null
if [[ $? -ne 0 ]]; then
    echo "❌ Format check failed. Run: cargo fmt"
    exit 1
fi

echo "  → clippy (zero warnings)..."
cargo clippy --all-targets -- -D warnings 2>&1 | tail -5
if [[ ${PIPESTATUS[0]} -ne 0 ]]; then
    echo "❌ Clippy check failed. Fix warnings before committing."
    exit 1
fi

if command -v typos &>/dev/null; then
    echo "  → typos..."
    typos $STAGED_RS 2>&1 || {
        echo "❌ Typos found. Fix them before committing."
        exit 1
    }
fi

echo "✅ Pre-commit checks passed."
'@

Set-Content -Path $hookFile -Value $hookContent -Encoding UTF8 -NoNewline
Write-Host "✅ Pre-commit hook installed at $hookFile" -ForegroundColor Green
Write-Host ""
Write-Host "Optional: Install the quality tools for full coverage:" -ForegroundColor Cyan
Write-Host "  cargo install typos-cli cargo-deny cargo-machete" -ForegroundColor White
