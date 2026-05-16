#!/usr/bin/env bash
# Phase 2 scaffold: after process_passport, build query witness inputs and prove when zkeys exist.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RARIMO="$ROOT_DIR/tools/zk-circuits/rarimo"
PASSPORT_JSON="${1:?passport.json}"
OUT_DIR="${2:-$ROOT_DIR/passport-data/runs/full-$(date +%Y%m%d-%H%M%S)}"
IDENTITY="${IDENTITY_STATE_JSON:-$ROOT_DIR/tools/zk-circuits/fixtures/identity-state-mock.json}"

mkdir -p "$OUT_DIR"
BASENAME="$(basename "$PASSPORT_JSON")"
GENERATED="$RARIMO/test/inputs/generated"

echo "=== Rarimo full prove (Phase 2) ==="
echo "Passport: $PASSPORT_JSON"
echo "Identity: $IDENTITY (set IDENTITY_STATE_JSON for real SMT)"

# Ensure register inputs exist
if ! ls "$GENERATED"/*.json >/dev/null 2>&1; then
  echo "error: no files in $GENERATED — run passport-pipeline.sh first"
  exit 1
fi

LATEST_GEN="$(ls -t "$GENERATED"/*.json | head -1)"
echo "Using generated input: $LATEST_GEN"
cp "$LATEST_GEN" "$OUT_DIR/register-input.json"
cp "$IDENTITY" "$OUT_DIR/identity-state.json"

# Rarimo production build is heavy; document gate
QUERY_ZKEY="${RARIMO_QUERY_ZKEY:-}"
if [[ -z "$QUERY_ZKEY" || ! -f "$QUERY_ZKEY" ]]; then
  echo ""
  echo "Phase 2 steps remaining:"
  echo "  1. cd tools/zk-circuits/rarimo && pnpm run zkit-make && pnpm run zkit-compile"
  echo "  2. Trusted setup for register + queryIdentity zkeys (see docs/PRODUCTION_ZK_ROADMAP.md)"
  echo "  3. export RARIMO_QUERY_ZKEY=/path/to/query_final.zkey"
  echo "  4. snarkjs groth16 prove with witness from test/inputs/generated + identity state"
  echo "  5. proof-adapter --rarimo-mode → stellar-submit-rarimo.sh"
  echo ""
  echo "Falling back to layout prove for Futurenet smoke (not full passport crypto)..."
  "$ROOT_DIR/scripts/passport-prove-layout.sh" "$PASSPORT_JSON" "$OUT_DIR" | tee "$OUT_DIR/layout-fallback.log"
  sed -n 's/^PAYLOAD=//p' "$OUT_DIR/layout-fallback.log" | tail -1 | xargs -I{} echo "PAYLOAD={}"
  exit 0
fi

echo "Query zkey: $QUERY_ZKEY"
echo "TODO: wire witness + snarkjs fullprove for queryIdentity (track in PRODUCTION_ZK_ROADMAP)"
exit 1
