#!/usr/bin/env bash
# One-time prep so passport day is: validate → process → prove → submit.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ZK="$ROOT_DIR/tools/zk-circuits"
BUILD_LAYOUT="$ZK/build/passport-layout"
PTAU="${ZK}/build/circuits_ptau10_final.ptau"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: missing $1 ($2)"
    exit 1
  }
}

echo "=== Wraith passport stack setup ==="
need circom "https://docs.circom.io/getting-started/installation/"
need snarkjs "pnpm add -g snarkjs"
need node "https://nodejs.org"
need jq "package manager"
need cargo "rustup"
need stellar-cli "https://developers.stellar.org/docs/tools/cli"

"$ROOT_DIR/scripts/setup-rarimo.sh"

mkdir -p "$ZK/build" "$ROOT_DIR/passport-data"
if [[ ! -f "$PTAU" ]]; then
  echo "Downloading dev PTAU (powersOfTau10)..."
  (cd "$ZK" && bash setup-trusted-setup.sh)
fi
[[ -f "$PTAU" ]] || { echo "error: PTAU missing at $PTAU"; exit 1; }

echo "Pre-building layout circuit zkey (RarimoQuery pipeline smoke)..."
mkdir -p "$BUILD_LAYOUT"
if [[ ! -f "$BUILD_LAYOUT/passport_layout_final.zkey" ]]; then
  circom "$ZK/circuits/rarimo_layout_stub.circom" --r1cs --wasm --sym -o "$BUILD_LAYOUT"
  if [[ -f "$PTAU" ]]; then
    snarkjs groth16 setup "$BUILD_LAYOUT/rarimo_layout_stub.r1cs" "$PTAU" \
      "$BUILD_LAYOUT/passport_layout_0000.zkey"
    snarkjs zkey contribute "$BUILD_LAYOUT/passport_layout_0000.zkey" \
      "$BUILD_LAYOUT/passport_layout_final.zkey" --name=wraith-passport-layout -v -e=wraith
    snarkjs zkey export verificationkey "$BUILD_LAYOUT/passport_layout_final.zkey" \
      "$BUILD_LAYOUT/verification_key.json"
  else
    echo "⚠ PTAU missing; layout zkey not built. Run: cd tools/zk-circuits && bash setup-trusted-setup.sh"
  fi
else
  echo "✓ layout zkey already present"
fi

echo ""
echo "=== Optional: Phase 2 query zkey (full Rarimo, ~144MB PTAU) ==="
echo "  make setup-rarimo-phase2"
echo ""
echo "=== Ready ==="
echo "Put passport JSON in:  passport-data/  (gitignored)"
echo "When you have a file:"
echo "  ./scripts/passport-ready-layout.sh tools/zk-circuits/fixtures/passport.layout.json"
echo "  ./scripts/passport-ready-full.sh passport-data/my-passport.json [--submit]"
echo ""
echo "See docs/PASSPORT_PLAYBOOK.md"
