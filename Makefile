.PHONY: fmt-check fmt clippy

fmt-check:
	cargo +nightly fmt --all -- --check

fmt:
	cargo +nightly fmt

clippy:
	cargo +nightly clippy --all-features --tests --