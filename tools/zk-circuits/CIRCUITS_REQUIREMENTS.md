# ZK Circuit Requirements for Wraith Identity Core

## Current Status

| Component | Status | Notes |
|-----------|--------|-------|
| Demo Circuits | :warning: Demo only | `age_check.circom`, `passport_verifier.circom`, `simple_mult.circom` |
| Rarimo Integration | :white_check_mark: Available | Production passport ZK circuits |

## Directory Structure

```
tools/zk-circuits/
├── circuits/                    # Demo circuits (circomlib-based)
│   ├── age_check.circom
│   ├── passport_verifier.circom
│   └── simple_mult.circom
├── rarimo/                      # Production circuits (ISO 18013C/ICAO)
│   ├── circuits/
│   │   ├── identityManagement/  # Register & Query circuits
│   │   └── ...
│   ├── README.md
│   └── package.json
├── build/                       # Compiled artifacts
├── proof/                       # Generated proofs
├── INTEGRATION.md               # This file
├── CIRCUITS_REQUIREMENTS.md     # Requirements doc
├── package.json
└── setup-trusted-setup.sh
```

## Demo Circuits

These use proper circomlib primitives but are for testing only:

### age_check.circom
- Uses `GreaterEqThan` from circomlib
- Checks age >= minAge
- Checks nationality in allowed list
- Constrains passportValid to boolean

### passport_verifier.circom
- Uses `GreaterEqThan` from circomlib
- Checks age >= minAge
- Checks humanity flag
- Checks country not excluded

### simple_mult.circom
- Basic multiplication test
- For pipeline testing only

## Production Circuits: Rarimo

### What Rarimo Provides

Rarimo's passport-zk-circuits implements:

| Feature | Description |
|---------|-------------|
| **BAC** | Basic Access Control - secure passport chip communication |
| **CA** | Chip Authentication - prove chip authenticity |
| **SOD** | Security Object Data - verify document signature |
| **MRZ** | Machine Readable Zone - extract personal data |
| **AA** | Active Authentication - prove chip not cloned |

### Circuit Types

| Circuit | Purpose |
|---------|---------|
| `registerIdentityBuilder` | Link identity key pair to passport |
| `queryIdentity` | TD3 passport queries |
| `queryIdentityTD1` | TD1 passport queries |

### Rarimo Public Signals (Query Circuit)

```
[0]  nullifier           - Poseidon hash for replay protection
[1]  birthDate           - Birth date (passport timestamp)
[2]  expirationDate     - Passport expiration
[3]  name               - Full name
[4]  nameResidual       - Name remainder
[5]  nationality         - Country code (UTF-8)
[6]  citizenship         - Citizenship (UTF-8)
[7]  sex                - Gender
[8]  documentNumber     - Document number
[9]  eventID            - Event/challenge ID
[10] eventData          - Bound event data
[11] idStateRoot        - Identity state Merkle root
[12] selector            - Controls data revelation
[13] currentDate         - Current date
... (more, see rarimo README)
```

## Integration with Wraith Contract

### Wraith Expected Claims

```rust
struct AttestedClaims {
    age: u32,           // Age in years
    country_code: u32,  // Country code
    is_human: bool,    // Always true for passport holders
}
```

### Rarimo → Wraith Mapping

| Rarimo Signal | Wraith Claim | Conversion |
|--------------|--------------|-----------|
| `birthDate` | `age` | Compute: currentDate - birthDate |
| `nationality` | `country_code` | Direct mapping |
| (always 1) | `is_human` | Passport holder = human |

## Trusted Setup

### Groth16 Requirements

Groth16 proving system requires:

1. **Powers of Tau (Universal)** - One-time setup per max circuit size
2. **Circuit-specific** - Per-circuit setup after PtA

### Options for PTAU

#### Option 1: Pre-contributed (Development)

Download commonly-used PTAU from Hermez/Polygon:

```bash
curl -L -o build/ptau/powersOfTau28_hez_final_15.ptau \
  https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_15.ptau
```

**WARNING**: Trust assumption - relying on contributor's honesty.

#### Option 2: Run Your Own (Production)

```bash
# Initialize ceremony for 2^20 constraints
snarkjs powersoftau new bn254 20 ceremony.ptau -v

# Contribute (do multiple times with different people)
snarkjs powersoftau contribute ceremony.ptau contrib1.ptau -e="entropy1"
snarkjs powersoftau contribute contrib1.ptau contrib2.ptau -e="entropy2"

# Beacon (use large random beacon)
snarkjs powersoftau beacon contrib2.ptau final.ptau 1 -n=2
```

#### Option 3: Use Established Ceremony

Use ceremony from Hermez, Polygon, or similar project.

### Circuit-Specific Setup

```bash
# After PtA is ready, setup each circuit
snarkjs groth16 setup circuit.r1cs final.ptau circuit_0000.zkey

# Contribute to ceremony (multi-party recommended)
snarkjs zkey contribute circuit_0000.zkey circuit_0001.zkey -n="First"

# Export verification key
snarkjs zkey export verificationkey circuit_final.zkey verification_key.json
```

## Proof Generation Flow

```bash
# 1. Prepare input (from passport data)
cd rarimo
node helpers/prepare-input.js passport.json input.json

# 2. Generate witness
node circuits/query_js/generate_witness.js \
  input.json \
  circuits/query.r1cs \
  witness.wtns

# 3. Generate proof
snarkjs groth16 prove \
  circuits/query_final.zkey \
  witness.wtns \
  proof.json \
  public.json

# 4. Convert to Soroban format
cd ../..
cargo run -p proof-adapter -- \
  --proof proof.json \
  --vk verification_key.json \
  --public public.json \
  --out output.json

# 5. Submit via SDK
# (Use StellarIdentityClient.verifyAndRecord)
```

## Reference Implementations

| Resource | URL |
|----------|-----|
| Rarimo Circuits | https://github.com/rarimo/passport-zk-circuits |
| Rarimo Audits | https://github.com/rarimo/passport-zk-circuits/tree/main/audits |
| circomlib | https://github.com/iden3/circomlib |
| SnarkJS | https://github.com/iden3/snarkjs |
| Circom Docs | https://docs.circom.io |

## Production Checklist

- [ ] Integrate rarimo circuits
- [ ] Run trusted setup ceremony
- [ ] Publish ceremony transcript
- [ ] Generate and securely store proving key
- [ ] Deploy verification key on-chain
- [ ] External security audit
- [ ] End-to-end integration testing

## Security Considerations

1. **PTAU Toxic Waste**: Never expose ceremony randomness
2. **Proving Key**: Protect in production
3. **Verification Key**: Can be public
4. **Circuit Soundness**: Rarimo is audited, demo circuits are not
5. **Input Validation**: Validate passport data before circuit execution

## Grant Application Notes

For grant applications, emphasize:

1. **Architecture**: Clean separation (circuits → adapter → contract → SDK)
2. **Library Usage**: circomlib, rarimo (established libraries)
3. **Trusted Setup**: Clear roadmap for ceremony
4. **Audit Plan**: Commitment to external security audit
5. **ICAO Compliance**: Rarimo follows ISO 18013C standards