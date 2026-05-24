#!/usr/bin/env bash
# Submit Groth16 RarimoQuery verification to Futurenet.
# Usage: stellar-submit-rarimo.sh <stellar-payload.json> [app_id_prefix]
set -euo pipefail

PAYLOAD="${1:?payload.json}"
APP_PREFIX="${2:-passport}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

: "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT}"
: "${STELLAR_NETWORK:=futurenet}"

CONTRACT_ID="${CONTRACT_ID:-$(jq -r .contractId "$ROOT_DIR/deployments/futurenet.json")}"
SUBJECT="$(stellar-cli keys address "$SOROBAN_SOURCE_ACCOUNT")"
APP_ID="${APP_PREFIX}$(date +%s)"
NULLIFIER="$(openssl rand -hex 32)"
ZERO32="0000000000000000000000000000000000000000000000000000000000000000"
CURRENT_DATE="${CURRENT_DATE_YMD:-$(date -u +%y%m%d)}"

invoke() {
  stellar-cli contract invoke \
    --id "$CONTRACT_ID" \
    --source-account "$SOROBAN_SOURCE_ACCOUNT" \
    --network "$STELLAR_NETWORK" \
    "$@"
}

VK_HASH=$(node "$ROOT_DIR/scripts/compute-vk-hash.mjs" "$PAYLOAD")
PUB_SIGNALS=$(jq -c '.public_signals_decimals' "$PAYLOAD")
CLAIMS=$(jq -c '.claims | {age, country_code, is_human}' "$PAYLOAD")

# shellcheck source=/dev/null
source "$ROOT_DIR/scripts/lib/write-stellar-proof-tmp.sh"
write_stellar_proof_tmp "$PAYLOAD" wraith-passport
trap 'rm -f "$PROOF_TMP" "$VK_TMP"' EXIT

PUBHASH=$(invoke --send=no -- compute_pub_signals_hash \
  --pub_signals "$PUB_SIGNALS" --current_date_ymd "$CURRENT_DATE" | tr -d '"')
POLICY="{\"owner\":\"$SUBJECT\",\"min_age\":18,\"require_humanity\":true,\"sanctions_root\":\"$ZERO32\",\"excluded_countries\":[],\"expiration_window\":0,\"sanctions_enabled\":false,\"claim_layout\":\"RarimoQuery\"}"

echo "Contract:  $CONTRACT_ID"
echo "App:       $APP_ID"
echo "Claims:    $CLAIMS"
echo "VK hash:   $VK_HASH"

invoke --send=yes -- register_app --app_id "$APP_ID" --policy "$POLICY" --vk_hash "\"$VK_HASH\""

invoke --send=yes -- verify_and_record \
  --app_id "$APP_ID" --subject "$SUBJECT" --nullifier "$NULLIFIER" \
  --public_inputs_hash "$PUBHASH" \
  --proof-file-path "$PROOF_TMP" \
  --vk-file-path "$VK_TMP" \
  --pub_signals "$PUB_SIGNALS" \
  --current_date_ymd "$CURRENT_DATE" \
  --claims "$CLAIMS"

echo "is_verified:"
invoke --send=no -- is_verified --app_id "$APP_ID" --subject "$SUBJECT"
echo "APP_ID=$APP_ID"
