# Production ZK Roadmap (Futurenet, no mainnet)

This plan covers production-grade circuits, passport integration, sanctions, trusted setup, and gas benchmarking **without mainnet deployment**. Mainnet stays blocked until: external audit, production ceremony transcript, and benchmark sign-off.

## What you already have

| Piece | Status |
|-------|--------|
| Soroban contract (Groth16 + attested, VK pin, claims binding) | Shipped on Futurenet |
| `proof-adapter` + `--rarimo-mode` | Maps Rarimo query `public.json` → Wraith claims |
| `rarimo/` submodule | Audited passport circuits (register + query) |
| `e2e_claims.circom` | Pipeline test only (3 pub signals) |
| Demo circuits | Must **not** ship as “production” |
| Sanctions | Fail-safe stub (`sanctions_enabled` rejects registration) |
| Trusted setup script | Downloads Hermez PTAU (dev); does not run full MPC |
| Futurenet E2E | `make e2e-attested`, `make e2e-groth16` |

## Architecture (target)

```
Passport chip / JMRTD
        │
        ▼
  passport.json (Rarimo input format)
        │
        ▼
  rarimo: registerIdentityBuilder  ──► identity in SMT (see Phase 2)
        │
        ▼
  rarimo: queryIdentity            ──► proof.json + public.json
        │
        ▼
  proof-adapter --rarimo-mode        ──► stellar-payload.json + claims
        │
        ▼
  verify_and_record (Futurenet)     ──► VerificationRecorded
```

Sanctions (Phase 3) adds a **fourth public signal** (or dedicated circuit) proving non-membership in `policy.sanctions_root`.

---

## Phase 1 — Production circuits (replace demos)

**Goal:** Stop using `age_check` / `passport_verifier` for anything labeled production. Use **Rarimo** (or a fork you audit).

### Steps

1. **Pin Rarimo version** in `tools/zk-circuits/rarimo` (commit SHA in `deployments/circuits.json` — add when built).
2. **Build production artifacts:**
   ```bash
   cd tools/zk-circuits
   pnpm run setup:rarimo
   pnpm run build:production   # rarimo compile
   ```
3. **Choose circuit for Wraith v1:**
   - **Register:** `registerIdentityBuilder` — passport authenticity (BAC/CA/SOD/DG1/DG15).
   - **Query:** `queryIdentity` — prove age/country/etc. from registered identity (needs `idStateRoot` + siblings).
4. **Mark demos explicitly** in UI/docs as `DEMO ONLY`; keep `e2e_claims` for CI pipeline smoke only.

### Exit criteria

- [ ] `rarimo` compiles on your machine; `verification_key.json` + `zkey` paths documented
- [ ] Adapter `--rarimo-mode` run on real `public.json` from query circuit (not fixtures)
- [ ] Demo circuits removed from “production” docs and grant copy

### References

- Rarimo audits: https://github.com/rarimo/passport-zk-circuits/tree/main/audits
- `tools/proof-adapter/src/rarimo_transformer.rs` (birthDate → age, nationality → country_code)

---

## Phase 2 — Passport path (JMRTD → Rarimo → contract)

**Goal:** End-to-end proof from passport data to `verify_and_record` on Futurenet.

### 2a — Extract passport JSON (JMRTD)

1. Install JMRTD + BouncyCastle (see `docs/PRODUCTION_RUNBOOK.md` JMRTD section).
2. Read chip → produce JSON matching Rarimo’s expected shape (`rarimo/test/inputs/passport`, `process_passport.js`).
3. **Known gap:** synthetic SOD often fails Rarimo’s ASN.1 parser — use **real test passport** or **Rarimo/Self official test vectors**.

```bash
# After JSON exists:
cd tools/zk-circuits/rarimo
node test/process_passport.js   # or project-specific wrapper
```

### 2b — Identity state (hardest part)

