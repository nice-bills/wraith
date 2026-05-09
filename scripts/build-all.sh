#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"
echo "[1/4] Running Rust tests"
cargo test

echo "[2/4] Building contract WASM (wasm32v1-none)"
cargo build -p stellar-identity-core --target wasm32v1-none

echo "[3/4] Building SDK"
cd "$ROOT_DIR/sdk/stellar-identity-sdk"
npm ci --silent
npm run build --silent

echo "[4/4] Build complete"
echo "Contract WASM: $ROOT_DIR/target/wasm32v1-none/debug/stellar_identity_core.wasm"
echo "SDK dist:      $ROOT_DIR/sdk/stellar-identity-sdk/dist"

