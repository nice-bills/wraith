# passport-data/

Put NFC scan JSON here. **Never commit real passport files.**

1. Scan with an app — see **`docs/PASSPORT_SCAN.md`** (Android: zkcreds-passport-dumper).
2. Normalize and run full path:

```bash
node scripts/normalize-passport-json.mjs ~/Downloads/dump.json passport-data/my-passport.json
./scripts/passport-ready-full.sh passport-data/my-passport.json
```

Layout demo (no scan): `./scripts/passport-ready-layout.sh tools/zk-circuits/fixtures/passport.layout.json`

Runs under `passport-data/runs/` (gitignored).
