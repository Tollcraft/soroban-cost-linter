#!/usr/bin/env bash
# Tests that .github/workflows/lint.yml places fmt and clippy in a dedicated
# single-platform job and that the build-and-test job does NOT duplicate them.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
WORKFLOW="$REPO_ROOT/.github/workflows/lint.yml"

pass() { echo "PASS: $1"; }
fail() { echo "FAIL: $1" >&2; exit 1; }

# --- A dedicated lint job must exist ---

if grep -q 'lint-checks:' "$WORKFLOW"; then
    pass "lint-checks job present"
else
    fail "lint.yml must define a 'lint-checks' job for fmt and clippy"
fi

# --- build-and-test must declare a dependency on lint-checks ---

if grep -A 5 'build-and-test:' "$WORKFLOW" | grep -q 'needs:.*lint-checks\|needs: lint-checks'; then
    pass "build-and-test needs lint-checks"
else
    fail "build-and-test must declare 'needs: lint-checks'"
fi

# --- build-and-test must NOT contain cargo fmt ---

BUILD_TEST_SECTION=$(awk '/^  build-and-test:/,0' "$WORKFLOW")
if echo "$BUILD_TEST_SECTION" | grep -q 'cargo fmt'; then
    fail "build-and-test must NOT run cargo fmt (it belongs in lint-checks)"
else
    pass "build-and-test does not duplicate cargo fmt"
fi

# --- build-and-test must NOT contain cargo clippy ---

if echo "$BUILD_TEST_SECTION" | grep -q 'cargo clippy'; then
    fail "build-and-test must NOT run cargo clippy (it belongs in lint-checks)"
else
    pass "build-and-test does not duplicate cargo clippy"
fi

# --- lint-checks must run on ubuntu-latest (verify by scanning for the
#     runs-on line near the lint-checks job header) ---

if grep -A 3 'lint-checks:' "$WORKFLOW" | grep -q 'ubuntu-latest'; then
    pass "lint-checks runs on ubuntu-latest"
else
    fail "lint-checks must run on ubuntu-latest (OS-independent check)"
fi

echo "All ci_lint_dedup checks passed."
