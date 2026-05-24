# @wraith/prover

Phase 2 backend — verification status API and admin hooks for the attested (KYC) path.

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Liveness + pinned contract id |
| GET | `/verify/:wallet?appId=` | On-chain `is_verified` + record (default `appId`: `wraith`) |
| POST | `/admin/approve` | `{ "appId", "approved" }` → `set_app_approval` |
| POST | `/webhook/kyc` | Stage KYC payload (in-memory; wire to `record_attested_result` next) |
| GET | `/admin/pending` | List staged KYC webhooks |

## Run

```bash
pnpm install
pnpm --filter @wraith/prover dev
```

Env: `CONTRACT_ID`, `RPC_URL`, `STABLE_APP_ID`, `PORT` (optional).

Submit attested proofs on Futurenet: `./scripts/attested-ready.sh` or `scan-intake.sh claims`.
