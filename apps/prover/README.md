# @wraith/prover

Phase 2 backend — KYC webhooks, attested prepare/submit, and verification status. **No frontend** in this package.

Full API reference: [`docs/PROVER_API.md`](../../docs/PROVER_API.md)

## Quick start

```bash
export SOROBAN_SOURCE_ACCOUNT=bills-futurenet
./scripts/register-stable-app.sh
pnpm --filter @wraith/prover dev
curl -s localhost:8787/health | jq .
```

## Webhook example (generic)

```bash
curl -s -X POST localhost:8787/webhook/kyc \
  -H 'Content-Type: application/json' \
  -H "x-wraith-webhook-secret: $PROVER_WEBHOOK_SECRET" \
  -d '{
    "wallet": "G...",
    "age": 25,
    "country": "NG",
    "is_human": true
  }'
```

Set `AUTO_SUBMIT_ATTESTED=1` to write on-chain immediately (dev keys only).

## Test

```bash
pnpm --filter @wraith/prover test
```
