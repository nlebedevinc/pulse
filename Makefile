VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo dev)
export PULSE_VERSION := $(VERSION)

.PHONY: build install test vet clean

build:
	cargo build --release
	@cp target/release/pulse pulse

install:
	cargo install --path . --force

test:
	cargo test

vet:
	cargo clippy --all-targets -- -D warnings

clean:
	cargo clean
	rm -f pulse
