#!/usr/bin/env bash
# bump-nightly.sh
#
# Move this crate to a newer Rust nightly. Updates the toolchain pin and the
# matching clippy_utils rev across every file that must stay in sync (the set
# enforced by .github/scripts/validate-toolchain-pins.sh). It edits the working
# tree only; it does NOT commit. The calling workflow decides what to do next.
#
# Usage:
#   bump-nightly.sh [TARGET_NIGHTLY]
#
#   TARGET_NIGHTLY   e.g. "nightly-2026-05-15". When omitted, the target is
#                    derived as (today - OFFSET_DAYS) so each scheduled run aims
#                    at a nightly that is ~1-2 weeks old.
#
# Environment:
#   OFFSET_DAYS      days before today to target (default: 7)
#   CLIPPY_REV       override the resolved clippy_utils rev
#   GITHUB_TOKEN     optional; raises the GitHub API rate limit for rev lookup
#
# Exit codes:
#   0  pins were updated
#   2  target already equals the current pin (nothing to do)
#   1  a resolution or edit step failed

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

CURRENT="$(sed -n 's/^channel = "\(nightly-[0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\}\)"$/\1/p' rust-toolchain)"
if [ -z "$CURRENT" ]; then
  echo "::error file=rust-toolchain::could not parse [toolchain].channel"
  exit 1
fi

OFFSET_DAYS="${OFFSET_DAYS:-7}"

if [ "${1:-}" != "" ]; then
  TARGET="$1"
else
  TARGET_DATE="$(date -u -d "-${OFFSET_DAYS} days" +%Y-%m-%d 2>/dev/null || date -u -v-"${OFFSET_DAYS}"d +%Y-%m-%d 2>/dev/null)"
  TARGET="nightly-${TARGET_DATE}"
fi

if ! printf '%s' "$TARGET" | grep -qE '^nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}$'; then
  echo "::error::malformed target nightly '${TARGET}'"
  exit 1
fi

echo "Current pin: ${CURRENT}"
echo "Target nightly: ${TARGET}"

if [ "$TARGET" = "$CURRENT" ]; then
  echo "Target equals the current pin; nothing to bump."
  exit 2
fi

NEW_DATE="${TARGET#nightly-}"

# ---- Resolve the matching clippy_utils rev ----
CLIPPY_REV="${CLIPPY_REV:-}"
if [ -z "$CLIPPY_REV" ]; then
  echo "Resolving clippy_utils rev for ${NEW_DATE} ..."
  CLIPPY_REV="$(node "$ROOT/.github/scripts/resolve-clippy-rev.js" "$NEW_DATE")" || {
    echo "::error::could not resolve a clippy_utils rev for ${NEW_DATE}."
    echo "::error::re-run with CLIPPY_REV=<40-char-sha> set explicitly (see the runbook)."
    exit 1
  }
fi
echo "clippy_utils rev: ${CLIPPY_REV}"

# ---- Apply edits ----
# rust-toolchain is the single source of truth.
sed -i -E "s/^channel = .*/channel = \"${TARGET}\"/" rust-toolchain

# clippy_utils rev in the lint crate.
sed -i -E "s/(rev = \"[a-f0-9]{40}\")/rev = \"${CLIPPY_REV}\"/" soroban_cost_lints/Cargo.toml

# Every other file that hardcodes the nightly string (kept consistent by
# validate-toolchain-pins.sh). Replace all dated occurrences in place.
NIGHTLY_FILES=(
  ".github/workflows/lint.yml"
  ".github/workflows/publish.yml"
  "action.yml"
  "templates/github-action.yml"
  "docs/integration.md"
  "CONTRIBUTING.md"
  "README.md"
  "docs/windows_setup.md"
)
for f in "${NIGHTLY_FILES[@]}"; do
  if [ -f "$f" ]; then
    sed -i -E "s/nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}/${TARGET}/g" "$f"
  fi
done

echo "Updated the following files:"
git --no-pager diff --stat || true
echo "Done. Working tree is modified; the caller should validate and commit."
