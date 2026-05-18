# Identity documents — what Wraith supports

## Pick one path

| Your document | Path | Guide |
|---------------|------|--------|
| Passport or ID **with NFC chip** | Rarimo ZK | `docs/PASSPORT_PLAYBOOK.md` |
| ID **without NFC** (your case) | **KYC attested** | **`docs/KYC_ATTESTED.md`** |
| Paper driver’s license (no chip) | **KYC attested** | **`docs/KYC_ATTESTED.md`** |

## Chip path (ICAO eMRTD) — Rarimo only

Needs NFC. Verifies chip signatures (`sod`, `dg1`, …).

| Document | Rarimo | Wraith |
|----------|--------|--------|
| NFC passport | TD3 | `make setup-rarimo-phase2` |
| NFC national ID card | TD1 | `make setup-rarimo-phase2-td1` |
| **No NFC** | — | Use **KYC attested** instead |

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

## National ID without NFC

Do **not** use passport/Rarimo scripts. Use the attested path:

```bash
export SOROBAN_SOURCE_ACCOUNT=bills-futurenet
./scripts/attested-ready.sh tools/zk-circuits/fixtures/claims.attested.template.json
```

See **`docs/KYC_ATTESTED.md`** for plugging in Persona/Sumsub later.

## National ID with NFC (chip card)

Same flow as passport, TD1 circuits: `make setup-rarimo-phase2-td1`, then `passport-ready-full.sh` after scan (`docs/PASSPORT_SCAN.md`).
