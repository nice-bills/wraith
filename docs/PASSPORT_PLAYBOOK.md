# Passport playbook — when your passport arrives

One-time setup, then three commands on passport day.

## Before you have the chip data

```bash
cd /home/bills/code/wraith
make setup-passport    # rarimo clone, PTAU, layout zkey (~few minutes)
make ci                # confirm repo healthy
```

**Do not commit** files under `passport-data/` or real `passport.json`.

---

## What you need from the chip

Rarimo expects JSON with at least:

| Field | Source |
|-------|--------|
| `sod` | Security Object (ICAO LDS) |
| `dg1` | Data group 1 (MRZ) |
| `dateOfBirth` | MRZ / DG1 — `YYMMDD` or `YYYYMMDD` |
| `nationality` or `country_code` | MRZ — ISO3 (e.g. `USA`) or numeric (e.g. `840`) |

Template: `tools/zk-circuits/fixtures/passport.template.json`

Typical path: **NFC read → JMRTD** (Java) → export to JSON. See `docs/PRODUCTION_RUNBOOK.md` (JMRTD classpath).

---

## Passport day (recommended)

```bash
# 1. Copy your export (keep it in passport-data/)
cp /path/from/jmrtd/my-passport.json passport-data/my-passport.json

# 2. Validate + Rarimo register inputs + layout proof + optional Futurenet
export SOROBAN_SOURCE_ACCOUNT=bills-futurenet   # only if using --submit
./scripts/passport-ready.sh passport-data/my-passport.json --submit
```

Without `--submit`: you still get `passport-data/runs/<timestamp>/stellar-payload.json` for review.

---

## What each step does

| Step | Script | Output |
|------|--------|--------|
| Validate | `validate-passport-json.mjs` | Confirms `sod` + `dg1` |
| Register inputs | `passport-pipeline.sh` → `process_passport.js` | `rarimo/test/inputs/generated/*.json` |
| Layout prove | `passport-prove-layout.sh` | `proof.json`, `stellar-payload.json` |
| On-chain | `stellar-submit-rarimo.sh` | `verify_and_record` on pinned contract |

**Layout path** uses `rarimo_layout_stub.circom` — proves the **RarimoQuery claim layout** (birthDate @ signal 1, nationality @ 5) on Futurenet. It does **not** prove full passport cryptography yet.

---

## Phase 2 — full Rarimo Groth16 (later)

1. `cd tools/zk-circuits/rarimo && pnpm run build:production` (long)
2. Trusted setup for register + query zkeys
3. Witness + `snarkjs groth16 prove` on **query** circuit
4. Identity state: `idStateRoot` + `idStateSiblings` (see `fixtures/identity-state-mock.json`)
5. `proof-adapter --rarimo-mode` → `stellar-submit-rarimo.sh`

Until then, use **attested path** for fastest MVP (`make e2e-attested`) if you only need prover-backed claims.

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `process_passport` ASN.1 error | SOD not valid ICAO; re-export from chip with JMRTD |
| `cannot map nationality` | Add `"country_code": 840` to JSON |
| `missing layout zkey` | `make setup-passport` |
| `CONFIRM_DEPLOY` | Don't deploy — use pinned `deployments/futurenet.json` |
| Age wrong on-chain | Set `CURRENT_DATE_YMD=260516` (YYMMDD UTC) |

---

## Pinned contract

`deployments/futurenet.json` — use `claim_layout: RarimoQuery` and `current_date_ymd` on `verify_and_record`.

Manual submit:

```bash
./scripts/lib/stellar-submit-rarimo.sh passport-data/runs/<run>/stellar-payload.json myapp
```
