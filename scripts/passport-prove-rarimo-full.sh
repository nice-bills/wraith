#!/usr/bin/env bash
# Phase 2: full Rarimo queryIdentity Groth16 → stellar-payload.json
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RARIMO="$ROOT_DIR/tools/zk-circuits/rarimo"
PASSPORT_JSON="${1:?passport.json}"
OUT_DIR="${2:-$ROOT_DIR/passport-data/runs/full-$(date +%Y%m%d-%H%M%S)}"
BUILD="${RARIMO_QUERY_BUILD:-$ROOT_DIR/tools/zk-circuits/build/rarimo-query}"
QUERY_ZKEY="${RARIMO_QUERY_ZKEY:-$BUILD/queryIdentity_final.zkey}"
QUERY_WASM="${RARIMO_QUERY_WASM:-$BUILD/queryIdentity_js/queryIdentity.wasm}"
CURRENT_DATE="${CURRENT_DATE_YMD:-$(date -u +%y%m%d)}"
GENERATED="$RARIMO/test/inputs/generated"

mkdir -p "$OUT_DIR"

# shellcheck source=/dev/null
[[ -f "$BUILD/phase2.env" ]] && source "$BUILD/phase2.env"

echo "=== Rarimo full prove (Phase 2) ==="
echo "Passport: $PASSPORT_JSON"
echo "Out:      $OUT_DIR"

[[ -f "$QUERY_ZKEY" && -f "$QUERY_WASM" ]] || {
  echo "error: query zkey/wasm missing. Run: ./scripts/setup-rarimo-phase2.sh"
  exit 1
}

if ! ls "$GENERATED"/*.json >/dev/null 2>&1; then
  echo "error: no register inputs — run passport-pipeline.sh first"
  exit 1
fi

REGISTER_INPUT="$(ls -t "$GENERATED"/*.json | head -1)"
echo "Register input: $REGISTER_INPUT"
cp "$REGISTER_INPUT" "$OUT_DIR/register-input.json"

echo ""
echo "=== Register witness (public outputs) ==="
"$ROOT_DIR/scripts/passport-register-witness.sh" "$OUT_DIR/register-input.json" "$OUT_DIR"

echo ""
echo "=== Query witness input (identity SMT) ==="
node "$ROOT_DIR/scripts/lib/build-rarimo-query-input.mjs" \
  "$OUT_DIR/register-input.json" \
  "$OUT_DIR/register-public.json" \
  "$OUT_DIR/query-input.json"

echo ""
echo "=== Query Groth16 prove ==="
snarkjs groth16 fullprove \
  "$OUT_DIR/query-input.json" \
  "$QUERY_WASM" \
  "$QUERY_ZKEY" \
  "$OUT_DIR/proof.json" \
  "$OUT_DIR/public.json"

cp "$BUILD/verification_key.json" "$OUT_DIR/verification_key.json" 2>/dev/null || \
  snarkjs zkey export verificationkey "$QUERY_ZKEY" "$OUT_DIR/verification_key.json"

PAYLOAD="$OUT_DIR/stellar-payload.json"
cargo run -q -p proof-adapter -- \
  --rarimo-mode --current-date "$CURRENT_DATE" \
  --proof "$OUT_DIR/proof.json" \
  --vk "$OUT_DIR/verification_key.json" \
  --public "$OUT_DIR/public.json" \
  --out "$PAYLOAD"

echo "PAYLOAD=$PAYLOAD"
echo "=== Phase 2 prove complete ==="
