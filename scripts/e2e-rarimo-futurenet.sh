#!/usr/bin/env bash
# Rarimo-layout Groth16 E2E: 6 public signals (birthDate@[1], nationality@[5]) on Futurenet.
# Uses rarimo_layout_stub until a full Rarimo query zkey is built and pinned in deployments/circuits.json.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ZK="$ROOT_DIR/tools/zk-circuits"
BUILD="$ZK/build/rarimo-layout-e2e"
: "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT}"

PASSPORT_FIXTURE="${PASSPORT_FIXTURE:-$ZK/fixtures/passport.layout.json}"
[[ -f "$ZK/build/passport-layout/passport_layout_final.zkey" ]] || {
  echo "Missing layout zkey. Run: make setup-passport"
  exit 1
}

mkdir -p "$BUILD"
echo "=== Rarimo-layout Groth16 E2E ==="
echo "Contract: $(jq -r .contractId "$ROOT_DIR/deployments/futurenet.json")"

"$ROOT_DIR/scripts/passport-prove-layout.sh" "$PASSPORT_FIXTURE" "$BUILD" | tee "$BUILD/prove.log"
PAYLOAD="$(sed -n 's/^PAYLOAD=//p' "$BUILD/prove.log")"
[[ -n "$PAYLOAD" && -f "$PAYLOAD" ]] || { echo "error: prove step failed"; exit 1; }

"$ROOT_DIR/scripts/lib/stellar-submit-rarimo.sh" "$PAYLOAD" "e2erarm"
echo "=== Rarimo-layout E2E OK ==="
