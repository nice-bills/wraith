# Wraith Identity Core - Production Runbook

## Contract Status
**DEPLOYED AND OPERATIONAL** on Futurenet

- Contract ID: `CD3CGXCZUUOGWRRKPAJKOZHHLOQY5QAEBHWAQUL54XTPHEULBFTDDOZQ`
- Network: Futurenet (Test SDF Future Network ; October 2022)
- Soroban RPC: https://rpc-futurenet.stellar.org:443
- Source Key: `GDYLBKGXBZ4QWVKZGGEWEZ2CY4BNVULZ62JNI6JYGOENLNH6I5RC5SA5`

## Verified Proofs

| Ledger | Circuit | App | Status |
|--------|---------|-----|--------|
| 2808394 | simple_mult | wave | ✓ Verified |
| 2808613 | simple | ageverify | ✓ Verified |
| 2812755 | age_check | wave | ✓ Verified |

## Core Components

### Contract (`stellar-identity-core`)
- VerificationKey structure: alpha, beta (G2), delta (G2), gamma (G2), ic (G1 array)
- Proof structure: a (G1), b (G2), c (G1)
- Groth16 verification using BN254 pairing

### SDK (`stellar-identity-sdk`)
- `verify-and-record.mjs`: CLI tool for submitting proofs
- `smoke-test.mjs`: Basic contract read operations
- Uses `stellar-cli` for contract invocation

### ZK Circuits (`tools/zk-circuits/`)
- `circuits/simple_mult.circom`: Trivial multiplication proof
- `circuits/age_check.circom`: Age verification (≥18, nationality, validity)
- `circuits/passport_verifier.circom`: Full passport verification

## Pipeline

1. **Circuit Compilation**: circom 2.2.3 → circuit.wasm + circuit.r1cs
2. **Proof Generation**: snarkjs groth16 prove → proof.json + public.json
3. **Adapter Conversion**: Transform proof to contract format
4. **Submission**: stellar-cli contract invoke --send=yes verify_and_record

## SDK Usage

```bash
node src/verify-and-record.mjs <adapter.json> <appId> <subject> [nullifier] [pubInputsHash]
```

### Adapter JSON Format
```json
{
  "proof": {
    "a": ["hex", "hex", "1"],
    "b": [["hex", "hex"], ["hex", "hex"], ["1", "0"]],
    "c": ["hex", "hex", "1"]
  },
  "verification_key": {
    "alpha": "hex (64 bytes)",
    "beta": "hex (128 bytes)",
    "gamma": "hex (128 bytes)",
    "delta": "hex (128 bytes)",
    "ic": ["hex (64 bytes)", ...]
  },
  "public_signals_decimals": ["18", "840", "826", "276"],
  "claims": {"age": 30, "country_code": 840, "is_human": true}
}
```

## JMRTD Setup

Dependencies installed in `~/Downloads/`:
- `jmrtd-0.8.6.jar` (487KB)
- `bcprov-jdk18on-1.78.jar` (8MB)
- `bcutil-jdk18on-1.84.jar` (691KB)
- `scuba-smartcards-0.0.20.jar` (85KB)
- `cert-cvc-1.4.13.jar` (79KB)

Classpath: `~/Downloads/jmrtd.jar:~/Downloads/scuba-smartcards.jar:~/Downloads/bcprov.jar:~/Downloads/bcutil-jdk18on.jar:~/Downloads/cert-cvc.jar`

## Known Issues

1. **Rarimo `process_passport.js`**: Expects specific ICAO LDS ASN.1 structure; synthetic SOD doesn't match
2. **Alternative**: Use Self's test fixtures directly for circuit testing

## Next Steps for Real Passport Integration

1. Use JMRTD to extract proper ICAO LDS SOD from real/test passport
2. Feed to Rarimo's parser OR use Self's test data format
3. Generate proof with proper public signals
4. Submit via SDK

## Contract Interface

```
verify_and_record(
  app_id: Symbol,
  subject: Address,
  nullifier: BytesN<32>,
  public_inputs_hash: BytesN<32>,
  vk: VerificationKey,
  proof: Proof,
  pub_signals: Vec<U256>,
  claims: AttestedClaims
) -> VerificationRecord
```

Error cases:
- AlreadyInitialized: subject already verified for app
- NullifierAlreadyUsed: nullifier replay protection
- InvalidProof: Groth16 verification failed
- MalformedVerifyingKey: IC length mismatch
