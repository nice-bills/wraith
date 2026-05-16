#!/usr/bin/env bash
# Path B — Full Rarimo pipeline (chip sod/dg1 → register inputs → query prove scaffold).
#
# Usage:
#   ./scripts/passport-ready-full.sh passport-data/my-passport.json [--submit]
#
# Prerequisites: NFC scan → JSON with real sod + dg1 (docs/PASSPORT_SCAN.md)
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
  echo "Scan first: docs/PASSPORT_SCAN.md"
  echo "Normalize:  node scripts/normalize-passport-json.mjs raw.json passport-data/my-passport.json"
  exit 1
fi

if [[ ! -d "$ROOT_DIR/tools/zk-circuits/rarimo" ]]; then
  echo "error: run make setup-passport"
  exit 1
fi

RUN_DIR="$ROOT_DIR/passport-data/runs/full-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN_DIR"
cp "$PASSPORT_JSON" "$RUN_DIR/passport.json"
PASSPORT_JSON="$RUN_DIR/passport.json"

echo "=== Path B: Full Rarimo ==="
echo ""

node "$ROOT_DIR/scripts/validate-passport-json.mjs" --mode full "$PASSPORT_JSON"

echo ""
echo "=== Register circuit inputs (process_passport) ==="
"$ROOT_DIR/scripts/passport-pipeline.sh" "$PASSPORT_JSON"

echo ""
echo "=== Query prove (Phase 2 scaffold) ==="
FULL_LOG="$RUN_DIR/full-prove.log"
if "$ROOT_DIR/scripts/passport-prove-rarimo-full.sh" "$PASSPORT_JSON" "$RUN_DIR" | tee "$FULL_LOG"; then
  PAYLOAD="$(sed -n 's/^PAYLOAD=//p' "$FULL_LOG" | tail -1)"
else
  echo ""
  echo "⚠ Full query prove not complete yet — register inputs are under rarimo/test/inputs/generated/"
  echo "  Continue with docs/PASSPORT_PLAYBOOK.md Phase 2 (build zkeys, identity SMT)."
  PAYLOAD="$(sed -n 's/^PAYLOAD=//p' "$RUN_DIR/layout-fallback.log" 2>/dev/null | tail -1 || true)"
fi

if [[ -n "${PAYLOAD:-}" && -f "$PAYLOAD" && "$SUBMIT" == "1" ]]; then
  : "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT}"
  "$ROOT_DIR/scripts/lib/stellar-submit-rarimo.sh" "$PAYLOAD" "passport"
elif [[ "$SUBMIT" == "1" ]]; then
  echo "error: no payload to submit — finish Phase 2 prove first"
  exit 1
fi

echo "=== full path complete (run: $RUN_DIR) ==="
