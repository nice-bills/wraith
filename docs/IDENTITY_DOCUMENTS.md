# Identity documents — what Wraith supports

## Chip required (ICAO eMRTD)

Wraith’s Rarimo path verifies **NFC chip data** (ICAO 9303 LDS): `sod`, `dg1`, often `dg15`. The chip signature proves the document is authentic.

| Document | Rarimo doc type | Wraith path |
|----------|-----------------|-------------|
| Biometric **passport** (booklet) | TD3 (`…_3_…` in circuit name) | `queryIdentity` |
| **National ID** / residence permit (card, NFC) | TD1 (`…_1_…`) | `queryIdentityTD1` |
| **Driver’s license** (paper/plastic, no chip) | — | **Not supported** |
| **Utility bill / selfie of ID** | — | **Not supported** |

Same scan apps usually work for **passport and chip ID cards** (zkcreds-passport-dumper, etc.). Export still goes to `passport-data/` — never commit it.

## Self Protocol (Celo) — different stack

[Self](https://self.xyz/) on [Celo](https://docs.celo.org/build-on-celo/build-with-self) is a **finished product**: Self mobile app, their circuits, Celo contracts (humanity, age, nationality). It supports passports, EU biometric ID cards, and Aadhaar.

**Wraith does not embed Self.** Wraith is **Stellar / Soroban + Rarimo Groth16 + your contract**. You cannot “take proofs from Self on Celo” and submit them to Wraith without a **bridge** (new verification keys, layout, trust model).

| | Self (Celo) | Wraith (Stellar) |
|---|-------------|------------------|
| Chain | Celo | Stellar Futurenet |
| Prover | Self app | Your machine (`passport-ready-full`) |
| Circuits | Self | Rarimo |
| Integration | Self SDK / contracts | `verify_and_record` |

Possible **later**: verify Self attestations on Stellar (separate feature). Not a substitute for Rarimo TD1 work.

## What to scan with your national ID

1. Confirm the card has **NFC** (contactless symbol).
2. Scan with Android [zkcreds-passport-dumper](https://github.com/rozbb/zkcreds-passport-dumper) (same MRZ unlock flow as passport).
3. `node scripts/normalize-passport-json.mjs dump.json passport-data/my-id.json`
4. `make setup-rarimo-phase2-td1` then `./scripts/passport-ready-full.sh passport-data/my-id.json`

`process_passport.js` picks TD1 vs TD3 from DG1 size; Wraith selects the matching query zkey and claim layout.
