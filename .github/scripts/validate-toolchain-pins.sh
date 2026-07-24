#!/usr/bin/env bash
# validate-toolchain-pins.sh
#
# Drift guard for the nightly toolchain pin.
# Uses rust-toolchain as the single source of truth for the nightly version
# and soroban_cost_lints/Cargo.toml for the clippy_utils git rev.
#
# Fails if any of the hardcoded copies drift from the canonical source.
#
# Usage: bash .github/scripts/validate-toolchain-pins.sh

set -euo pipefail

# ---- Parse canonical nightly from rust-toolchain (single source of truth) ----
CANONICAL_NIGHTLY=$(sed -n 's/^channel = "\(nightly-[0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\}\)"$/\1/p' rust-toolchain)
if [ -z "$CANONICAL_NIGHTLY" ]; then
  echo "::error file=rust-toolchain::Could not parse [toolchain].channel from rust-toolchain"
  exit 1
fi
echo "Canonical nightly (from rust-toolchain): ${CANONICAL_NIGHTLY}"

NIGHTLY_DATE="${CANONICAL_NIGHTLY#nightly-}"
FAILED=0

# ---- Check every file that hardcodes the nightly string ----
check_nightly_in_file() {
  local file="$1"
  local matches
  matches=$(grep -oE 'nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}' "$file" | sort -u || true)
  if [ -z "$matches" ]; then
    echo "::warning file=$file::No nightly version found in $file (expected ${CANONICAL_NIGHTLY})"
    return
  fi
  while IFS= read -r found; do
    if [ "$found" != "$CANONICAL_NIGHTLY" ]; then
      echo "::error file=$file::Mismatch in ${file}: found '${found}' but expected '${CANONICAL_NIGHTLY}'. Edit ${file} to use the canonical version."
      FAILED=1
    fi
  done <<< "$matches"
  if [ "$FAILED" -eq 0 ]; then
    echo "OK: ${file} matches canonical nightly"
  fi
}

check_nightly_in_file ".github/workflows/lint.yml"
check_nightly_in_file "templates/github-action.yml"
check_nightly_in_file "docs/integration.md"

# ---- Check clippy_utils git rev matches the nightly date ----
CLIPPY_REV=$(sed -n 's/.*rev = "\([a-f0-9]\{40\}\)".*/\1/p' soroban_cost_lints/Cargo.toml)
if [ -z "$CLIPPY_REV" ]; then
  echo "::error file=soroban_cost_lints/Cargo.toml::Could not parse clippy_utils git rev"
  FAILED=1
else
  echo "clippy_utils rev: ${CLIPPY_REV}"

  RESPONSE=$(curl -s "https://api.github.com/repos/rust-lang/rust-clippy/commits/${CLIPPY_REV}")

  # Try jq first, then python3 as fallback
  COMMIT_DATE=""
  if command -v jq &>/dev/null; then
    COMMIT_DATE=$(echo "$RESPONSE" | jq -r '.commit.committer.date' 2>/dev/null | cut -d'T' -f1)
  fi
  if [ -z "$COMMIT_DATE" ] || [ "$COMMIT_DATE" = "null" ]; then
    if command -v python3 &>/dev/null; then
      COMMIT_DATE=$(echo "$RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['commit']['committer']['date'].split('T')[0])" 2>/dev/null)
    fi
  fi

  if [ -z "$COMMIT_DATE" ] || [ "$COMMIT_DATE" = "null" ]; then
    echo "::error file=soroban_cost_lints/Cargo.toml::Invalid or unreachable clippy_utils rev ${CLIPPY_REV}. Update the rev in soroban_cost_lints/Cargo.toml."
    FAILED=1
  else
    COMMIT_TS=$(date -d "$COMMIT_DATE" +%s 2>/dev/null)
    NIGHTLY_TS=$(date -d "$NIGHTLY_DATE" +%s 2>/dev/null)
    if [ -z "$COMMIT_TS" ] || [ -z "$NIGHTLY_TS" ]; then
      echo "::error::Cannot compare dates (date command may not support -d flag)"
      FAILED=1
    else
      DIFF=$((COMMIT_TS - NIGHTLY_TS))
      DIFF_ABS=${DIFF#-}
      DAYS_DIFF=$((DIFF_ABS / 86400))

      if [ "$DAYS_DIFF" -gt 3 ]; then
        echo "::error file=soroban_cost_lints/Cargo.toml::clippy_utils rev ${CLIPPY_REV} is from ${COMMIT_DATE} (${DAYS_DIFF} days away from nightly ${CANONICAL_NIGHTLY}). Update the rev in soroban_cost_lints/Cargo.toml to match the nightly."
        FAILED=1
      else
        echo "OK: clippy_utils rev ${CLIPPY_REV} (${COMMIT_DATE}) is within 3 days of nightly ${NIGHTLY_DATE}"
      fi
    fi
  fi
fi

if [ "$FAILED" -eq 1 ]; then
  exit 1
fi

echo "All toolchain pin checks passed."
