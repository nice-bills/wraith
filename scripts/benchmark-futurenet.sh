#!/usr/bin/env bash
# Simulate and/or report fees for Wraith contract methods on Futurenet.
#
# Usage:
#   export SOROBAN_SOURCE_ACCOUNT=bills-futurenet
#   ./scripts/benchmark-futurenet.sh
#
# After running e2e scripts, pass tx hashes for write-path fees:
#   TX_REGISTER_APP=<hash> TX_VERIFY_GROTH16=<hash> TX_RECORD_ATTESTED=<hash> \
#     ./scripts/benchmark-futurenet.sh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
: "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT}"
: "${STELLAR_NETWORK:=futurenet}"

CONTRACT_ID="${CONTRACT_ID:-$(jq -r .contractId "$ROOT_DIR/deployments/futurenet.json")}"
SUBJECT="$(stellar-cli keys address "$SOROBAN_SOURCE_ACCOUNT")"
OUT_DIR="$ROOT_DIR/deployments/benchmarks"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_FILE="$OUT_DIR/futurenet-$STAMP.json"

mkdir -p "$OUT_DIR"

invoke_sim() {
  local name="$1"
  shift
  echo "  simulate: $name"
  if ! stellar-cli contract invoke \
    --id "$CONTRACT_ID" \
    --source-account "$SOROBAN_SOURCE_ACCOUNT" \
    --network "$STELLAR_NETWORK" \
    --send=no \
    "$@" >/dev/null 2>&1; then
    echo "  warning: simulation failed for $name (non-fatal)" >&2
  fi
}

fetch_fee() {
  local name="$1"
  local hash="$2"
  if [[ -z "$hash" ]]; then
    echo "  skip fee $name (no TX_* hash)" >&2
    return 0
  fi
  echo "  fee: $name ($hash)" >&2
  local raw
  raw="$(stellar-cli tx fetch fee --hash "$hash" --network "$STELLAR_NETWORK" --output json 2>/dev/null)" || return 0
  echo "$raw" | jq -c --arg op "$name" --arg hash "$hash" '{operation: $op, tx_hash: $hash, fee: .}'
}

echo "=== Wraith Futurenet benchmark ==="
echo "Contract: $CONTRACT_ID"
echo "Subject:  $SUBJECT"
echo ""

echo "1. Read-path simulation (no fee; confirms RPC + auth layout)"
invoke_sim get_admin -- get_admin
invoke_sim get_prover -- get_prover
invoke_sim is_verified -- is_verified --app_id "__none__" --subject "$SUBJECT" || true

echo ""
echo "2. Optional live read tx (charges fee; captures hash for get_admin)"
LIVE_READ="${BENCHMARK_LIVE_READ:-0}"
READ_HASH=""
if [[ "$LIVE_READ" == "1" ]]; then
  OUT="$(stellar-cli contract invoke \
    --id "$CONTRACT_ID" \
    --source-account "$SOROBAN_SOURCE_ACCOUNT" \
    --network "$STELLAR_NETWORK" \
    --send=yes \
    -- get_admin 2>&1)" || true
  READ_HASH="$(echo "$OUT" | sed -n 's/.*Signing transaction: \([0-9a-f]*\).*/\1/p' | head -1)"
  echo "   get_admin tx: ${READ_HASH:-unknown}"
fi

echo ""
echo "3. Write-path fees (set env vars to tx hashes from e2e or deploy)"
FEE_JSON='[]'
FEE_LINES="$(mktemp)"
trap 'rm -f "$FEE_LINES"' EXIT
fetch_fee register_app "${TX_REGISTER_APP:-}" >>"$FEE_LINES" || true
fetch_fee verify_and_record_groth16 "${TX_VERIFY_GROTH16:-}" >>"$FEE_LINES" || true
fetch_fee record_attested_result "${TX_RECORD_ATTESTED:-}" >>"$FEE_LINES" || true
fetch_fee verify_and_record "${TX_VERIFY_AND_RECORD:-}" >>"$FEE_LINES" || true
fetch_fee get_admin_live "$READ_HASH" >>"$FEE_LINES" || true
if [[ -s "$FEE_LINES" ]]; then
  FEE_JSON="$(jq -s '.' "$FEE_LINES")"
fi

jq -n \
  --arg network "$STELLAR_NETWORK" \
  --arg contract "$CONTRACT_ID" \
  --arg at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --argjson fees "$FEE_JSON" \
  '{
    network: $network,
    contractId: $contract,
    benchmarkedAt: $at,
    notes: "fees from stellar tx fetch fee; simulate paths are not included",
    fees: $fees
  }' >"$OUT_FILE"

echo ""
echo "Wrote $OUT_FILE"
if [[ "$FEE_JSON" == "[]" ]]; then
  echo ""
  echo "Tip: run e2e, then re-run with tx hashes from 'Signing transaction:' lines, e.g.:"
  echo "  TX_VERIFY_GROTH16=<hash> ./scripts/benchmark-futurenet.sh"
fi
