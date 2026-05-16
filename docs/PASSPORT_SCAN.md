# Passport NFC scan → JSON

Wraith does **not** include a scanner app in this repo. You use a reader app or JMRTD once, save JSON under `passport-data/`, then run our scripts.

## Recommended apps (scan → JSON)

| Platform | Tool | Output |
|----------|------|--------|
| **Android** | [zkcreds-passport-dumper](https://github.com/rozbb/zkcreds-passport-dumper) | Large JSON blob (share to laptop). Prebuilt APKs in repo releases. |
| **Android** | [tananaev/passport-reader](https://github.com/tananaev/passport-reader) | Alternative NFC reader (upstream of zkcreds dumper). |
| **iOS** | [rarimo/NFCPassportReader](https://github.com/rarimo/NFCPassportReader) | Swift library — needs a small host app to export JSON (Rarimo ecosystem). |
| **Desktop** | **JMRTD** (Java) + USB NFC reader | Export matching Rarimo `sod` / `dg1` / `dg15` fields. See `docs/PRODUCTION_RUNBOOK.md`. |

### Android quick path (most practical)

1. Install **zkcreds-passport-dumper** APK from the [repo](https://github.com/rozbb/zkcreds-passport-dumper).
2. Enter MRZ fields (passport number, DOB, expiry) — required to unlock the chip.
3. Hold phone on passport until read completes.
4. Share the JSON to your machine (Signal, email, `adb`, etc.).
5. Normalize and run:

```bash
cd /home/bills/code/wraith
node scripts/normalize-passport-json.mjs ~/Downloads/passport-dump.json passport-data/my-passport.json
./scripts/passport-ready.sh --full passport-data/my-passport.json
```

If field names differ, edit `scripts/normalize-passport-json.mjs` aliases or map manually to `tools/zk-circuits/fixtures/passport.template.json`.

---

## What you need in the JSON

### Path A — Layout demo (no scan required)

| Field | Example |
|-------|---------|
| `dateOfBirth` | `950101` (YYMMDD) |
| `nationality` or `country_code` | `USA` or `840` |

Fixture: `tools/zk-circuits/fixtures/passport.layout.json`

```bash
./scripts/passport-ready-layout.sh tools/zk-circuits/fixtures/passport.layout.json
```

This proves **RarimoQuery signal layout** on Futurenet — **not** that your passport is genuine.

### Path B — Full Rarimo (real scan)

| Field | Source |
|-------|--------|
| `sod` | Security Object (base64 or hex) |
| `dg1` | Data group 1 |
| `dg15` | Active Authentication key (many passports) |
| `dateOfBirth`, `nationality` | MRZ / DG1 |

```bash
./scripts/passport-ready-full.sh passport-data/my-passport.json
```

`process_passport.js` parses **real ICAO ASN.1** in `sod`. Placeholder text like `<base64...>` will fail with `Illegal character at offset 0` — that is expected until you scan.

---

## After scan

```bash
make setup-passport          # once per machine
./scripts/passport-ready.sh --full passport-data/my-passport.json
./scripts/passport-ready.sh --full passport-data/my-passport.json --submit   # Futurenet
```

Phase 2 (full query Groth16 + identity tree): `docs/PASSPORT_PLAYBOOK.md` and `scripts/passport-prove-rarimo-full.sh`.

**Never commit** real passport JSON.
