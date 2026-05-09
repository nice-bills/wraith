# Self Stellar Identity Core

Stellar-native identity verification stack with contract, adapter, SDK, and deployment tooling.

## What is included

- **Soroban contract (`stellar-identity-core`)**
  - app policy registry
  - app lifecycle management
  - app-scoped verification records
  - nullifier replay protection
  - BN254 Groth16 verification path (`verify_and_record`)
  - attested prover ingestion path (`record_attested_result`)
  - policy enforcement (min age, humanity requirement, excluded countries)
  - lifecycle and verification events
- **Proof adapter CLI (`proof-adapter`)**
  - converts `snarkjs`-style `proof.json`, `verification_key.json`, and `public.json`
  - emits Soroban-friendly BN254 payload JSON (hex-encoded points + scalar list)
  - optional claims derivation from public signal indexes
- **TypeScript SDK (`@wraith/stellar-identity-sdk`)**
  - simple wrapper to invoke contract methods from frontend/backend apps
  - runtime validation and Soroban RPC invoke helper
- **Ops tooling**
  - `scripts/build-all.sh` for full local build
  - `scripts/deploy-contract.sh` for Soroban deployment
  - `docs/PRODUCTION_RUNBOOK.md` for end-to-end execution
  - `docs/ARCHITECTURE_NOTES.md` for the current build direction

## Project layout

```text
self-stellar/
├── contracts/stellar-identity-core
├── tools/proof-adapter
└── sdk/stellar-identity-sdk
```

## Build and test

### 1) Contract + adapter (Rust workspace)

```bash
cd /home/bills/code/self-stellar
cargo test
```

Build the contract WASM:

```bash
cargo build -p stellar-identity-core --target wasm32v1-none
```

### 2) Proof adapter usage

```bash
cd /home/bills/code/self-stellar
cargo run -p proof-adapter -- \
  --proof /path/to/proof.json \
  --vk /path/to/verification_key.json \
  --public /path/to/public.json \
  --age-index 0 \
  --country-index 1 \
  --humanity-index 2 \
  --out /tmp/stellar-proof-payload.json
```

### 3) SDK build

```bash
cd /home/bills/code/self-stellar/sdk/stellar-identity-sdk
npm install
npm run build
```

### 4) One-command local build

```bash
cd /home/bills/code/self-stellar
./scripts/build-all.sh
```

## Contract methods

- `init(admin, prover)`
- `set_prover(new_prover)`
- `register_app(app_id, policy)`
- `update_app_policy(app_id, policy)`
- `revoke_app(app_id)`
- `get_policy(app_id)`
- `is_app_registered(app_id)`
- `get_admin()`
- `get_prover()`
- `verify_and_record(app_id, subject, nullifier, public_inputs_hash, vk, proof, pub_signals, claims)`
- `record_attested_result(prover, app_id, subject, nullifier, public_inputs_hash, attestation_hash, claims)`
- `is_verified(app_id, subject)`
- `get_record(app_id, subject)`
- `has_nullifier(nullifier)`

## Emitted events

- `Initialized`
- `AppRegistered`
- `AppUpdated`
- `AppRevoked`
- `ProverUpdated`
- `VerificationRecorded`

## Notes

- The Groth16 method expects BN254 points encoded in Soroban host format.
- The attested path remains pluggable for TEE-backed proving services.
- Claims are enforced by app policy at record time.
- For production launch, complete external security audit and cost benchmarking.
- For design context and source-of-truth comparisons, read `docs/ARCHITECTURE_NOTES.md`.
