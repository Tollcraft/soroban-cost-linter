# License Information for Corpus Contracts

All contracts in `tests/corpus/contracts/` are original code written specifically
for the soroban-cost-linter false-positive benchmarking corpus.

- **Author**: soroban-cost-linter contributors
- **License**: Apache-2.0 (same as the main project)
- **Third-party code**: None. These fixtures are not derived from or copies of
  any existing open-source Soroban contracts. They are minimal examples that
  exercise common Soroban contract patterns (token, timelock, swap) solely for
  the purpose of measuring lint false-positive rates.

## Rationale for Vendoring (vs. Fetching at CI Time)

Contracts are vendored in-repo to:

1. **Pin exact code** — fetching at CI time would require trusting remote
   sources and handling network failures, rate limits, and mutable tags.
2. **Reproducibility** — every commit of the linter carries a fixed set of
   test fixtures; CI results are deterministic.
3. **License compliance** — vendoring lets us verify licenses once up front.

The tradeoff is repository size growth (the contracts are minimal, ~50–100 LoC
each) and the need to periodically refresh the corpus as the Soroban SDK
evolves.
