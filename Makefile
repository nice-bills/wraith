.PHONY: test wasm sdk build-all ci smoke e2e-attested e2e-groth16 e2e-rarimo setup-zk benchmark-futurenet

test:
	cargo test

ci:
	./scripts/ci-local.sh

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

e2e-rarimo:
	./scripts/e2e-rarimo-futurenet.sh

setup-zk:
	./scripts/setup-production-zk.sh

benchmark-futurenet:
	./scripts/benchmark-futurenet.sh

