.PHONY: test wasm sdk build-all ci smoke e2e-attested e2e-groth16 e2e-rarimo setup-zk setup-passport attested-ready passport-ready benchmark-futurenet

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

e2e-rarimo-query:
	./scripts/e2e-rarimo-query-futurenet.sh

setup-zk:
	./scripts/setup-production-zk.sh

setup-passport:
	./scripts/setup-passport-stack.sh

setup-rarimo-phase2:
	./scripts/setup-rarimo-phase2.sh

setup-rarimo-phase2-td1:
	./scripts/setup-rarimo-phase2-td1.sh

passport-layout:
	./scripts/passport-ready-layout.sh tools/zk-circuits/fixtures/passport.layout.json

attested-ready:
	./scripts/attested-ready.sh

passport-ready:
	@echo "No NFC:  ./scripts/attested-ready.sh  (docs/KYC_ATTESTED.md)"
	@echo "NFC ZK:  ./scripts/passport-ready-layout.sh (docs/PASSPORT_PLAYBOOK.md)"

benchmark-futurenet:
	./scripts/benchmark-futurenet.sh

