#!/usr/bin/env bash
# Path A — Layout / RarimoQuery demo on Futurenet (no chip crypto).
# Needs: dateOfBirth + nationality or country_code only.
#
# Usage:
#   ./scripts/passport-ready-layout.sh passport-data/my.json [--submit]
#   ./scripts/passport-ready-layout.sh tools/zk-circuits/fixtures/passport.layout.json
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
  echo "Demo fixture: tools/zk-circuits/fixtures/passport.layout.json"
  echo "First time:   make setup-passport"
  exit 1
fi

[[ -f "$ROOT_DIR/tools/zk-circuits/build/passport-layout/passport_layout_final.zkey" ]] || {
  echo "error: run make setup-passport first"
  exit 1
}

RUN_DIR="$ROOT_DIR/passport-data/runs/layout-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN_DIR"
cp "$PASSPORT_JSON" "$RUN_DIR/passport.json"
PASSPORT_JSON="$RUN_DIR/passport.json"

echo "=== Path A: Layout (RarimoQuery) ==="
echo "Note: This does NOT prove passport cryptography — contract layout demo only."
echo ""

node "$ROOT_DIR/scripts/validate-passport-json.mjs" --mode layout "$PASSPORT_JSON"

PROVE_LOG="$RUN_DIR/prove.log"
"$ROOT_DIR/scripts/passport-prove-layout.sh" "$PASSPORT_JSON" "$RUN_DIR" | tee "$PROVE_LOG"
PAYLOAD="$(sed -n 's/^PAYLOAD=//p' "$PROVE_LOG")"
[[ -n "$PAYLOAD" && -f "$PAYLOAD" ]] || { echo "error: layout prove failed"; exit 1; }

echo ""
echo "Payload: $PAYLOAD"
echo "Run dir: $RUN_DIR"

if [[ "$SUBMIT" == "1" ]]; then
  : "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT for --submit}"
  "$ROOT_DIR/scripts/lib/stellar-submit-rarimo.sh" "$PAYLOAD" "layout"
else
  echo ""
  echo "Submit: export SOROBAN_SOURCE_ACCOUNT=... && \\"
  echo "  ./scripts/lib/stellar-submit-rarimo.sh $PAYLOAD layout"
fi

echo "=== layout path complete ==="
