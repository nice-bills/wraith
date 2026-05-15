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
