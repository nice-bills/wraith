#!/usr/bin/env bash
# Compute on-chain VK hash (matches stellar-identity-core).
#
# Usage:
#   ./scripts/pin-vk-hash.sh path/to/stellar-payload.json
#   ./scripts/pin-vk-hash.sh path/to/snarkjs/verification_key.json
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INPUT="${1:?stellar-payload.json or verification_key.json}"

PAYLOAD="$(mktemp)"
trap 'rm -f "$PAYLOAD"' EXIT

if jq -e '.verification_key.alpha' "$INPUT" >/dev/null 2>&1; then
  cp "$INPUT" "$PAYLOAD"
else
  PROOF="${ROOT_DIR}/tools/proof-adapter/fixtures/proof.json"
  PUB="${ROOT_DIR}/tools/proof-adapter/fixtures/rarimo_query_public.json"
  cargo run -q -p proof-adapter -- \
    --proof "$PROOF" \
    --vk "$INPUT" \
    --public "$PUB" \
    --rarimo-mode --current-date 250101 \
    --out "$PAYLOAD"
fi

HASH="$(node "$ROOT_DIR/scripts/compute-vk-hash.mjs" "$PAYLOAD")"
echo "vkHash: $HASH"
echo ""
echo "Pin under deployments/circuits.json → production.queryPhase2.vkHash"
