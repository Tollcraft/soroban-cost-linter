# validate-toolchain-pins.ps1
#
# Drift guard for the nightly toolchain pin (PowerShell version).
# Uses rust-toolchain as the single source of truth for the nightly version
# and soroban_cost_lints/Cargo.toml for the clippy_utils git rev.
#
# Fails if any of the hardcoded copies drift from the canonical source.
#
# Usage: powershell -ExecutionPolicy Bypass -File .github/scripts/validate-toolchain-pins.ps1

$ErrorActionPreference = "Stop"

# ---- Parse canonical nightly from rust-toolchain (single source of truth) ----
$toolchainContent = Get-Content -Raw "rust-toolchain"
if ($toolchainContent -match 'channel = "(nightly-\d{4}-\d{2}-\d{2})"') {
    $canonicalNightly = $Matches[1]
} else {
    Write-Error "::error file=rust-toolchain::Could not parse [toolchain].channel from rust-toolchain"
    exit 1
}
Write-Host "Canonical nightly (from rust-toolchain): $canonicalNightly"

$nightlyDate = $canonicalNightly -replace '^nightly-', ''
$failed = $false

# ---- Check every file that hardcodes the nightly string ----
function Check-NightlyInFile {
    param([string]$file)

    $content = Get-Content -Raw $file
    $pattern = 'nightly-\d{4}-\d{2}-\d{2}'
    $matches = [regex]::Matches($content, $pattern) | ForEach-Object { $_.Value } | Sort-Object -Unique

    if ($matches.Count -eq 0) {
        Write-Warning "No nightly version found in $file (expected $canonicalNightly)"
        return
    }

    $localFailed = $false
    foreach ($found in $matches) {
        if ($found -ne $canonicalNightly) {
            Write-Error "::error file=$file::Mismatch in ${file}: found '${found}' but expected '${canonicalNightly}'. Edit ${file} to use the canonical version."
            $script:failed = $true
            $localFailed = $true
        }
    }

    if (-not $localFailed) {
        Write-Host "OK: $file matches canonical nightly"
    }
}

Check-NightlyInFile ".github/workflows/lint.yml"
Check-NightlyInFile ".github/workflows/publish.yml"
Check-NightlyInFile "action.yml"
Check-NightlyInFile "docs/integration.md"
Check-NightlyInFile "templates/github-action.yml"
Check-NightlyInFile "CONTRIBUTING.md"
Check-NightlyInFile "README.md"
Check-NightlyInFile "docs/windows_setup.md"

# ---- Check clippy_utils git rev matches the nightly date ----
$cargoToml = Get-Content -Raw "soroban_cost_lints/Cargo.toml"
if ($cargoToml -match 'rev = "([a-f0-9]{40})"') {
    $clippyRev = $Matches[1]
} else {
    Write-Error "::error file=soroban_cost_lints/Cargo.toml::Could not parse clippy_utils git rev"
    $failed = $true
}

if ($clippyRev) {
    Write-Host "clippy_utils rev: $clippyRev"

    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/rust-lang/rust-clippy/commits/$clippyRev"
        $commitDate = ($response.commit.committer.date -split 'T')[0]
    } catch {
        Write-Error "::error file=soroban_cost_lints/Cargo.toml::Invalid or unreachable clippy_utils rev $clippyRev. Update the rev in soroban_cost_lints/Cargo.toml."
        $failed = $true
        $commitDate = $null
    }

    if ($commitDate) {
        $commitTs = [DateTimeOffset]::Parse($commitDate).ToUnixTimeSeconds()
        $nightlyTs = [DateTimeOffset]::Parse($nightlyDate).ToUnixTimeSeconds()
        $diffAbs = [Math]::Abs($commitTs - $nightlyTs)
        $daysDiff = [int]($diffAbs / 86400)

        if ($daysDiff -gt 3) {
            Write-Error "::error file=soroban_cost_lints/Cargo.toml::clippy_utils rev $clippyRev is from $commitDate ($daysDiff days away from nightly $canonicalNightly). Update the rev in soroban_cost_lints/Cargo.toml to match the nightly."
            $failed = $true
        } else {
            Write-Host "OK: clippy_utils rev $clippyRev ($commitDate) is within 3 days of nightly $nightlyDate"
        }
    }
}

if ($failed) {
    exit 1
}

Write-Host "All toolchain pin checks passed."
