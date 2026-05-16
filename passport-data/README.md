# passport-data/

Put your passport JSON here (from JMRTD / NFC export). **Never commit real files.**

```bash
cp /path/to/export.json passport-data/my-passport.json
./scripts/passport-ready.sh passport-data/my-passport.json
```

Runs are saved under `passport-data/runs/<timestamp>/` (gitignored).
