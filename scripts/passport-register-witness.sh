#!/usr/bin/env bash
# Compile per-passport register circuit and export public outputs (for query witness).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RARIMO="$ROOT_DIR/tools/zk-circuits/rarimo"
REGISTER_JSON="${1:?register-input.json from process_passport}"
OUT_DIR="${2:-$(dirname "$REGISTER_JSON")}"

NAME="$(basename "$REGISTER_JSON" .json)"
CIRCOM="$RARIMO/test/circuits/generated/${NAME}.circom"
BUILD="$ROOT_DIR/tools/zk-circuits/build/rarimo-register/${NAME}"
SYM="$BUILD/${NAME}.sym"

[[ -f "$CIRCOM" ]] || {
  echo "error: missing $CIRCOM — run passport-pipeline.sh first"
  exit 1
}

mkdir -p "$BUILD" "$OUT_DIR"
if [[ ! -f "$BUILD/${NAME}_js/${NAME}.wasm" ]]; then
  echo "Compiling register circuit $NAME (may take several minutes)..."
  (cd "$RARIMO" && circom "test/circuits/generated/${NAME}.circom" --r1cs --wasm --sym -o "$BUILD")
fi

WASM="$BUILD/${NAME}_js/${NAME}.wasm"
GEN_WITNESS="$BUILD/${NAME}_js/generate_witness.js"
[[ -f "$WASM" ]] || { echo "error: wasm not found at $WASM"; exit 1; }
[[ -f "$SYM" ]] || { echo "error: sym not found at $SYM"; exit 1; }

WTNS="$OUT_DIR/register.wtns"
node "$GEN_WITNESS" "$WASM" "$REGISTER_JSON" "$WTNS"
snarkjs wtns export json "$WTNS" "$OUT_DIR/register-witness.json"

node <<NODE
const fs = require("fs");
const sym = fs.readFileSync("$SYM", "utf8").split("\n");
const witness = require("$OUT_DIR/register-witness.json");
const outputs = [
  "main.dg15PubKeyHash",
  "main.passportHash",
  "main.dg1Commitment",
  "main.pkIdentityHash",
];
const pub = outputs.map((name) => {
  const line = sym.find((l) => l.endsWith("," + name) || l.includes("," + name));
  if (!line) throw new Error("signal not in sym: " + name);
  const idx = line.split(",")[0];
  return witness[idx];
});
fs.writeFileSync("$OUT_DIR/register-public.json", JSON.stringify(pub));
console.log("register-public:", pub.map((v, i) => outputs[i] + "=" + String(v).slice(0, 20) + "...").join(" "));
NODE

echo "Wrote $OUT_DIR/register-public.json"
