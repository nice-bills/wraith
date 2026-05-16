#!/usr/bin/env bash
# Prove layout circuit from passport JSON claims → stellar-payload.json (RarimoQuery path).
# Uses pre-built zkey from: make setup-passport
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ZK="$ROOT_DIR/tools/zk-circuits"
BUILD="$ZK/build/passport-layout"
PASSPORT_JSON="${1:?passport.json}"
OUT_DIR="${2:-$ROOT_DIR/passport-data/runs/$(date +%Y%m%d-%H%M%S)}"
PTAU="${ZK}/build/circuits_ptau10_final.ptau"
CURRENT_DATE="${CURRENT_DATE_YMD:-$(date -u +%y%m%d)}"

mkdir -p "$OUT_DIR"

CLAIMS_JSON="$(node "$ROOT_DIR/scripts/lib/extract-passport-claims.mjs" "$PASSPORT_JSON")"
BIRTH="$(echo "$CLAIMS_JSON" | jq -r .birthYymmdd)"
COUNTRY="$(echo "$CLAIMS_JSON" | jq -r .countryCode)"

echo "=== Passport layout prove ==="
echo "Passport:  $PASSPORT_JSON"
echo "Output:    $OUT_DIR"
echo "birthDate: $BIRTH  country: $COUNTRY  currentDate: $CURRENT_DATE"

[[ -f "$BUILD/passport_layout_final.zkey" ]] || {
  echo "error: missing layout zkey. Run: make setup-passport"
  exit 1
}

if [[ ! -f "$BUILD/rarimo_layout_stub_js/rarimo_layout_stub.wasm" ]]; then
  circom "$ZK/circuits/rarimo_layout_stub.circom" --r1cs --wasm --sym -o "$BUILD"
fi

cat > "$OUT_DIR/input.json" <<EOF
{"nullifier":"1","birth_date":"$BIRTH","expiration_date":"300101","pad3":"0","pad4":"0","nationality":"$COUNTRY"}
EOF

snarkjs groth16 fullprove "$OUT_DIR/input.json" \
  "$BUILD/rarimo_layout_stub_js/rarimo_layout_stub.wasm" \
  "$BUILD/passport_layout_final.zkey" \
  "$OUT_DIR/proof.json" "$OUT_DIR/public.json"

cp "$BUILD/verification_key.json" "$OUT_DIR/verification_key.json" 2>/dev/null || \
  snarkjs zkey export verificationkey "$BUILD/passport_layout_final.zkey" "$OUT_DIR/verification_key.json"

PAYLOAD="$OUT_DIR/stellar-payload.json"
cargo run -q -p proof-adapter -- \
  --rarimo-mode --current-date "$CURRENT_DATE" \
  --proof "$OUT_DIR/proof.json" \
  --vk "$OUT_DIR/verification_key.json" \
  --public "$OUT_DIR/public.json" \
  --out "$PAYLOAD"

if [[ "$(readlink -f "$PASSPORT_JSON")" != "$(readlink -f "$OUT_DIR/passport.json")" ]]; then
  cp "$PASSPORT_JSON" "$OUT_DIR/passport.json"
fi
echo "$CLAIMS_JSON" | jq . > "$OUT_DIR/claims-derived.json"
echo "PAYLOAD=$PAYLOAD"
echo "=== Done ==="
