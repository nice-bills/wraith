#!/usr/bin/env bash
# Run when you have passport JSON: validate → Rarimo register inputs → layout prove → optional Futurenet.
#
# Usage:
#   ./scripts/passport-ready.sh passport-data/my-passport.json
#   ./scripts/passport-ready.sh passport-data/my-passport.json --submit
#
# First time only: make setup-passport
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PASSPORT_JSON="${1:-}"
SUBMIT=0
shift || true
for arg in "$@"; do
  case "$arg" in
    --submit) SUBMIT=1 ;;
    *) echo "Unknown option: $arg"; exit 1 ;;
  esac
done

if [[ -z "$PASSPORT_JSON" || ! -f "$PASSPORT_JSON" ]]; then
  echo "Usage: $0 <passport.json> [--submit]"
  echo ""
  echo "First time: make setup-passport"
  echo "Template:   tools/zk-circuits/fixtures/passport.template.json"
  echo "Drop files: passport-data/  (gitignored)"
  exit 1
fi

if [[ ! -d "$ROOT_DIR/tools/zk-circuits/rarimo" ]]; then
  echo "error: rarimo not installed. Run: make setup-passport"
  exit 1
fi

RUN_DIR="$ROOT_DIR/passport-data/runs/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN_DIR"
cp "$PASSPORT_JSON" "$RUN_DIR/passport.json"
PASSPORT_JSON="$RUN_DIR/passport.json"

echo "=== 1/4 Validate passport JSON ==="
node "$ROOT_DIR/scripts/validate-passport-json.mjs" "$PASSPORT_JSON"

echo ""
echo "=== 2/4 Rarimo register inputs (process_passport) ==="
if "$ROOT_DIR/scripts/passport-pipeline.sh" "$PASSPORT_JSON"; then
  echo "✓ Generated under tools/zk-circuits/rarimo/test/inputs/generated/"
  echo "  (Full register/query Groth16: build rarimo circuits — see PASSPORT_PLAYBOOK.md Phase 2)"
else
  echo "⚠ process_passport failed (often ASN.1 / SOD). Layout path may still work if dateOfBirth + country are set."
fi

echo ""
echo "=== 3/4 Layout prove → stellar payload ==="
PROVE_LOG="$RUN_DIR/prove.log"
"$ROOT_DIR/scripts/passport-prove-layout.sh" "$PASSPORT_JSON" "$RUN_DIR" | tee "$PROVE_LOG"
PAYLOAD="$(sed -n 's/^PAYLOAD=//p' "$PROVE_LOG")"
[[ -n "$PAYLOAD" && -f "$PAYLOAD" ]] || { echo "error: layout prove did not produce payload"; exit 1; }

echo ""
echo "Payload: $PAYLOAD"
echo "Run dir: $RUN_DIR"

if [[ "$SUBMIT" == "1" ]]; then
  echo ""
  echo "=== 4/4 Submit Futurenet (RarimoQuery) ==="
  : "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT for --submit}"
  "$ROOT_DIR/scripts/lib/stellar-submit-rarimo.sh" "$PAYLOAD" "passport"
else
  echo ""
  echo "=== 4/4 Skipped on-chain (pass --submit to verify on Futurenet) ==="
  echo "  export SOROBAN_SOURCE_ACCOUNT=bills-futurenet"
  echo "  ./scripts/lib/stellar-submit-rarimo.sh $PAYLOAD passport"
fi

echo ""
echo "=== passport-ready complete ==="
