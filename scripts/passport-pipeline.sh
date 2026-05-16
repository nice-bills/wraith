#!/usr/bin/env bash
# Prepare Rarimo circuit inputs from passport JSON (JMRTD / manual export).
# Does not submit on-chain. See docs/PRODUCTION_ZK_ROADMAP.md.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RARIMO="$ROOT_DIR/tools/zk-circuits/rarimo"
PASSPORT_JSON="${1:-}"

if [[ -z "$PASSPORT_JSON" ]]; then
  echo "Usage: $0 <passport.json>"
  echo ""
  echo "Place passport JSON under: $RARIMO/test/inputs/passport/"
  echo "Generate with JMRTD (see docs/PRODUCTION_RUNBOOK.md) or Rarimo test vectors."
  exit 1
fi

if [[ ! -f "$PASSPORT_JSON" ]]; then
  echo "error: file not found: $PASSPORT_JSON"
  exit 1
fi

if [[ ! -d "$RARIMO" ]]; then
  echo "error: rarimo submodule missing. Run: cd tools/zk-circuits && pnpm run setup:rarimo"
  exit 1
fi

DEST="$RARIMO/test/inputs/passport/$(basename "$PASSPORT_JSON")"
mkdir -p "$(dirname "$DEST")"
cp "$PASSPORT_JSON" "$DEST"
echo "Copied to $DEST"

echo "Generating Rarimo register inputs..."
cd "$RARIMO"
node test/process_passport.js "test/inputs/passport/$(basename "$PASSPORT_JSON")"

echo ""
echo "Generated files under $RARIMO/test/inputs/generated/ and circuits/generated/"
echo "Next: build Rarimo circuits (pnpm run build:production) and prove query circuit."
echo "Then: proof-adapter --rarimo-mode --current-date YYMMDD ..."
