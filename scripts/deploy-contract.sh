#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WASM_PATH="$ROOT_DIR/target/wasm32v1-none/release/stellar_identity_core.wasm"

: "${SOROBAN_RPC_URL:?SOROBAN_RPC_URL is required}"
: "${SOROBAN_NETWORK_PASSPHRASE:?SOROBAN_NETWORK_PASSPHRASE is required}"
: "${SOROBAN_SOURCE_ACCOUNT:?SOROBAN_SOURCE_ACCOUNT is required (identity alias or address)}"

if ! command -v soroban >/dev/null 2>&1; then
  echo "error: soroban CLI not found"
  echo "install: https://developers.stellar.org/docs/tools/soroban-cli"
  exit 1
fi

cd "$ROOT_DIR"

if [[ ! -f "$WASM_PATH" ]]; then
  echo "WASM artifact not found. Building..."
  cargo build -p stellar-identity-core --target wasm32v1-none --release
fi

echo "Deploying contract..."
CONTRACT_ID="$(
  soroban contract deploy \
    --wasm "$WASM_PATH" \
    --source-account "$SOROBAN_SOURCE_ACCOUNT" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE"
)"

echo "Contract deployed:"
echo "$CONTRACT_ID"

