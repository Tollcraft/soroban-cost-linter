#!/usr/bin/env bash
# Tests that .github/workflows/lint.yml defines paths-ignore filters so
# documentation-only pull requests bypass the heavy compiler matrix and
# benchmark runs.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
WORKFLOW="$REPO_ROOT/.github/workflows/lint.yml"

pass() { echo "PASS: $1"; }
fail() { echo "FAIL: $1" >&2; exit 1; }

# --- paths-ignore must be present on the pull_request trigger ---

if grep -q 'paths-ignore' "$WORKFLOW"; then
    pass "paths-ignore present in lint.yml"
else
    fail "lint.yml must define paths-ignore under the pull_request trigger"
fi

# --- Markdown files must be ignored ---

if grep -A 20 'paths-ignore' "$WORKFLOW" | grep -q "'\\*\\*\\.md'\\|\"\\*\\*\\.md\""; then
    pass "'**.md' in paths-ignore"
else
    fail "lint.yml paths-ignore must include '**.md' to skip docs-only PRs"
fi

# --- docs/ directory must be ignored ---

if grep -A 20 'paths-ignore' "$WORKFLOW" | grep -q "docs/"; then
    pass "'docs/**' in paths-ignore"
else
    fail "lint.yml paths-ignore must include 'docs/**'"
fi

echo "All docs_only_paths checks passed."
