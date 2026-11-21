#!/usr/bin/env bash
# Verify that every file in soroban_cost_lints/ui_pending/ is listed in the
# Rust manifest test (cargo-cost-lint/tests/ui_pending_manifest.rs).
#
# Exits non-zero if any file on disk is missing from MANIFEST or any MANIFEST
# entry is missing from disk.  Run from the workspace root.
#
# This script is a lightweight shell-level companion to the Rust test so that
# the check can be run in CI without a full `cargo test` build.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

UI_PENDING="${WORKSPACE_ROOT}/soroban_cost_lints/ui_pending"
MANIFEST_RS="${WORKSPACE_ROOT}/cargo-cost-lint/tests/ui_pending_manifest.rs"

if [[ ! -d "${UI_PENDING}" ]]; then
    echo "ERROR: ui_pending directory not found: ${UI_PENDING}" >&2
    exit 1
fi

if [[ ! -f "${MANIFEST_RS}" ]]; then
    echo "ERROR: manifest test not found: ${MANIFEST_RS}" >&2
    exit 1
fi

# Collect files on disk (names only, sorted).
mapfile -t DISK_FILES < <(find "${UI_PENDING}" -maxdepth 1 -type f -printf '%f\n' | sort)

# Extract the MANIFEST entries from the Rust source file.
# Lines look like:  "README.md",
mapfile -t MANIFEST_FILES < <(
    grep -oP '(?<=")\S+(?=",)' "${MANIFEST_RS}" | sort
)

ERRORS=0

# Check for files on disk missing from the manifest.
for f in "${DISK_FILES[@]}"; do
    if ! printf '%s\n' "${MANIFEST_FILES[@]}" | grep -qxF "${f}"; then
        echo "ORPHAN: '${f}' is in ui_pending/ but not in the manifest." >&2
        ERRORS=$((ERRORS + 1))
    fi
done

# Check for manifest entries missing from disk.
for f in "${MANIFEST_FILES[@]}"; do
    if ! printf '%s\n' "${DISK_FILES[@]}" | grep -qxF "${f}"; then
        echo "MISSING: '${f}' is in the manifest but not in ui_pending/." >&2
        ERRORS=$((ERRORS + 1))
    fi
done

if [[ "${ERRORS}" -gt 0 ]]; then
    echo ""
    echo "ui_pending manifest check FAILED (${ERRORS} issue(s))." >&2
    echo "Update MANIFEST in ${MANIFEST_RS} to match the directory contents." >&2
    exit 1
fi

echo "ui_pending manifest check passed (${#DISK_FILES[@]} file(s) verified)."
