#!/usr/bin/env bash
# Passport entrypoint — pick Path A (layout) or Path B (full Rarimo).
#
#   ./scripts/passport-ready.sh --layout <json> [--submit]   # demo / Futurenet RarimoQuery
#   ./scripts/passport-ready.sh --full <json> [--submit]     # after NFC scan
#   ./scripts/passport-ready.sh <json>                     # same as --full if sod+dg1, else --layout
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE=""
PASSPORT_JSON=""
SUBMIT_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --layout) MODE=layout; shift ;;
    --full) MODE=full; shift ;;
    --submit) SUBMIT_ARGS+=(--submit); shift ;;
    -*) echo "Unknown option: $1"; exit 1 ;;
    *)
      if [[ -z "$PASSPORT_JSON" ]]; then
        PASSPORT_JSON="$1"
      else
        echo "Unexpected argument: $1"
        exit 1
      fi
      shift
      ;;
  esac
done

if [[ -z "$PASSPORT_JSON" ]]; then
  cat <<EOF
Usage:
  $0 --layout <passport.json> [--submit]   Path A: claims only (demo)
  $0 --full <passport.json> [--submit]     Path B: real sod/dg1 from NFC scan

Scan guide: docs/PASSPORT_SCAN.md
Demo:       tools/zk-circuits/fixtures/passport.layout.json
First time: make setup-passport
EOF
  exit 1
fi

if [[ -z "$MODE" ]]; then
  if node "$ROOT_DIR/scripts/validate-passport-json.mjs" --mode full "$PASSPORT_JSON" 2>/dev/null; then
    MODE=full
  else
    MODE=layout
    echo "(auto-selected --layout; use --full after NFC scan)"
  fi
fi

case "$MODE" in
  layout) exec "$ROOT_DIR/scripts/passport-ready-layout.sh" "$PASSPORT_JSON" "${SUBMIT_ARGS[@]}" ;;
  full) exec "$ROOT_DIR/scripts/passport-ready-full.sh" "$PASSPORT_JSON" "${SUBMIT_ARGS[@]}" ;;
  *) echo "mode must be layout or full"; exit 1 ;;
esac
