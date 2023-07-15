
all:

fmt:
	cargo +nightly fmt

doc:
	cargo doc --all-features

clippy:
	cargo clippy --all-features

build-release:
	cargo build --release

build-debug:
	cargo build

test-release:
	cargo nextest run --release --no-fail-fast

test-debug:
	cargo nextest run --no-fail-fast

test-release-all-features:
	cargo nextest run --release --no-fail-fast --all-features

test-debug-all-features:
	cargo nextest run --no-fail-fast --all-features


clean:
	cargo clean
