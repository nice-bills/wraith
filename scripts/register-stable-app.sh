#!/usr/bin/env bash
# Register the stable app id from deployments/stable-app.json (idempotent).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPEC="$ROOT_DIR/deployments/stable-app.json"
: "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT}"
: "${STELLAR_NETWORK:=futurenet}"

CONTRACT_ID="${CONTRACT_ID:-$(jq -r .contractId "$ROOT_DIR/deployments/futurenet.json")}"
APP_ID="${STABLE_APP_ID:-$(jq -r .appId "$SPEC")}"
PROVER="$(stellar-cli keys address "$SOROBAN_SOURCE_ACCOUNT")"
ZERO32="0000000000000000000000000000000000000000000000000000000000000000"

MIN_AGE="$(jq -r .policy.min_age "$SPEC")"
REQ_HUMAN="$(jq -r .policy.require_humanity "$SPEC")"
CLAIM_LAYOUT="$(jq -r .policy.claim_layout "$SPEC")"
EXPIRY="$(jq -r .policy.expiration_window "$SPEC")"
SANCTIONS="$(jq -r .policy.sanctions_enabled "$SPEC")"
EXCLUDED="$(jq -c .policy.excluded_countries "$SPEC")"

POLICY=$(jq -nc \
  --arg owner "$PROVER" \
  --arg zero "$ZERO32" \
  --argjson min_age "$MIN_AGE" \
  --argjson req_human "$REQ_HUMAN" \
  --argjson excluded "$EXCLUDED" \
  --argjson expiry "$EXPIRY" \
  --argjson sanctions "$SANCTIONS" \
  --arg layout "$CLAIM_LAYOUT" \
  '{owner:$owner, min_age:$min_age, require_humanity:$req_human, sanctions_root:$zero,
    excluded_countries:$excluded, expiration_window:$expiry, sanctions_enabled:$sanctions,
    claim_layout:$layout}')

invoke() {
  stellar-cli contract invoke \
    --id "$CONTRACT_ID" \
    --source-account "$SOROBAN_SOURCE_ACCOUNT" \
    --network "$STELLAR_NETWORK" \
    "$@"
}

echo "=== Register stable app ==="
echo "Contract: $CONTRACT_ID"
echo "App:      $APP_ID"
echo "Owner:    $PROVER"

if invoke --send=no -- is_app_registered --app_id "$APP_ID" 2>/dev/null | grep -q true; then
  echo "Already registered."
  exit 0
fi

invoke --send=yes -- register_app --app_id "$APP_ID" --policy "$POLICY" --vk_hash null
echo "Registered app: $APP_ID"
