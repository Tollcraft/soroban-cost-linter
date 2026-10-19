.PHONY: fmt fmt-check lint test check-docs sync-lint-data bench doc check check-pins

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

check-docs:
	cargo run -p generate-lint-docs -- --check

# Refresh cargo-cost-lint/lint-data/, the snapshot build.rs reads when the
# crate is built outside this workspace (e.g. from a published package).
sync-lint-data:
	UPDATE_LINT_DATA=1 cargo test -p cargo-cost-lint --test lint_data_snapshot

bench:
	cargo bench -p cargo-cost-lint

doc:
	cargo doc --no-deps --open

check-pins:
	bash .github/scripts/validate-toolchain-pins.sh

# Note: The corpus baseline regeneration step from CI (which is conditional on
# tests/corpus/baseline.json being empty and requires cargo-dylint) is omitted
# here because it requires external tools installed; make check runs the
# toolchain-pin guard and the static and test gates that match CI.
check: check-pins fmt-check lint test check-docs

