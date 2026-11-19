#!/usr/bin/env bash
# Tests that validate-toolchain-pins.ps1 catches a missing guarded file,
# emits an error annotation, and exits non-zero — matching the Bash
# counterpart's behavior at validate-toolchain-pins.sh:29.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
PS1_SCRIPT="$REPO_ROOT/.github/scripts/validate-toolchain-pins.ps1"

pass() { echo "PASS: $1"; }
fail() { echo "FAIL: $1" >&2; exit 1; }

# --- Verify the try/catch block is present in the script ---

if grep -q 'Get-Content -Raw \$file -ErrorAction Stop' "$PS1_SCRIPT"; then
    pass "Get-Content uses -ErrorAction Stop"
else
    fail "Get-Content in Check-NightlyInFile must use -ErrorAction Stop"
fi

if grep -q 'try {' "$PS1_SCRIPT" && grep -q 'catch {' "$PS1_SCRIPT"; then
    pass "try/catch block present in Check-NightlyInFile"
else
    fail "Missing try/catch block around Get-Content in Check-NightlyInFile"
fi

if grep -q '::error file=\$file::File not found' "$PS1_SCRIPT"; then
    pass "error annotation emitted on missing file"
else
    fail "Missing '::error file=\$file::File not found' annotation in catch block"
fi

if grep -q '\$script:failed = \$true' "$PS1_SCRIPT"; then
    pass "\$script:failed incremented in catch block"
else
    fail "Missing '\$script:failed = \$true' in catch block"
fi

echo "All toolchain_pin_ps1_missing_file checks passed."
