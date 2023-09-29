.PHONY: build

build:
	cargo build
	wasm-pack build --target web --out-dir ./www/pkg  --release
