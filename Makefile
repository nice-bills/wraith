.PHONY: test wasm sdk build-all ci smoke e2e-attested e2e-groth16 e2e-rarimo setup-zk setup-passport passport-ready benchmark-futurenet

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

setup-passport:
	./scripts/setup-passport-stack.sh

setup-rarimo-phase2:
	./scripts/setup-rarimo-phase2.sh

passport-layout:
	./scripts/passport-ready-layout.sh tools/zk-circuits/fixtures/passport.layout.json

passport-ready:
	@echo "Path A (layout): ./scripts/passport-ready-layout.sh tools/zk-circuits/fixtures/passport.layout.json"
	@echo "Path B (full):   ./scripts/passport-ready-full.sh passport-data/my-passport.json"
	@echo "Scan guide:      docs/PASSPORT_SCAN.md"
	@echo "First run:       make setup-passport"

benchmark-futurenet:
	./scripts/benchmark-futurenet.sh

