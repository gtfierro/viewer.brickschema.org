.PHONY: build

build:
	cargo build
	wasm-pack build
