#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_FILE="${1:-/tmp/stellar-proof-payload.json}"

cd "$ROOT_DIR"

echo "Generating adapter payload from fixtures..."
cargo run -p proof-adapter -- \
  --proof "$ROOT_DIR/tools/proof-adapter/fixtures/proof.json" \
  --vk "$ROOT_DIR/tools/proof-adapter/fixtures/verification_key.json" \
  --public "$ROOT_DIR/tools/proof-adapter/fixtures/public.json" \
  --age-index 0 \
  --country-index 1 \
  --humanity-index 2 \
  --out "$OUT_FILE"

echo "Adapter output written to: $OUT_FILE"
echo "You can now pass this payload into your app/backend to call verify_and_record or record_attested_result."

