.PHONY: release debug test

release:
	cargo build --release

debug:
	cargo build

test:
	cargo test

