.PHONY: test wasm sdk build-all smoke

test:
	cargo test

wasm:
	cargo build -p stellar-identity-core --target wasm32v1-none

sdk:
	pnpm install --frozen-lockfile
	pnpm --filter @wraith/stellar-identity-sdk run build

build-all:
	./scripts/build-all.sh

smoke:
	./scripts/adapter-smoke.sh

