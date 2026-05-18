# AGENTS.md

## Cursor Cloud specific instructions

### Overview

Wraith Identity Core is a Stellar/Soroban blockchain identity verification stack. It contains:

- **Soroban contract** (`contracts/stellar-identity-core/`) — Rust, compiled to WASM via `wasm32v1-none`
- **Proof adapter CLI** (`tools/proof-adapter/`) — Rust CLI that converts snarkjs artifacts to Soroban payloads
- **TypeScript SDK** (`sdk/stellar-identity-sdk/`) — thin client for contract invocation
- **ZK circuits** (`tools/zk-circuits/`) — optional Circom circuits for Groth16 proofs

### Toolchain requirements

- **Rust stable** (edition 2024 requires ≥1.85). Components: `rustfmt`, `clippy`. Target: `wasm32v1-none`.
- **Node.js ≥22** and **pnpm 9.15.0** (enforced via `.npmrc` `package-manager-strict=true`). Activate pnpm via `corepack enable && corepack prepare pnpm@9.15.0 --activate`.
- System utilities: `jq`, `openssl` (pre-installed on most systems).

### Key commands

| Task | Command |
|------|---------|
| Full local CI (mirrors GitHub Actions) | `make ci` |
| Rust tests only | `cargo test --all` |
| Lint (format + clippy) | `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings` |
| Build contract WASM | `cargo build -p stellar-identity-core --target wasm32v1-none --release` |
| SDK install + build | `pnpm install --frozen-lockfile && pnpm --filter @wraith/stellar-identity-sdk run build` |
| SDK tests | `pnpm --filter @wraith/stellar-identity-sdk test` |
| Adapter smoke test | `make smoke` |
| Full local build | `./scripts/build-all.sh` |

### Gotchas

- The project enforces pnpm 9.15.0 strictly. Using a different pnpm major version will fail. Always use corepack to pin the version.
- `make ci` runs `scripts/ci-local.sh` which includes all checks from the GitHub Actions CI. It is the single best command to validate changes before pushing.
- The `pnpm --filter` output shows "No projects matched the filters" before running — this is a cosmetic pnpm log and is not an error.
- E2E tests (`make e2e-attested`, `make e2e-groth16`, etc.) require a funded Stellar Futurenet identity (`SOROBAN_SOURCE_ACCOUNT` env var) and network access. These are not part of `make ci`.
- ZK circuit compilation requires `circom` 2.2.3 and `snarkjs`, which are optional and not needed for `make ci`.
- No Docker, databases, or background services are required for core development.