Rarimo **query** assumes the passport is registered in an **identity sparse Merkle tree** (`idStateRoot`, `idStateSiblings`).

| Approach | Pros | Cons |
|----------|------|------|
| **A. Rarimo on-chain SMT** (their stack) | Full fidelity | Not Stellar-native; extra chain |
| **B. Wraith contract stores commitment** | Stellar-native | Contract + circuit changes |
| **C. Attested register, Groth16 query only** | Fast MVP | Register step trusted/TEE |
| **D. Mock SMT for Futurenet** | Unblocks E2E | Not production |

**Recommended for you now:** **D → C → B**

1. **Futurenet:** mock `idStateRoot` / siblings from Rarimo test fixtures; run query proof → adapter → `verify_and_record` (script: `scripts/e2e-rarimo-futurenet.sh` — add in Phase 2).
2. **Product:** attested `record_attested_result` after your prover runs register+query off-chain.
3. **Later:** store `passportHash` / `dg1Commitment` on Soroban after register proof.

### 2c — Prove and submit

```bash
# Generate proof (paths depend on rarimo build output)
snarkjs groth16 prove <query.zkey> witness.wtns proof.json public.json

cargo run -p proof-adapter -- \
  --rarimo-mode \
  --current-date 260515 \
  --proof proof.json --vk verification_key.json --public public.json \
  --out build/rarimo/stellar-payload.json

# Register app with VK hash, then verify_and_record (same as e2e-groth16-futurenet.sh)
```

### Exit criteria

- [ ] One successful Futurenet `verify_and_record` with Rarimo-derived claims (not `e2e_claims`)
- [ ] Documented passport JSON source (test vector name + hash)
- [ ] `make e2e-rarimo` in CI optional (heavy; nightly)

---

## Phase 3 — Sanctions (Merkle non-inclusion)

**Goal:** `sanctions_enabled: true` means something cryptographically, not a stub.

### Circuit

Add `tools/zk-circuits/circuits/sanctions_non_inclusion.circom` (or compose with query):

- **Private:** subject id / wallet hash, Merkle path siblings
- **Public:** `sanctions_root`, `cleared` (1 if leaf not in tree)
- Use circomlib `MerkleTreeChecker` or sparse tree template (align depth with list size, e.g. 20–32)

### Contract changes (`stellar-identity-core`)

1. Allow `sanctions_enabled: true` at registration only if `sanctions_root != 0`.
2. Replace `check_sanctions` stub:
   - Require `pub_signals.len() >= 4` when sanctions enabled
   - `pub_signals[3]` decodes to `cleared == 1`
   - Hash of `pub_signals[0..3]` still binds to `public_inputs_hash`
3. **VK strategy:** either
   - **Single circuit** (query + sanctions) → one VK hash per app, or
   - **Two-step:** sanctions checked in same `verify_and_record` with extended IC length (adapter validates `ic.len() == pub_signals.len() + 1`)

### Adapter

- `--sanctions-index 3` (or compose in rarimo-mode output)
- Document OFAC/sanctions list → Merkle tree builder (off-chain, versioned, root in policy)

### Exit criteria

- [ ] Unit tests: inclusion in list → proof fails; not in list → passes
- [ ] Futurenet E2E with `sanctions_enabled: true` and real Merkle root
- [ ] README sanctions section updated from “stub” to “beta with circuit X”

---

## Phase 4 — Trusted setup (production ceremony)

**Two tracks** — do not confuse them.

### Track A — Development / Futurenet (now)

- PTAU: `setup-trusted-setup.sh` downloads Hermez `powersOfTau28_hez_final_15.ptau`
- Per-circuit: `snarkjs groth16 setup` + **single** `zkey contribute` (fine for Futurenet)
- **Trust:** you trust Hermez + your machine

### Track B — Production (before mainnet / real users)

