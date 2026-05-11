# Wraith Identity Core

Stellar-native identity verification stack with contract, adapter, SDK, and deployment tooling.

## What is included

- **Soroban contract (`stellar-identity-core`)**
  - app policy registry
  - app lifecycle management
  - app-scoped verification records
  - nullifier replay protection (app-scoped)
  - BN254 Groth16 verification path (`verify_and_record`)
  - attested prover ingestion path (`record_attested_result`)
  - policy enforcement (min age, humanity requirement, excluded countries)
  - VK hash pinning (mandatory for Groth16 verification)
  - claims derived from public signals (cryptographically bound)
  - app approval mode for admin-controlled registration
  - record expiration enforcement on read
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

```
wraith/
├── contracts/stellar-identity-core
├── tools/proof-adapter
└── sdk/stellar-identity-sdk
```

## Build and test

### 1) Contract + adapter (Rust workspace)

```bash
cd /home/bills/code/wraith
cargo test
```

Build the contract WASM:

```bash
cargo build -p stellar-identity-core --target wasm32v1-none
```

### 2) Proof adapter usage

```bash
cd /home/bills/code/wraith
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
cd /home/bills/code/wraith/sdk/stellar-identity-sdk
npm install
npm run build
```

### 4) One-command local build

```bash
cd /home/bills/code/wraith
./scripts/build-all.sh
```

## Contract methods

- `init(admin, prover)`
- `set_prover(new_prover)`
- `register_app(app_id, policy, vk_hash?)` — app_id must be admin-approved if approval mode is required
- `update_app_policy(app_id, policy, vk_hash?)`
- `revoke_app(app_id)`
- `get_policy(app_id)`
- `get_vk_hash(app_id)`
- `is_app_registered(app_id)`
- `is_approved_app(app_id)`
- `set_app_approval(app_id, approved)` — admin only
- `set_approval_mode(required)` — admin only
- `get_admin()`
- `get_prover()`
- `verify_and_record(app_id, subject, nullifier, public_inputs_hash, vk, proof, pub_signals, claims)` — requires VK hash to be registered; claims must match derived from pub_signals
- `record_attested_result(prover, app_id, subject, nullifier, public_inputs_hash, attestation_hash, claims)` — prover auth required
- `is_verified(app_id, subject)` — enforces expiration if configured
- `get_record(app_id, subject)` — returns None if expired
- `has_nullifier(app_id, nullifier)`

## Security model

- VK hash is **mandatory** for Groth16 verification — unregistered VKs are rejected
- Claims are **cryptographically bound** to the proof — derived from pub_signals[0..2] and verified against supplied claims
- `public_inputs_hash` is **validated** against recomputed hash of pub_signals
- Nullifiers are **app-scoped** — `Nullifier(app_id, nullifier)` prevents cross-app linkability
- App registration requires **admin approval** when approval mode is enabled
- Verification records **expire** based on `expiration_window` — enforced on read

## Notes

- The Groth16 method expects BN254 points encoded in Soroban host format.
- The attested path remains pluggable for TEE-backed proving services.
- Claims are enforced by app policy at record time and derived from public signals for on-chain proofs.

## Known Limitations

### Sanctions Enforcement (Beta)
`sanctions_enabled` in `AppPolicy` is currently a **fail-safe stub**. Setting it to `true` will always reject verification with `SanctionsCheckFailed` until real circuit integration exists. This is intentional — enabling sanctions without a bound proof circuit would create a false compliance claim.

Real implementation requires:
1. A dedicated sanctions circuit that produces a Merkle non-inclusion proof
2. Proof elements passed as additional `pub_signals` (e.g., siblings + leaf index)
3. Contract verification of the proof against the stored `sanctions_root` (Merkle root)

Do NOT rely on `sanctions_enabled: true` for compliance until this circuit is integrated.

### Expiration Semantics
Records that exceed their `expiration_window` become invisible to `is_verified()` and `get_record()`, but their nullifiers still block re-verification. This means a user who expires appears "unverified" but cannot re-verify until the expiration window passes. Plan your `expiration_window` values accordingly.

### Revocation Semantics
Revoking an app removes its policy and VK hash, but existing verification records and nullifiers persist. Re-registering the same `app_id` will show the app as registered, but prior nullifiers still block duplicate verification. Use fresh `app_id` values for new deployment cycles.

## Production Readiness

For production launch:
- Complete external security audit
- Cost benchmarking on Futurenet
- Design context: see `docs/ARCHITECTURE_NOTES.md`
