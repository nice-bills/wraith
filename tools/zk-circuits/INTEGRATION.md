# Wraith ZK Circuit Integration

## Overview

This directory contains ZK circuits for Wraith Identity Core:

| Circuit | Type | Status |
|---------|------|--------|
| `circuits/age_check.circom` | Demo | Works with circomlib |
| `circuits/passport_verifier.circom` | Demo | Works with circomlib |
| `circuits/simple_mult.circom` | Demo | Works with circomlib |
| `rarimo/` | Production | ISO 18013C/ICAO compliant passport ZK |

## Quick Start

```bash
# Install dependencies
cd tools/zk-circuits
npm install

# Setup rarimo (requires network)
git submodule update --init --recursive
cd rarimo && npm install && cd ..

# Run trusted setup
bash setup-trusted-setup.sh

# Build production circuits
npm run build:production
```

## Rarimo Integration

### What Rarimo Provides

Rarimo's `passport-zk-circuits` provides production-ready circuits for:

- **Passport Verification**: BAC, CA, SOD verification per ICAO standards
- **Identity Registration**: Link identity key pair to passport
- **Identity Queries**: Prove attributes from registered passport

### Rarimo Public Signals (Query Circuit)

The rarimo query circuit outputs these public signals:

| Index | Signal | Description |
|-------|--------|-------------|
| 0 | `nullifier` | Poseidon hash for replay protection |
| 1 | `birthDate` | Birth date (passport timestamp format) |
| 2 | `expirationDate` | Passport expiration date |
| 5 | `nationality` | Country code (UTF-8 encoded) |
| 6 | `citizenship` | Citizenship (UTF-8 encoded) |
| 12 | `selector` | Controls what data is revealed |

### Mapping Rarimo to Wraith Claims

Wraith's contract expects claims derived from public signals:

```rust
// Wraith expected format
struct AttestedClaims {
    age: u32,
    country_code: u32,
    is_human: bool,
}
```

**Mapping from Rarimo signals:**

| Rarimo Signal | Wraith Claim | Conversion |
|--------------|--------------|------------|
| `birthDate` | `age` | Compute from birthDate vs currentDate |
| `nationality` | `country_code` | Direct mapping |
| (always true) | `is_human` | Passport holder = human |

### Integration Options

#### Option A: New Contract Function (Recommended)

Add `verify_and_record_rarimo()` to the contract that accepts rarimo proofs:

```rust
pub fn verify_and_record_rarimo(
    env: Env,
    app_id: Symbol,
    subject: Address,
    nullifier: BytesN<32>,
    public_inputs_hash: BytesN<32>,
    vk: VerificationKey,
    proof: Proof,
    pub_signals: Vec<Bn254Fr>,  // Rarimo format
    current_date: u32,          // For age calculation
) -> Result<VerificationRecord, IdentityError>
```

#### Option B: Adapter Layer

Create an adapter that transforms rarimo outputs to Wraith format before calling existing `verify_and_record`.

## Trusted Setup

### What is Trusted Setup?

Groth16 requires a "trusted setup" ceremony to generate proving/verification keys. This prevents a "toxic waste" that could allow forge proofs.

### Setup Options

#### Option 1: Pre-contributed PTAU (Development/Testing)

Download a commonly-used PTAU file:

```bash
# Download from Hermez network
curl -L -o build/ptau/powersOfTau28_hez_final_15.ptau \
  https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_15.ptau
```

**WARNING**: This means trusting that the contributors didn't collude.

#### Option 2: Run Your Own Ceremony

For production, run your own ceremony:

```bash
# Initialize ceremony
snarkjs powersoftau new bn254 15 build/ptau/ceremony.ptau -v

# Contribute randomness (do this with many participants)
snarkjs powersoftau contribute build/ptau/ceremony.ptau build/ptau/contribution.ptau --name="First contribution" -e="random entropy"

# Export final ptau
snarkjs powersoftau beacon build/ptau/contribution.ptau build/ptau/final.ptau 1 -n=2
```

#### Option 3: Use Established Ceremony

Use a ceremony from an established project (e.g., Hermez, Polygon).

## Building Rarimo Circuits

```bash
cd rarimo

# Install dependencies
npm install

# Compile circuits (requires circom 2.x)
npm run compile

# This generates:
# - circuits/*.r1cs (constraint systems)
# - circuits/*_js/*.wasm (witness calculators)
# - circuits/*_js/witness_calculator.js
```

## Generating Proofs

### With Rarimo

```bash
# Generate witness
node circuits/query_js/generate_witness.js \
  input.json \
  circuits/query.r1cs \
  witness.wtns

# Generate proof
snarkjs groth16 prove \
  circuits/query_final.zkey \
  witness.wtns \
  proof.json \
  public.json
```

### Convert for Wraith

```bash
# Use proof-adapter to convert to Soroban format
cargo run -p proof-adapter -- \
  --proof proof.json \
  --vk verification_key.json \
  --public public.json \
  --out output.json
```

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Passport       │     │  Rarimo         │     │  Proof          │
│   (JSON)        │────▶│  Circuits       │────▶│  Adapter        │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                         │
                                                         ▼
                        ┌─────────────────┐     ┌─────────────────┐
                        │  Soroban        │◀────│  Stellar       │
                        │  Contract      │     │  Identity SDK  │
                        └─────────────────┘     └─────────────────┘
```

## Reference Circuits

| Circuit | Use Case | Source |
|---------|----------|--------|
| Register Identity | Link passport to identity | rarimo |
| Query Identity | Prove passport attributes | rarimo |
| Age Check | Prove age >= X | demo |
| Citizenship Check | Prove citizenship | rarimo |

## Security Notes

- Always use audited circuits for production
- Verify trusted setup ceremony transcript
- Keep proving keys secure
- Never reuse randomness from ceremony

## Links

- [Rarimo GitHub](https://github.com/rarimo/passport-zk-circuits)
- [Circom Docs](https://docs.circom.io/)
- [SnarkJS](https://github.com/iden3/snarkjs)
- [circomlib](https://github.com/iden3/circomlib)