#!/usr/bin/env bash
# One-time: Rarimo queryIdentity compile + dev zkey (Phase 2 Groth16).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RARIMO="$ROOT_DIR/tools/zk-circuits/rarimo"
BUILD="$ROOT_DIR/tools/zk-circuits/build/rarimo-query"
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

mkdir -p "$RARIMO/test/circuits"
QUERY_WRAPPER="$RARIMO/test/circuits/queryIdentity.circom"
if [[ -f "$PATCHES/queryIdentity.circom" ]]; then
  cp "$PATCHES/queryIdentity.circom" "$QUERY_WRAPPER"
elif [[ ! -f "$QUERY_WRAPPER" ]]; then
  cat > "$QUERY_WRAPPER" << 'EOF'
pragma circom  2.1.6;
include "../../circuits/identityManagement/queryIdentity.circom";
component main { public [eventID, eventData, idStateRoot, selector, currentDate,
  timestampLowerbound, timestampUpperbound, identityCounterLowerbound, identityCounterUpperbound,
  birthDateLowerbound, birthDateUpperbound, expirationDateLowerbound, expirationDateUpperbound,
  citizenshipMask ] } = QueryIdentity(80);
EOF
fi

mkdir -p "$BUILD" "$PTAU_DIR"
echo "=== Compiling queryIdentity (TD3, depth 80) ==="
(cd "$RARIMO" && circom test/circuits/queryIdentity.circom --r1cs --wasm -o "$BUILD")

PTAU="${PTAU_FILE:-$PTAU_DIR/powersOfTau28_hez_final_17.ptau}"
if [[ ! -f "$PTAU" ]]; then
  echo "Downloading powersOfTau28_hez_final_17.ptau (~144MB)..."
  curl -L --fail -o "$PTAU" \
    "https://storage.googleapis.com/zkevm/ptau/powersOfTau28_hez_final_17.ptau" || {
    echo "Generating dev powersOfTau 17 locally..."
    PTAU="$PTAU_DIR/powersOfTau17_final.ptau"
    snarkjs powersoftau new bn128 17 "$PTAU_DIR/ptau17_0000.ptau" -v
    echo "wraith-dev" | snarkjs powersoftau contribute "$PTAU_DIR/ptau17_0000.ptau" \
      "$PTAU_DIR/ptau17_0001.ptau" --name=wraith-dev -v -e=wraith-dev-phase2
    snarkjs powersoftau prepare phase2 "$PTAU_DIR/ptau17_0001.ptau" "$PTAU" -v
  }
fi

if [[ ! -f "$BUILD/queryIdentity_final.zkey" ]]; then
  echo "=== Groth16 setup for queryIdentity ==="
  snarkjs groth16 setup "$BUILD/queryIdentity.r1cs" "$PTAU" "$BUILD/queryIdentity_0000.zkey"
  echo "wraith-query" | snarkjs zkey contribute "$BUILD/queryIdentity_0000.zkey" \
    "$BUILD/queryIdentity_final.zkey" --name=wraith-query -v -e=wraith-query
  snarkjs zkey export verificationkey "$BUILD/queryIdentity_final.zkey" "$BUILD/verification_key.json"
fi

cat > "$BUILD/phase2.env" <<EOF
export RARIMO_QUERY_ZKEY=$BUILD/queryIdentity_final.zkey
export RARIMO_QUERY_WASM=$BUILD/queryIdentity_js/queryIdentity.wasm
export RARIMO_QUERY_BUILD=$BUILD
EOF

echo ""
echo "=== Phase 2 query zkey ready ==="
echo "  source $BUILD/phase2.env"
echo "  Then: ./scripts/passport-ready-full.sh passport-data/my-passport.json"
