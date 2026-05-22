# passport-data/

Gitignored workspace for scans, intake manifests, and prove runs.

## Quick start

```bash
# Detect route from any JSON
./scripts/scan-intake.sh detect tools/zk-circuits/fixtures/passport.layout.json

# NFC dump from phone
./scripts/scan-intake.sh nfc ~/Downloads/passport-dump.json

# Non-NFC photo ID
./scripts/scan-intake.sh kyc --front ~/Photos/id-front.jpg

# MRZ from card (no chip)
./scripts/scan-intake.sh mrz "<MRZ lines>"
```

Full guide: **`docs/DOCUMENT_INTAKE.md`**

**Never commit real passport or ID files.**
