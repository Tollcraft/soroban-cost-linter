.PHONY: fmt lint test doc check

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

doc:
	cargo doc --no-deps --open

check: fmt lint test