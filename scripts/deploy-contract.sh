#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WASM_PATH="$ROOT_DIR/target/wasm32v1-none/release/stellar_identity_core.wasm"
DEPLOYMENTS_DIR="$ROOT_DIR/deployments"

# stellar-cli identity alias (e.g. bills-futurenet) — secrets stay in ~/.config/stellar
: "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT to a stellar-cli identity alias}"
: "${STELLAR_NETWORK:=futurenet}"
# Optional separate prover; defaults to deployer public key
INIT_PROVER="${INIT_PROVER:-}"

if ! command -v stellar-cli >/dev/null 2>&1; then
  echo "error: stellar-cli not found (https://developers.stellar.org/docs/tools/cli)"
  exit 1
fi

cd "$ROOT_DIR"

if [[ ! -f "$WASM_PATH" ]]; then
  echo "Building WASM..."
  cargo build -p stellar-identity-core --target wasm32v1-none --release
fi

DEPLOYER_ADDRESS="$(stellar-cli keys address "$SOROBAN_SOURCE_ACCOUNT")"
ADMIN_ADDRESS="${INIT_ADMIN:-$DEPLOYER_ADDRESS}"
PROVER_ADDRESS="${INIT_PROVER:-$DEPLOYER_ADDRESS}"

echo "Network: $STELLAR_NETWORK"
echo "Source identity: $SOROBAN_SOURCE_ACCOUNT"
echo "Admin: $ADMIN_ADDRESS"
echo "Prover: $PROVER_ADDRESS"

DEPLOY_LOG="$(mktemp)"
trap 'rm -f "$DEPLOY_LOG"' EXIT

stellar-cli contract deploy \
  --wasm "$WASM_PATH" \
  --source-account "$SOROBAN_SOURCE_ACCOUNT" \
  --network "$STELLAR_NETWORK" \
  --alias wraith-identity-core 2>&1 | tee "$DEPLOY_LOG"

CONTRACT_ID="$(grep -oE 'C[A-Z0-9]{55}' "$DEPLOY_LOG" | tail -1)"
if [[ -z "$CONTRACT_ID" ]]; then
  echo "error: could not parse contract id from deploy output"
  exit 1
fi

echo ""
echo "Initializing contract..."
stellar-cli contract invoke \
  --id "$CONTRACT_ID" \
  --source-account "$SOROBAN_SOURCE_ACCOUNT" \
  --network "$STELLAR_NETWORK" \
  --send=yes \
  -- init --admin "$ADMIN_ADDRESS" --prover "$PROVER_ADDRESS"

echo "$CONTRACT_ID" > "$ROOT_DIR/.contract-address"

mkdir -p "$DEPLOYMENTS_DIR"
DEPLOYED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
WASM_SHA256="$(sha256sum "$WASM_PATH" | awk '{print $1}')"

cat > "$DEPLOYMENTS_DIR/${STELLAR_NETWORK}.json" <<EOF
{
  "network": "$STELLAR_NETWORK",
  "contractId": "$CONTRACT_ID",
  "wasmPath": "target/wasm32v1-none/release/stellar_identity_core.wasm",
  "wasmSha256": "$WASM_SHA256",
  "deployedAt": "$DEPLOYED_AT",
  "admin": "$ADMIN_ADDRESS",
  "prover": "$PROVER_ADDRESS",
  "packageVersion": "0.1.0"
}
EOF

echo ""
echo "Deployed: $CONTRACT_ID"
echo "Wrote $ROOT_DIR/.contract-address"
echo "Wrote $DEPLOYMENTS_DIR/${STELLAR_NETWORK}.json"
