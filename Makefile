.PHONY: test wasm sdk build-all smoke

test:
	cargo test

wasm:
	cargo build -p stellar-identity-core --target wasm32v1-none

sdk:
	cd sdk/stellar-identity-sdk && npm install --silent && npm run build --silent

build-all:
	./scripts/build-all.sh

smoke:
	./scripts/adapter-smoke.sh

