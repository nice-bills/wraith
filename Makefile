.PHONY: test wasm sdk build-all ci smoke e2e-attested e2e-groth16 e2e-rarimo e2e-rarimo-query setup-zk setup-passport setup-rarimo-phase2 setup-rarimo-phase2-td1 passport-layout attested-ready passport-ready scan-intake scan-detect benchmark-futurenet register-stable-app prover-dev prover-test

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

register-stable-app:
	./scripts/register-stable-app.sh

prover-dev:
	pnpm --filter @wraith/prover dev

prover-test:
	pnpm --filter @wraith/prover test

passport-ready:
	@echo "No NFC:  ./scripts/scan-intake.sh kyc --front <photo>  (docs/DOCUMENT_INTAKE.md)"
	@echo "NFC:     ./scripts/scan-intake.sh nfc <dump.json>"
	@echo "Detect:  ./scripts/scan-intake.sh detect <file.json>"

scan-detect:
	@test -n "$(FILE)" || (echo "Usage: make scan-detect FILE=path/to/doc.json" && exit 1)
	./scripts/scan-intake.sh detect "$(FILE)"

scan-intake:
	@test -n "$(FILE)" || (echo "Usage: make scan-intake FILE=path/to/doc.json" && exit 1)
	./scripts/scan-intake.sh auto "$(FILE)"

benchmark-futurenet:
	./scripts/benchmark-futurenet.sh

