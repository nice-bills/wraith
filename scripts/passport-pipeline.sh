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
  echo "Or run full flow: ./scripts/passport-ready.sh passport-data/my-passport.json"
  echo "Template: tools/zk-circuits/fixtures/passport.template.json"
  exit 1
fi

if [[ ! -d "$RARIMO" ]]; then
  echo "error: rarimo missing. Run: make setup-passport"
  exit 1
fi

node "$ROOT_DIR/scripts/validate-passport-json.mjs" --mode full "$PASSPORT_JSON"

if [[ ! -f "$PASSPORT_JSON" ]]; then
  echo "error: file not found: $PASSPORT_JSON"
  exit 1
fi

DEST="$RARIMO/test/inputs/passport/$(basename "$PASSPORT_JSON")"
mkdir -p "$(dirname "$DEST")"
cp "$PASSPORT_JSON" "$DEST"
echo "Copied to $DEST"

echo "Generating Rarimo register inputs..."
BASENAME="$(basename "$PASSPORT_JSON")"
cd "$RARIMO"
node -e "
const { processPassport } = require('./test/process_passport.js');
const name = processPassport('test/inputs/passport/${BASENAME}');
console.log('Circuit name:', name);
"

echo ""
echo "Generated: $RARIMO/test/inputs/generated/"
echo "Next: ./scripts/passport-prove-layout.sh $PASSPORT_JSON"
echo "  or:  ./scripts/passport-ready.sh $PASSPORT_JSON [--submit]"
