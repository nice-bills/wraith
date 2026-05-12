# Wraith Identity Core - Production Runbook

## Contract Status
**DEPLOYED AND OPERATIONAL** on Futurenet

- Network: Futurenet (Test SDF Future Network ; October 2022)
- Soroban RPC: https://rpc-futurenet.stellar.org:443

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
- `smoke-test.mjs`: Read-only contract smoke test (requires `CONTRACT_ID` env var)
- Full TypeScript SDK in `src/index.ts` for programmatic invocation
- Uses `stellar-cli` for contract invocation

### ZK Circuits (`tools/zk-circuits/`)
**⚠️ DEMO-ONLY: These circuits are not production-sound and should not be used for real identity verification.**

- `circuits/simple_mult.circom`: Trivial multiplication proof (demo only)
- `circuits/age_check.circom`: Age verification (≥18, nationality, validity) - **demo only, not cryptographically sound**
- `circuits/passport_verifier.circom`: Passport verification - **demo only, contains known issues**

For production use, replace with properly audited circuits using real range checks, proper comparator gadgets, and passport document authenticity verification.

## Pipeline

1. **Circuit Compilation**: circom 2.2.3 → circuit.wasm + circuit.r1cs
2. **Proof Generation**: snarkjs groth16 prove → proof.json + public.json
3. **Adapter Conversion**: `cargo run -p proof-adapter` to transform proof to contract format
4. **Submission**: stellar-cli contract invoke --send=yes verify_and_record

## SDK Usage

```bash
# Read-only smoke test (no writes)
CONTRACT_ID=<contract_id> node smoke-test.mjs
```

### Adapter JSON Format (proof-adapter output)
```json
{
  "proof": {
    "a": "0x...",   // G1 point as 0x-prefixed hex string (64 bytes)
    "b": "0x...",   // G2 point as 0x-prefixed hex string (128 bytes)
    "c": "0x..."    // G1 point as 0x-prefixed hex string (64 bytes)
  },
  "verification_key": {
    "alpha": "0x...",   // G1 point, 64 hex bytes
    "beta": "0x...",    // G2 point, 128 hex bytes
    "gamma": "0x...",   // G2 point, 128 hex bytes
    "delta": "0x...",  // G2 point, 128 hex bytes
    "ic": ["0x...", "0x...", ...]  // G1 points array, min 1 element
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
  pub_signals: Vec<Bn254Fr>,
  claims: AttestedClaims
) -> VerificationRecord
```

Error cases:
- AlreadyInitialized: contract already initialized
- SubjectAlreadyVerified: subject already has a non-expired record for this app
- NullifierAlreadyUsed: nullifier already used for this app
- InvalidProof: Groth16 verification failed
- VkMismatch: VK hash does not match registered hash
- PolicyViolation: claims violate app policy