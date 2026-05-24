# Prover API (`@wraith/prover`)

Backend service for **Model B (KYC attested)** — no frontend in this repo. Integrates Persona/Sumsub webhooks and on-chain `record_attested_result`.

## Run

```bash
export SOROBAN_SOURCE_ACCOUNT=bills-futurenet
./scripts/register-stable-app.sh          # once: app id `wraith`
pnpm --filter @wraith/prover dev
```

| Env | Purpose |
|-----|---------|
| `SOROBAN_SOURCE_ACCOUNT` | Prover key (must match contract `get_prover`) |
| `STABLE_APP_ID` | Default app (default: `wraith` from `deployments/stable-app.json`) |
| `PROVER_WEBHOOK_SECRET` | Require `x-wraith-webhook-secret` on webhooks |
| `AUTO_SUBMIT_ATTESTED=1` | Submit on-chain immediately after webhook (dev: same key as subject) |
| `KYC_DEFAULT_AGE` / `KYC_DEFAULT_COUNTRY` | Fallback when vendor payload lacks fields |

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Contract id + stable app |
| GET | `/config/stable-app` | Policy template |
| GET | `/verify/:wallet` | `is_verified` + record |
| POST | `/attested/prepare` | Build hashes; optional `{ submit: true }` |
| POST | `/webhook/kyc` | Generic `{ wallet, age, country_code }` |
| POST | `/webhook/persona` | Persona inquiry approved |
| POST | `/webhook/sumsub` | Sumsub `applicantReviewed` GREEN |
| POST | `/admin/submit-attested` | `{ "id" }` from pending queue |
| POST | `/admin/approve` | `{ "appId", "approved" }` |
| GET | `/admin/pending` | Staged KYC awaiting submit |

## Production note

`record_attested_result` requires **prover + subject** signatures. This service submits with `SOROBAN_SOURCE_ACCOUNT` (dev: one key for both). Production: return `prepared` hashes from webhook and have the **user wallet** co-sign the transaction (Freighter / wallet app — not in this repo yet).

## CLI equivalent

```bash
./scripts/attested-ready.sh claims.json --app wraith --subject G... --json
./scripts/attested-ready.sh claims.json --prepare-only --json
```
