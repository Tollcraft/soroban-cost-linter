# cargo-budget-report

Reports Soroban smart-contract function resource costs as a share of the network's resource limits.

Each reported metric shows its raw value alongside its percentage of the corresponding Soroban protocol resource limit — so you can immediately tell whether a function is near the ceiling or safely in the clear.

## What the percentages mean

The **Share of Limit** column shows what fraction of the network's per-transaction resource budget each function consumes:

- **CPU Instructions** — share of the maximum instruction count allowed per transaction
- **Read Bytes** — share of the maximum bytes the transaction may read from the ledger
- **Write Bytes** — share of the maximum bytes the transaction may write to the ledger

A function at **5%** of the instruction limit needs no attention. One at **85%** is one input away from failing on-chain.

## Where the limits come from

Limits are fetched from the Soroban network whenever possible. If the network is unreachable, the tool falls back to **hardcoded limits versioned against Protocol 22** (10 billion instructions, 100 MB read/write bytes). These hardcoded limits are documented on every run and must be reviewed when the protocol upgrades.

## Usage

```bash
# Build and report with default testnet limits
cargo run -p cargo-budget-report

# Use a custom network endpoint
cargo run -p cargo-budget-report -- --network https://my-soroban-rpc.example.com

# Raise the flag threshold to 80%
cargo run -p cargo-budget-report -- --threshold 80

# Get JSON output for CI consumption
cargo run -p cargo-budget-report -- --format json
```

## Output formats

### Text (default)

A table with Package, Function, Metric, Value, Share of Limit, Threshold, and Flag columns. Rows where the share exceeds the `--threshold` are flagged with a warning symbol and listed in a summary below the table.

### JSON

Structured JSON including the limits used, the limit source (network or hardcoded with protocol version), the threshold, and every function report with its share percentage.