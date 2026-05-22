# Document intake — scan, parse, verify

One entrypoint routes your document to the right Wraith path.

```bash
./scripts/scan-intake.sh detect <file.json>    # what path applies?
./scripts/scan-intake.sh auto <file.json>      # detect + run
```

## Pick your document type

| Document | How you capture it | Wraith path | Command |
|----------|-------------------|-------------|---------|
| **NFC passport / chip ID** | Android [zkcreds-passport-dumper](https://github.com/rozbb/zkcreds-passport-dumper) | Rarimo full (ZK) | `scan-intake.sh nfc raw.json` |
| **Claims-only demo** | Manual JSON | Rarimo layout | `scan-intake.sh auto passport.layout.json` |
| **Non-NFC ID (your case)** | Photo + KYC or MRZ | KYC attested | `scan-intake.sh kyc` or `scan-intake.sh mrz` |
| **Already verified claims** | Backend / KYC output | Attested | `scan-intake.sh claims claims.json` |

Guides: `docs/PASSPORT_SCAN.md` (NFC) · `docs/KYC_ATTESTED.md` (no chip)

---

## NFC chip scan (passport or biometric ID card)

1. Scan with Android app → share JSON to your laptop.
2. Intake:

```bash
./scripts/scan-intake.sh nfc ~/Downloads/passport-dump.json
# optional Futurenet submit:
./scripts/scan-intake.sh nfc ~/Downloads/dump.json --submit
```

This runs: **normalize** → **validate** → **passport-ready-full** (Phase 2 Groth16 when zkeys exist).

Manual steps (same pipeline):

```bash
node scripts/normalize-passport-json.mjs raw.json passport-data/my-doc.json
node scripts/detect-document-path.mjs passport-data/my-doc.json
./scripts/passport-ready.sh --full passport-data/my-doc.json
```

**Never commit** real scan JSON — keep under `passport-data/` (gitignored).

---

## Non-NFC national ID / paper license (photo path)

Wraith does **not** OCR photos in-repo. You stage files for a KYC vendor (or manual review), then record on-chain.

### Option A — Photo intake manifest

```bash
./scripts/scan-intake.sh kyc \
  --front ~/Photos/id-front.jpg \
  --back ~/Photos/id-back.jpg \
  --selfie ~/Photos/selfie.jpg
```

Creates `passport-data/intake/<timestamp>/manifest.json` (gitignored).

After KYC approves:

```bash
# You or vendor set claims:
echo '{"age":28,"country_code":840,"is_human":true}' > passport-data/my-claims.json
./scripts/scan-intake.sh claims passport-data/my-claims.json
```

### Option B — MRZ from back of card (no NFC, no vendor yet)

If you can read the MRZ lines on your ID:

```bash
./scripts/scan-intake.sh mrz "I<UTOD231458907<<<<<<<<<<<<<<<
8608122F1110315UTO<<<<<<<<<<<6
ERIKSSON<<ANNA<MARIA<<<<<<<<<<"
```

Parses TD1/TD3 MRZ → builds `{ age, country_code, is_human }` → `attested-ready.sh`.

Or build claims without submitting:

```bash
node scripts/build-attested-claims.mjs --mrz --file mrz.txt
node scripts/build-attested-claims.mjs passport-data/claims-only.json
```

---

## Detect path only

```bash
node scripts/detect-document-path.mjs tools/zk-circuits/fixtures/passport.layout.json
# → { "path": "layout", ... }

node scripts/detect-document-path.mjs tools/zk-circuits/fixtures/nfc-dump.nested.template.json
# → { "path": "full", ... }
```

---

## Validate

```bash
node scripts/validate-passport-json.mjs --mode layout  tools/zk-circuits/fixtures/passport.layout.json
node scripts/validate-passport-json.mjs --mode full    passport-data/my-passport.json
node scripts/validate-passport-json.mjs --mode attested tools/zk-circuits/fixtures/claims.attested.template.json
```

---

## Makefile shortcuts

```bash
make scan-detect FILE=tools/zk-circuits/fixtures/passport.layout.json
make scan-intake FILE=tools/zk-circuits/fixtures/passport.layout.json
```

---

## What we do not ship (yet)

- In-app NFC reader (use Android/iOS apps above)
- Photo OCR / liveness (use Persona, Sumsub, or manual review)
- Mobile wallet UI (app layer — next)

The intake layer gets documents **into the right pipeline**; the app layer will wrap this for end users.
