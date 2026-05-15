.PHONY: test wasm sdk build-all smoke e2e-attested e2e-groth16

test:
	cargo test

wasm:
	cargo build -p stellar-identity-core --target wasm32v1-none --release

sdk:
	pnpm install --frozen-lockfile
	pnpm --filter @wraith/stellar-identity-sdk run build

build-all:
	./scripts/build-all.sh

smoke:
	./scripts/adapter-smoke.sh

e2e-attested:
	./scripts/e2e-attested-futurenet.sh

e2e-groth16:
	./scripts/e2e-groth16-futurenet.sh

