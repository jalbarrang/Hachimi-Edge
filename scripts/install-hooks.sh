#!/usr/bin/env bash
# =============================================================================
# Install git pre-commit hook that runs quick quality gates before every commit.
# Run once: ./scripts/install-hooks.sh
# =============================================================================

set -euo pipefail

HOOK_DIR="$(git rev-parse --show-toplevel)/.git/hooks"
HOOK_FILE="$HOOK_DIR/pre-commit"

cat > "$HOOK_FILE" << 'HOOK'
#!/usr/bin/env bash
# Pre-commit hook: quick quality gates (fmt + clippy only)
# Installed by scripts/install-hooks.sh
# To skip: git commit --no-verify

set -euo pipefail

echo "🔍 Running pre-commit quality gates..."

# Only check staged Rust files to keep it fast
STAGED_RS=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' || true)

if [[ -z "$STAGED_RS" ]]; then
    echo "  No Rust files staged, skipping checks."
    exit 0
fi

# Format check
echo "  → rustfmt..."
cargo fmt --check 2>/dev/null
if [[ $? -ne 0 ]]; then
    echo "❌ Format check failed. Run: cargo fmt"
    exit 1
fi

# Quick clippy (zero warnings)
echo "  → clippy (zero warnings)..."
cargo clippy --all-targets -- -D warnings 2>&1 | tail -5
if [[ ${PIPESTATUS[0]} -ne 0 ]]; then
    echo "❌ Clippy check failed. Fix warnings before committing."
    exit 1
fi

# Typos (if installed)
if command -v typos &>/dev/null; then
    echo "  → typos..."
    typos $STAGED_RS 2>&1 || {
        echo "❌ Typos found. Fix them before committing."
        exit 1
    }
fi

echo "✅ Pre-commit checks passed."
HOOK

chmod +x "$HOOK_FILE"
echo "✅ Pre-commit hook installed at $HOOK_FILE"
echo ""
echo "Optional: Install the quality tools for full coverage:"
echo "  cargo install typos-cli cargo-deny cargo-machete"
