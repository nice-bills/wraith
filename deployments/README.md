# Deployment pins

## Policy

- **`futurenet.json` is the team pin** — one contract ID everyone uses for E2E and integration.
- **Do not redeploy on every merge.** Merges run `make ci` only (no network, no new contract).
- **Deploy only** when the contract ABI or storage layout breaks and you intentionally cut a new release.

## Pinned Futurenet contract (current)

See `futurenet.json` for `contractId`, `wasmSha256`, and `previousContractId`.

Override for a one-off test without changing the pin:

```bash
export CONTRACT_ID=C...
make e2e-groth16
```

## Deploying a new instance

```bash
export SOROBAN_SOURCE_ACCOUNT=bills-futurenet
export CONFIRM_DEPLOY=1
export UPDATE_DEPLOYMENT_JSON=1   # only when pinning for the whole repo
./scripts/deploy-contract.sh
```

Without `UPDATE_DEPLOYMENT_JSON=1`, the new ID is written only to `.contract-address` (gitignored).

## CI

GitHub Actions never deploy. It runs the same checks as `make ci`.
