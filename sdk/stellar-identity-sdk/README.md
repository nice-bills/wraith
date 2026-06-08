# @wraith/stellar-identity-sdk

Thin TypeScript client for the Stellar identity core contract.

## Core methods

- `init`
- `getAdmin`
- `getProver`
- `setProver`
- `registerApp(appId, policy, vkHash?)` — returns AppPolicy
- `updateAppPolicy(appId, policy, vkHash?)`
- `revokeApp(appId)`
- `getPolicy(appId)`
- `getVkHash(appId)`
- `isAppRegistered(appId)`
- `isApprovedApp(appId)`
- `setAppApproval(appId, approved)`
- `setApprovalMode(required)`
- `verifyAndRecord(appId, subject, payload)` — requires registered VK hash
- `recordAttestedResult(appId, subject, payload)`
- `isVerified(appId, subject)` — enforces expiration
- `getRecord(appId, subject)` — returns null if expired
- `hasNullifier(appId, nullifier)`

## Helpers

- `fromAdapterPayload(adapter, nullifier, options?)` — converts `proof-adapter` JSON into `verifyAndRecord` input
- `computePublicInputsHash(publicSignalsHex, currentDateYmd?)` — matches the contract digest, including Rarimo `current_date_ymd` binding
- `computeAttestedClaimsHash(claims)` — matches `hash_attested_claims`
- `computeAttestationHash(...)` / `buildAttestedPayload(...)` — build attested prover inputs off-chain
- `normalizeHex32(value)` — pad and normalize a 32-byte hex value

For `ClaimLayout = "RarimoQuery"`, preserve the adapter's `current_date_ymd` value. `fromAdapterPayload()` carries it through automatically and includes it in the default `publicInputsHash`.

## AppPolicy interface

```typescript
interface AppPolicy {
  owner: string;
  minAge: number;
  requireHumanity: boolean;
  sanctionsRoot: Hex;
  excludedCountries: number[];
  expirationWindow: number;
  sanctionsEnabled: boolean;
  claimLayout: ClaimLayout;
}
```

## Build

```bash
pnpm install
pnpm run build
```

## Test

```bash
pnpm test
```
