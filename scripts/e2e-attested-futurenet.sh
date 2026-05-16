#!/usr/bin/env bash
# Attested-path E2E on Futurenet (no passport / no ZK).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
: "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT (stellar-cli identity)}"
: "${STELLAR_NETWORK:=futurenet}"

CONTRACT_ID="${CONTRACT_ID:-$(jq -r .contractId "$ROOT_DIR/deployments/futurenet.json")}"
PROVER="$(stellar-cli keys address "$SOROBAN_SOURCE_ACCOUNT")"
SUBJECT="${SUBJECT:-$PROVER}"
APP_ID="e2eatt$(date +%s)"
NULLIFIER="$(openssl rand -hex 32)"
CLAIMS='{"age":30,"country_code":840,"is_human":true}'
ZERO32="0000000000000000000000000000000000000000000000000000000000000000"
POLICY=$(cat <<EOF
{"owner":"$PROVER","min_age":18,"require_humanity":false,"sanctions_root":"$ZERO32","excluded_countries":[],"expiration_window":0,"sanctions_enabled":false,"claim_layout":"Standard"}
EOF
)

invoke() {
  stellar-cli contract invoke \
    --id "$CONTRACT_ID" \
    --source-account "$SOROBAN_SOURCE_ACCOUNT" \
    --network "$STELLAR_NETWORK" \
    "$@"
}

echo "=== Attested E2E (Futurenet) ==="
echo "Contract: $CONTRACT_ID"
echo "App:      $APP_ID"
echo "Prover:   $PROVER"
echo "Subject:  $SUBJECT"
echo ""

echo "1. Register app (if needed)..."
if invoke --send=no -- is_app_registered --app_id "$APP_ID" 2>/dev/null | grep -q false; then
  invoke --send=yes -- register_app \
    --app_id "$APP_ID" \
    --policy "$POLICY" \
    --vk_hash null
  echo "   registered $APP_ID"
else
  echo "   already registered"
fi

echo "2. Compute hashes on-chain..."
PUB_HASH=$(invoke --send=no -- hash_attested_claims --claims "$CLAIMS" | tr -d '"')
ATT_HASH=$(invoke --send=no -- hash_attestation \
  --prover "$PROVER" \
  --app_id "$APP_ID" \
  --subject "$SUBJECT" \
  --nullifier "$NULLIFIER" \
  --claims "$CLAIMS" | tr -d '"')
echo "   public_inputs_hash: $PUB_HASH"
echo "   attestation_hash:   $ATT_HASH"

echo "3. record_attested_result..."
TX=$(invoke --send=yes -- record_attested_result \
  --prover "$PROVER" \
  --app_id "$APP_ID" \
  --subject "$SUBJECT" \
  --nullifier "$NULLIFIER" \
  --public_inputs_hash "$PUB_HASH" \
  --attestation_hash "$ATT_HASH" \
  --claims "$CLAIMS")
echo "$TX"

echo "4. is_verified..."
invoke --send=no -- is_verified --app_id "$APP_ID" --subject "$SUBJECT"
echo ""
echo "=== Attested E2E OK ==="
