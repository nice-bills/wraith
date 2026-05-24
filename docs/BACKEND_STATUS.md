# Backend completion status (no frontend)

Last updated after Phase 2 prover + thermo-nuclear refactors.

## Done

| Area | Status |
|------|--------|
| Soroban contract | Futurenet pinned; `claims.rs`, `finalize_verification`, tests split |
| Claim layout spec | `deployments/claim-layout-spec.json` + cross-layer goldens |
| Proof adapter | Standard + Rarimo layouts; CLI smoke |
| SDK | `@wraith/stellar-identity-sdk`; attested + Groth16 helpers |
| Document intake | `scan-intake.sh`, MRZ, KYC staging, attested path |
| KYC attested ops | `attested-ready.sh`, `register-stable-app.sh`, `e2e-attested` |
| **Prover API** | `apps/prover` — webhooks, prepare/submit, verify status |
| CI | `make ci` — Rust, SDK, adapter, intake tests |

## Your path (non-NFC national ID)

1. KYC vendor or manual review → claims JSON  
2. `./scripts/scan-intake.sh claims file.json` **or** prover webhook  
3. On-chain `record_attested_result` for app `wraith`

## Not in scope (explicitly deferred)

| Item | Notes |
|------|--------|
| **Frontend / Freighter** | Phase 3 — do not add until requested |
| Mainnet deploy | Phase 7 — Futurenet only for now |
| Contract redeploy for new Rarimo indices | Only if you switch to NFC ZK on-chain |
| Production MPC ceremony | `docs/PRODUCTION_ZK_ROADMAP.md` Phase 4 |
| Sanctions Merkle circuit | Phase 3 in ZK roadmap |

## Optional next backend work

- Wire prover `AUTO_SUBMIT` with separate subject signer (XDR builder)  
- Persona/Sumsub field mapping for your exact vendor templates  
- Hosted prover + `PROVER_WEBHOOK_SECRET` in production  
- `make e2e-prover` against Futurenet (integration test)
