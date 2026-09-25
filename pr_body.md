- closes #485
- closes #487
- closes #491
- closes #492

## Changes
- **#485**: Modified `config.rs` to return `LinterResult<Self>` instead of `Result<Self, String>` from `from_file_validated`, leveraging the existing `LinterError` type. Test assertions were also updated to check the string representation of the returned error.
- **#487**: Normalised lint names inside `BudgetConfig::from_file_validated` and `build_effective_lint_flags`. This ensures that kebab-case lint names (e.g., `soroban-storage-in-loop`) are correctly converted to snake_case before validation or checking against `LINT_NAMES`.
- **#491**: Made the configuration loading in `main.rs` strict. Previously, a missing or unreadable `budget.toml` was caught and swallowed. Now, `BudgetConfig::from_file_validated` propagates I/O errors so the CLI correctly reports the error and exits.
- **#492**: Stopped `main.rs` from ignoring malformed `budget.toml` errors. Instead of printing a warning and continuing when TOML parsing failed, it now fails loudly by propagating the validation error, unifying the exit behaviour of invalid TOML and invalid lint levels.