1. **Powers of Tau:** run or join MPC; publish transcript (`build/ptau/transcript.md` + hashes).
2. **Circuit phase:** multi-party `zkey contribute` (≥3 parties); discard toxic waste.
3. **Artifacts:**
   - **Public:** `verification_key.json`, VK hash on-chain
   - **Private:** proving key in HSM / prover service only
4. **Pin in repo:** `deployments/ceremony.json` with PTAU hash, zkey hash, participant list, date

```bash
# Production PTAU (example — adjust power for constraint count)
snarkjs powersoftau new bn254 20 build/ptau/ceremony_0000.ptau -v
snarkjs powersoftau contribute ... # repeat N times
snarkjs powersoftau beacon ... final.ptau
```

### Exit criteria

- [ ] `deployments/ceremony.json` committed for the circuit you ship
- [ ] Proving key not in git
- [ ] Grant/audit narrative references transcript hash

---

## Phase 5 — Futurenet validation (replaces mainnet for now)

| Test | Command |
|------|---------|
| Unit | `cargo test`, `pnpm --filter @wraith/stellar-identity-sdk test` |
| Adapter | `make smoke` |
| Attested E2E | `SOROBAN_SOURCE_ACCOUNT=... make e2e-attested` |
| Groth16 pipeline | `make e2e-groth16` |
| Rarimo E2E | `make e2e-rarimo` (add with Phase 2) |
| Sanctions E2E | `make e2e-sanctions` (add with Phase 3) |

Record results in `deployments/futurenet-validation.json` (ledger, app_id, tx hash, claims).

---

## Phase 6 — Gas / resource benchmarking (Futurenet)

**Goal:** Data for cost estimates before any mainnet decision.

```bash
export SOROBAN_SOURCE_ACCOUNT=bills-futurenet
./scripts/benchmark-futurenet.sh
# After E2E, measure real write paths:
TX_VERIFY_AND_RECORD=<hash> TX_RECORD_ATTESTED=<hash> ./scripts/benchmark-futurenet.sh
```

Script uses `stellar tx fetch fee` (proposed vs charged resource fees). Compare:

- `register_app`
- `record_attested_result`
- `verify_and_record` (Groth16 — largest)
- `is_verified` (read)

Store output under `deployments/benchmarks/` (commit JSON summaries, not keys).

### Exit criteria

- [ ] Table of charged resource fees for each method
- [ ] Notes on IC length / pub_signal count impact on `verify_and_record`

---

## Mainnet — explicitly out of scope

Do **not** deploy to public mainnet until:

1. External audit of contract + adapter + ceremony
2. Phase 3 sanctions (if compliance is a selling point)
3. Phase 4 Track B ceremony for the circuit you ship
4. Phase 6 benchmarks reviewed

---

## Suggested order of work

```
Phase 1 (Rarimo build) ──► Phase 2 (passport E2E, mock SMT ok)
        │
        ├──► Phase 6 (benchmark in parallel)
        │
        └──► Phase 4 Track A (zkey for query circuit on Futurenet)

Phase 3 (sanctions) ──► after Phase 2 proves adapter path

Phase 4 Track B ──► before any “production launch” narrative

Mainnet ──► deferred
```

## First concrete tasks (this week)

1. `cd tools/zk-circuits && pnpm run build:production` — fix compile errors if any.
2. Run Rarimo tests: `cd rarimo && pnpm test` with official passport fixture.
3. `./scripts/benchmark-futurenet.sh` — baseline fees on current contract.
4. Sketch `sanctions_non_inclusion.circom` + contract `pub_signals[3]` (design doc in PR).

## Related files

- `tools/zk-circuits/CIRCUITS_REQUIREMENTS.md`
- `tools/zk-circuits/INTEGRATION.md`
- `tools/proof-adapter/src/rarimo_transformer.rs`
- `scripts/e2e-groth16-futurenet.sh` (template for `e2e-rarimo-futurenet.sh`)
- `scripts/benchmark-futurenet.sh`
