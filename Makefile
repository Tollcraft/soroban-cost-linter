.PHONY: fmt lint test bench doc check

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

bench:
	cargo bench -p cargo-cost-lint

doc:
	cargo doc --no-deps --open

check: fmt lint test