# KYC / attested path (no NFC chip)

Use this when the user has **no NFC** on their ID (paper license, old national ID card, photo-only docs).

You do **not** use Rarimo or passport scripts. You use **`record_attested_result`** — already deployed on Futurenet.

## How it works (3 parties)

```text
1. User ──► KYC company (Persona, Sumsub, etc.)
              checks ID photo + liveness
              sends result to YOUR backend (webhook)

2. Your backend ──► decides claims: age, country, is_human

3. User wallet + prover wallet ──► Stellar: record_attested_result
              contract stores "verified" for that app
```

Wraith does **not** import another project's ZK circuits. The KYC company replaces the chip; **your contract** stores the outcome.

## What you need

| Role | Who |
|------|-----|
| **Prover** | Your backend's Stellar address (set at `init` / `deployments/futurenet.json` → `prover`) |
| **Subject** | User's Stellar wallet |
| **App** | You register an app with policy (`min_age`, `country`, etc.) |

Both **prover and subject must sign** the `record_attested_result` transaction.

## Test it now (no KYC company yet)

Simulate "KYC passed" on Futurenet with your own keys:

```bash
export SOROBAN_SOURCE_ACCOUNT=bills-futurenet   # prover + subject for testing
./scripts/attested-ready.sh --age 25 --country 840 --human
```

Or with a claims file:

```json
{ "age": 25, "country_code": 840, "is_human": true }
```

```bash
./scripts/attested-ready.sh claims.json
```

## Plug in a real KYC provider (later)

1. Sign up with a vendor (e.g. [Persona](https://withpersona.com/), [Sumsub](https://sumsub.com/)).
2. On **approved** webhook, map their payload → `{ age, country_code, is_human }`.
3. Your server builds the attestation hashes (use SDK `buildAttestedPayload` or contract `hash_*` helpers).
4. User signs a Stellar tx that calls `record_attested_result` (your app/wallet co-signs as prover).

**This repo:** use `apps/prover` webhooks + `POST /admin/submit-attested`, or `AUTO_SUBMIT_ATTESTED=1` for dev (single key). See `docs/PROVER_API.md`.

We do **not** ship a Persona API key in this repo. Configure webhooks to point at your prover host.

## Not the same thing

| Product | What it does |
|---------|----------------|
| **Wraith attested path** | Your Soroban contract records verification |
| **Self / Rarimo** | Chip ZK only — wrong for non-NFC ID |
| **Gitcoin Passport / Holonym stamps** | Separate Stellar stamps — different contract, optional add-on |

## Your national ID (no NFC)

Use **this path only**. Run `make e2e-attested` or `./scripts/attested-ready.sh` after you (or a KYC vendor) accept the document off-chain.
