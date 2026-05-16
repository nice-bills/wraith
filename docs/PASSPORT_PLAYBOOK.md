# Passport playbook

Two paths: **A (layout)** works today on Futurenet; **B (full)** needs an NFC scan first.

| | Path A — Layout | Path B — Full Rarimo |
|---|-----------------|----------------------|
| **Scan required?** | No | Yes (`sod` + `dg1` from chip) |
| **Proves** | RarimoQuery public-signal layout | Passport crypto + query (Phase 2) |
| **Command** | `passport-ready-layout.sh` | `passport-ready-full.sh` |
| **Fixture** | `fixtures/passport.layout.json` | `fixtures/passport.template.json` (after scan) |

**Scan apps:** `docs/PASSPORT_SCAN.md`

---

## One-time setup

```bash
cd /home/bills/code/wraith
make setup-passport
make ci
```

---

## Path A — Layout (Futurenet today)

```bash
# Demo without passport
./scripts/passport-ready-layout.sh tools/zk-circuits/fixtures/passport.layout.json

# With your DOB/country only (not a real passport proof)
./scripts/passport-ready-layout.sh passport-data/claims.json --submit
```

Steps: validate (layout) → `passport-prove-layout.sh` → optional `stellar-submit-rarimo.sh`.

---

## Path B — Full (after NFC scan)

```bash
# 1. Scan with Android app → save JSON (see PASSPORT_SCAN.md)
node scripts/normalize-passport-json.mjs ~/Downloads/dump.json passport-data/my-passport.json

# 2. Register inputs + Phase 2 scaffold
./scripts/passport-ready-full.sh passport-data/my-passport.json

# 3. Futurenet (when query zkey + payload ready)
export SOROBAN_SOURCE_ACCOUNT=bills-futurenet
./scripts/passport-ready-full.sh passport-data/my-passport.json --submit
```

| Step | Script | Output |
|------|--------|--------|
| Validate | `validate-passport-json.mjs --mode full` | Real `sod` + `dg1` |
| Register inputs | `passport-pipeline.sh` → `process_passport.js` | `rarimo/test/inputs/generated/*.json` |
| Query prove | `passport-prove-rarimo-full.sh` | Full Groth16 when zkeys exist; else layout fallback |

Phase 2 completion: build Rarimo query zkey, real `idStateRoot` / siblings (`fixtures/identity-state-mock.json` is placeholder).

---

## Auto-detect mode

```bash
./scripts/passport-ready.sh passport-data/my-passport.json
```

Uses **full** if `sod`/`dg1` validate; otherwise **layout**.

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `Illegal character at offset 0` | Not real chip data — scan with app in PASSPORT_SCAN.md |
| `Full path missing sod/dg1` | Run normalize script or fill template after scan |
| `missing layout zkey` | `make setup-passport` |
| `process_passport` fails | Re-export; check dg15 for your passport type |

---

## Pinned contract

`deployments/futurenet.json` — `claim_layout: RarimoQuery`, `current_date_ymd` on `verify_and_record`.
