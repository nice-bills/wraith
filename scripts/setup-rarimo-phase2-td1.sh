#!/usr/bin/env bash
# TD1 query circuit (national ID / card) — same PTAU as TD3 Phase 2.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RARIMO="$ROOT_DIR/tools/zk-circuits/rarimo"
BUILD="$ROOT_DIR/tools/zk-circuits/build/rarimo-query-td1"
PTAU_DIR="$ROOT_DIR/tools/zk-circuits/build/ptau"
PATCHES="$ROOT_DIR/tools/zk-circuits/patches/rarimo"

need() { command -v "$1" >/dev/null || { echo "error: missing $1"; exit 1; }; }
need circom
need snarkjs

"$ROOT_DIR/scripts/setup-rarimo.sh"

echo "=== Applying Rarimo compile patches ==="
mkdir -p "$RARIMO/circuits/lib/circuits/babyjubjub"
cp "$PATCHES/babyPbk.circom" "$RARIMO/circuits/lib/circuits/babyjubjub/babyPbk.circom"
for f in identityStateVerifier.circom registerIdentityLight.circom; do
  if ! grep -q 'babyPbk.circom' "$RARIMO/circuits/identityManagement/$f" 2>/dev/null; then
    sed -i '/curve.circom/a include "../lib/circuits/babyjubjub/babyPbk.circom";' \
      "$RARIMO/circuits/identityManagement/$f"
  fi
done

mkdir -p "$BUILD" "$RARIMO/test/circuits"
cp "$PATCHES/queryIdentityTD1.circom" "$RARIMO/test/circuits/queryIdentityTD1.circom"

echo "=== Compiling queryIdentityTD1 (national ID / TD1) ==="
(cd "$RARIMO" && circom test/circuits/queryIdentityTD1.circom --r1cs --wasm -o "$BUILD")

PTAU="${PTAU_FILE:-$PTAU_DIR/powersOfTau28_hez_final_17.ptau}"
[[ -f "$PTAU" ]] || {
  echo "error: missing $PTAU — run make setup-rarimo-phase2 first (downloads PTAU)"
  exit 1
}

if [[ ! -f "$BUILD/queryIdentity_final.zkey" ]]; then
  echo "=== Groth16 setup for queryIdentityTD1 ==="
  snarkjs groth16 setup "$BUILD/queryIdentityTD1.r1cs" "$PTAU" "$BUILD/queryIdentity_0000.zkey"
  echo "wraith-query-td1" | snarkjs zkey contribute "$BUILD/queryIdentity_0000.zkey" \
    "$BUILD/queryIdentity_final.zkey" --name=wraith-query-td1 -v -e=wraith-query-td1
  snarkjs zkey export verificationkey "$BUILD/queryIdentity_final.zkey" "$BUILD/verification_key.json"
fi

cat > "$BUILD/phase2-td1.env" <<EOF
export RARIMO_DOC_TYPE=td1
export RARIMO_QUERY_ZKEY=$BUILD/queryIdentity_final.zkey
export RARIMO_QUERY_WASM=$BUILD/queryIdentityTD1_js/queryIdentity.wasm
export RARIMO_QUERY_BUILD=$BUILD
EOF

echo ""
echo "=== TD1 query zkey ready (national ID) ==="
echo "  source $BUILD/phase2-td1.env"
echo "  ./scripts/passport-ready-full.sh passport-data/my-id.json"
