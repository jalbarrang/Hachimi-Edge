#!/usr/bin/env bash
# =============================================================================
# Local quality gate script (bash) — mirrors CI pipeline.
# Run before pushing to catch issues locally.
#
# Usage:
#   ./scripts/quality-gates.sh          # full check
#   ./scripts/quality-gates.sh --quick   # fmt + clippy only
# =============================================================================

set -euo pipefail

QUICK=false
[[ "${1:-}" == "--quick" ]] && QUICK=true

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
NC='\033[0m'

FAILED=()

run_gate() {
    local name="$1"
    shift
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  $name${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    if "$@"; then
        echo -e "  ${GREEN}✓ PASSED: $name${NC}"
    else
        echo -e "  ${RED}✗ FAILED: $name${NC}"
        FAILED+=("$name")
    fi
}

START=$(date +%s)

# Gate 1: Formatting
run_gate "Rustfmt (core)" cargo fmt --check
run_gate "Rustfmt (plugin)" bash -c "cd plugins/training-tracker && cargo fmt --check"

# Gate 2: Typos
if [[ "$QUICK" == false ]] && command -v typos &>/dev/null; then
    run_gate "Typos" typos
elif [[ "$QUICK" == false ]]; then
    echo -e "\n  ${YELLOW}⚠ SKIP: typos not installed (cargo install typos-cli)${NC}"
fi

# Gate 3: Supply chain
if [[ "$QUICK" == false ]] && command -v cargo-deny &>/dev/null; then
    run_gate "cargo-deny" cargo deny check
elif [[ "$QUICK" == false ]]; then
    echo -e "\n  ${YELLOW}⚠ SKIP: cargo-deny not installed (cargo install cargo-deny)${NC}"
fi

# Gate 4: Unused deps
if [[ "$QUICK" == false ]] && command -v cargo-machete &>/dev/null; then
    run_gate "cargo-machete (core)" cargo machete
    run_gate "cargo-machete (plugin)" bash -c "cd plugins/training-tracker && cargo machete"
elif [[ "$QUICK" == false ]]; then
    echo -e "\n  ${YELLOW}⚠ SKIP: cargo-machete not installed (cargo install cargo-machete)${NC}"
fi

# Gate 5: Clippy (zero warnings)
run_gate "Clippy (core)" cargo clippy --all-targets -- -D warnings
run_gate "Clippy (plugin)" bash -c "cd plugins/training-tracker && cargo clippy --all-targets -- -D warnings"

# Gate 6: Type check
if [[ "$QUICK" == false ]]; then
    run_gate "Cargo check (core)" cargo check --all-targets
    run_gate "Cargo check (plugin)" bash -c "cd plugins/training-tracker && cargo check --all-targets"
fi

# Summary
END=$(date +%s)
ELAPSED=$((END - START))
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Quality Gates Complete (${ELAPSED}s)${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

if [[ ${#FAILED[@]} -gt 0 ]]; then
    echo ""
    echo -e "  ${RED}✗ ${#FAILED[@]} gate(s) FAILED:${NC}"
    for f in "${FAILED[@]}"; do
        echo -e "    ${RED}- $f${NC}"
    done
    echo ""
    exit 1
else
    echo ""
    echo -e "  ${GREEN}✓ All gates passed — safe to push.${NC}"
    echo ""
    exit 0
fi
