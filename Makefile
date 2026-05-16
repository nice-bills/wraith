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

passport-ready:
	@echo "Usage: ./scripts/passport-ready.sh passport-data/my-passport.json [--submit]"
	@echo "First run: make setup-passport"

benchmark-futurenet:
	./scripts/benchmark-futurenet.sh

