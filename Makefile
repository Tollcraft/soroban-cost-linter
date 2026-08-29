.PHONY: fmt fmt-check lint test check-docs bench doc check

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

bench:
	cargo bench -p cargo-cost-lint

doc:
	cargo doc --no-deps --open

# Note: The corpus baseline regeneration step from CI (which is conditional on
# tests/corpus/baseline.json being empty and requires cargo-dylint) is omitted
# here because it requires external tools installed; make check runs the static
# and test gates that match CI.
check: fmt-check lint test check-docs