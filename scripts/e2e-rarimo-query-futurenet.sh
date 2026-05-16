#!/usr/bin/env bash
# Full Phase 2 E2E: scanned passport JSON → query Groth16 → verify_and_record (Futurenet).
#
# Requires:
#   SOROBAN_SOURCE_ACCOUNT
#   PASSPORT_JSON (real sod/dg1 from docs/PASSPORT_SCAN.md)
#   make setup-rarimo-phase2 (query zkey)
#
# Usage:
#   export SOROBAN_SOURCE_ACCOUNT=bills-futurenet
#   export PASSPORT_JSON=passport-data/my-passport.json
#   ./scripts/e2e-rarimo-query-futurenet.sh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
: "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT}"
: "${PASSPORT_JSON:?Set PASSPORT_JSON to scanned passport JSON}"

[[ -f "$ROOT_DIR/tools/zk-circuits/build/rarimo-query/queryIdentity_final.zkey" ]] || {
  echo "error: run make setup-rarimo-phase2 first"
  exit 1
}

# shellcheck source=/dev/null
source "$ROOT_DIR/tools/zk-circuits/build/rarimo-query/phase2.env"

RUN_DIR="$ROOT_DIR/passport-data/runs/e2e-query-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN_DIR"

echo "=== Full Rarimo query E2E (Futurenet) ==="
echo "Passport: $PASSPORT_JSON"
echo "Contract: $(jq -r .contractId "$ROOT_DIR/deployments/futurenet.json")"

"$ROOT_DIR/scripts/passport-ready-full.sh" "$PASSPORT_JSON" --submit 2>&1 | tee "$RUN_DIR/full.log"

PAYLOAD="$(sed -n 's/^PAYLOAD=//p' "$RUN_DIR/full.log" | tail -1)"
[[ -n "$PAYLOAD" && -f "$PAYLOAD" ]] || { echo "error: no payload produced"; exit 1; }

echo ""
echo "=== E2E query path OK ==="
echo "Payload: $PAYLOAD"
echo "Run dir: $RUN_DIR"
