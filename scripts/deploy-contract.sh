#!/usr/bin/env bash
# Deploy a NEW Soroban contract instance (new contract ID every time).
#
# This is NOT part of CI or merge workflow. Run only for intentional breaking releases.
#
# Required:
#   export SOROBAN_SOURCE_ACCOUNT=bills-futurenet
#   export CONFIRM_DEPLOY=1
#
# Optional:
#   UPDATE_DEPLOYMENT_JSON=1  — also overwrite deployments/<network>.json (for maintainer releases)
#   STELLAR_NETWORK=futurenet
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WASM_PATH="$ROOT_DIR/target/wasm32v1-none/release/stellar_identity_core.wasm"
DEPLOYMENTS_DIR="$ROOT_DIR/deployments"

: "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT to a stellar-cli identity alias}"
: "${STELLAR_NETWORK:=futurenet}"
INIT_PROVER="${INIT_PROVER:-}"

if [[ "${CONFIRM_DEPLOY:-}" != "1" ]]; then
  echo "error: refusing to deploy without CONFIRM_DEPLOY=1"
  echo ""
  echo "Deploying creates a NEW contract ID. Apps, VK hashes, and verifications on the"
  echo "previous instance are not migrated. Do not deploy on every merge — use:"
  echo ""
  echo "  make ci                    # local checks (no network)"
  echo "  make e2e-attested          # test against pinned ID in deployments/futurenet.json"
  echo ""
  if [[ -f "$DEPLOYMENTS_DIR/${STELLAR_NETWORK}.json" ]]; then
    echo "Pinned ${STELLAR_NETWORK} contract:"
    jq -r '"  " + .contractId + " (deployed " + .deployedAt + ")"' \
      "$DEPLOYMENTS_DIR/${STELLAR_NETWORK}.json" 2>/dev/null || true
  fi
  echo ""
  echo "To deploy a new instance intentionally:"
  echo "  export CONFIRM_DEPLOY=1"
  echo "  # optional: commit new pinned ID for the team"
  echo "  export UPDATE_DEPLOYMENT_JSON=1"
  echo "  ./scripts/deploy-contract.sh"
  exit 1
fi

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

PREVIOUS_ID=""
if [[ -f "$DEPLOYMENTS_DIR/${STELLAR_NETWORK}.json" ]]; then
  PREVIOUS_ID="$(jq -r .contractId "$DEPLOYMENTS_DIR/${STELLAR_NETWORK}.json" 2>/dev/null || true)"
fi

echo "⚠️  Deploying NEW contract on $STELLAR_NETWORK (not an upgrade)."
if [[ -n "$PREVIOUS_ID" && "$PREVIOUS_ID" != "null" ]]; then
  echo "   Current pinned ID: $PREVIOUS_ID"
fi
echo "Network: $STELLAR_NETWORK"
echo "Source identity: $SOROBAN_SOURCE_ACCOUNT"
echo "Admin: $ADMIN_ADDRESS"
echo "Prover: $PROVER_ADDRESS"
echo ""

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

DEPLOYED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
WASM_SHA256="$(sha256sum "$WASM_PATH" | awk '{print $1}')"

echo ""
echo "Deployed: $CONTRACT_ID"
echo "Wrote $ROOT_DIR/.contract-address (gitignored)"

if [[ "${UPDATE_DEPLOYMENT_JSON:-}" == "1" ]]; then
  mkdir -p "$DEPLOYMENTS_DIR"
  RPC_URL="https://rpc-futurenet.stellar.org:443"
  PASSPHRASE="Test SDF Future Network ; October 2022"
  if [[ "$STELLAR_NETWORK" == "testnet" ]]; then
    RPC_URL="https://soroban-testnet.stellar.org"
    PASSPHRASE="Test SDF Network ; September 2015"
  fi

  cat > "$DEPLOYMENTS_DIR/${STELLAR_NETWORK}.json" <<EOF
{
  "network": "$STELLAR_NETWORK",
  "rpcUrl": "$RPC_URL",
  "networkPassphrase": "$PASSPHRASE",
  "contractId": "$CONTRACT_ID",
  "wasmPath": "target/wasm32v1-none/release/stellar_identity_core.wasm",
  "wasmSha256": "$WASM_SHA256",
  "deployedAt": "$DEPLOYED_AT",
  "admin": "$ADMIN_ADDRESS",
  "prover": "$PROVER_ADDRESS",
  "packageVersion": "0.1.0",
  "previousContractId": "${PREVIOUS_ID:-null}",
  "notes": "Intentional release deploy; see deployments/README.md"
}
EOF
  echo "Wrote $DEPLOYMENTS_DIR/${STELLAR_NETWORK}.json — commit only if releasing a new pinned ID for everyone."
else
  echo ""
  echo "deployments/${STELLAR_NETWORK}.json was NOT updated (team pin unchanged)."
  echo "E2E scripts still use the pinned ID in that file."
  echo "To pin this deploy for the repo: UPDATE_DEPLOYMENT_JSON=1 CONFIRM_DEPLOY=1 ./scripts/deploy-contract.sh"
  echo "Or test this instance only: CONTRACT_ID=$CONTRACT_ID make e2e-attested"
fi
